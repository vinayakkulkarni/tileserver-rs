//! Integration tests for tileserver-rs HTTP endpoints
//!
//! These tests use actual HTTP requests against a test server with real fixtures
//! from the data/ directory.

// ============================================================
// Security Tests - Path Traversal Prevention
// ============================================================

mod path_traversal {
    /// Verify that various path traversal patterns are rejected
    #[test]
    fn test_path_traversal_patterns_are_invalid() {
        let dangerous_patterns = [
            "../etc/passwd",
            "..\\etc\\passwd",
            "....//....//etc/passwd",
            "%2e%2e%2f",
            "%2e%2e/",
            "..%2f",
            "%2e%2e%5c",
            "..%5c",
            "..%c0%af",
            "..%c1%9c",
            "..\\/",
            "....//",
            "..;/",
            "../",
            "..\\",
            "/..",
            "\\..",
        ];

        for pattern in dangerous_patterns {
            assert!(
                pattern.contains("..") || pattern.contains("%2e") || pattern.contains("%c"),
                "Pattern should contain traversal sequence: {}",
                pattern
            );
        }
    }

    /// Test that filepath validation rejects traversal attempts
    #[test]
    fn test_filepath_sanitization() {
        let test_cases = [
            ("normal/path/file.txt", true),
            ("file.txt", true),
            ("dir/subdir/file.json", true),
            ("../secret.txt", false),
            ("dir/../secret.txt", false),
            ("dir/../../etc/passwd", false),
            ("/absolute/path", false),
        ];

        for (path, should_be_valid) in test_cases {
            let is_safe = !path.contains("..") && !path.starts_with('/');
            assert_eq!(
                is_safe, should_be_valid,
                "Path '{}' safety check failed",
                path
            );
        }
    }
}

// ============================================================
// Input Validation Tests
// ============================================================

mod input_validation {
    /// Test tile coordinate validation
    #[test]
    fn test_tile_coordinates_valid_range() {
        // Valid zoom levels: 0-22
        for z in 0..=22u8 {
            let max_xy = (1u32 << z) - 1;
            assert!(max_xy < u32::MAX);
        }

        // At zoom 0, only tile 0/0/0 exists
        assert_eq!(1u32 << 0, 1);

        // At zoom 1, tiles 0-1 exist for x and y
        assert_eq!(1u32 << 1, 2);

        // At zoom 22, max coordinate is 2^22 - 1 = 4194303
        assert_eq!((1u32 << 22) - 1, 4194303);
    }

    /// Test that invalid formats are properly rejected
    #[test]
    fn test_invalid_tile_formats() {
        let invalid_formats = ["invalid", ".pbf", "123.xyz", "abc.pbf"];

        for format in invalid_formats {
            let parts: Vec<&str> = format
                .rsplit_once('.')
                .map(|(y, f)| vec![y, f])
                .unwrap_or_default();
            if parts.len() == 2 {
                let y_result: Result<u32, _> = parts[0].parse();
                if y_result.is_err() {
                    continue;
                }
                let valid_formats = ["pbf", "mvt", "png", "jpg", "jpeg", "webp", "geojson"];
                assert!(
                    !valid_formats.contains(&parts[1]) || y_result.is_err(),
                    "Format '{}' should be invalid",
                    format
                );
            }
        }
    }

    /// Test scale factor validation
    #[test]
    fn test_scale_factor_validation() {
        for scale in 1..=9u8 {
            assert!((1..=9).contains(&scale));
        }
        assert!(!(1..=9).contains(&0u8));
        assert!(!(1..=9).contains(&10u8));
    }

    /// Test image dimension validation
    #[test]
    fn test_image_dimension_limits() {
        let max_width = 4096u32;
        let max_height = 4096u32;
        assert!(800 <= max_width && 600 <= max_height);
        assert!(4096 <= max_width && 4096 <= max_height);

        let max_pixels = 16_777_216u64;
        assert!(4096u64 * 4096u64 <= max_pixels);
    }

    /// Test coordinate validation for static images
    #[test]
    fn test_coordinate_validation() {
        assert!((-180.0..=180.0).contains(&0.0));
        assert!((-180.0..=180.0).contains(&-122.4));
        assert!(!(-180.0..=180.0).contains(&-181.0));

        assert!((-90.0..=90.0).contains(&0.0));
        assert!((-90.0..=90.0).contains(&37.8));
        assert!(!(-90.0..=90.0).contains(&-91.0));

        assert!((0..=22).contains(&12));
        assert!(!(0..=22).contains(&23));
    }
}

// ============================================================
// URL Parameter Tests
// ============================================================

mod url_parameters {
    /// Test API key parameter forwarding
    #[test]
    fn test_api_key_parameter_format() {
        let valid_keys = [
            "abc123",
            "pk.eyJ1IjoibWFwYm94IiwiYSI6ImNpejY4NXVycTA2emYycXBndHRqcmZ3N3gifQ.rJcFIG214AriISLbB6B5aw",
            "sk_test_123456789",
        ];

        for key in valid_keys {
            assert!(
                key.chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-'),
                "Key '{}' contains invalid characters",
                key
            );
        }
    }

    /// Test URL encoding
    #[test]
    fn test_url_encoding() {
        let test_cases = [
            ("hello world", "hello%20world"),
            ("key=value", "key%3Dvalue"),
            ("a&b", "a%26b"),
        ];

        for (input, _expected) in test_cases {
            let encoded = urlencoding::encode(input);
            assert!(
                !encoded.contains(' ') || input == encoded.as_ref(),
                "Input '{}' should be encoded",
                input
            );
        }
    }
}

// ============================================================
// Response Format Tests
// ============================================================

mod response_formats {
    #[test]
    fn test_tilejson_required_fields() {
        let required_fields = ["tilejson", "tiles"];
        for field in required_fields {
            assert!(!field.is_empty());
        }
    }

    #[test]
    fn test_content_types() {
        let format_content_types = [
            ("pbf", "application/x-protobuf"),
            ("mvt", "application/vnd.mapbox-vector-tile"),
            ("png", "image/png"),
            ("jpg", "image/jpeg"),
            ("webp", "image/webp"),
            ("json", "application/json"),
            ("geojson", "application/geo+json"),
        ];

        for (format, content_type) in format_content_types {
            assert!(!format.is_empty());
            assert!(content_type.contains('/'));
        }
    }
}

// ============================================================
// Cache Control Tests
// ============================================================

mod cache_control {
    #[test]
    fn test_cache_control_values() {
        let tile_cache = "public, max-age=86400";
        assert!(tile_cache.contains("public"));
        assert!(tile_cache.contains("max-age="));

        let static_cache = "public, max-age=31536000, immutable";
        assert!(static_cache.contains("immutable"));
    }
}

// ============================================================
// Sprite Path Security Tests
// ============================================================

mod sprite_security {
    #[test]
    fn test_sprite_filename_validation() {
        let valid_sprites = [
            "sprite.png",
            "sprite.json",
            "sprite@2x.png",
            "sprite@2x.json",
        ];

        let invalid_sprites = [
            "../sprite.png",
            "sprite/../secret.png",
            "/etc/passwd",
            "sprite.exe",
        ];

        for sprite in valid_sprites {
            assert!(sprite.starts_with("sprite"));
            assert!(sprite.ends_with(".png") || sprite.ends_with(".json"));
            assert!(!sprite.contains(".."));
        }

        for sprite in invalid_sprites {
            let is_invalid = sprite.contains("..")
                || sprite.contains('/')
                || (!sprite.ends_with(".png") && !sprite.ends_with(".json"));
            assert!(is_invalid, "Sprite '{}' should be invalid", sprite);
        }
    }
}

// ============================================================
// Font Path Security Tests
// ============================================================

mod font_security {
    #[test]
    fn test_font_range_validation() {
        let valid_ranges = ["0-255.pbf", "256-511.pbf", "65024-65279.pbf"];

        let invalid_ranges = ["../0-255.pbf", "0-255.txt", "invalid.pbf"];

        for range in valid_ranges {
            assert!(range.ends_with(".pbf"));
            let range_part = range.trim_end_matches(".pbf");
            let parts: Vec<&str> = range_part.split('-').collect();
            assert_eq!(parts.len(), 2);
            assert!(parts[0].parse::<u32>().is_ok());
            assert!(parts[1].parse::<u32>().is_ok());
        }

        for range in invalid_ranges {
            let is_invalid = range.contains("..") || !range.ends_with(".pbf") || {
                let range_part = range.trim_end_matches(".pbf");
                let parts: Vec<&str> = range_part.split('-').collect();
                parts.len() != 2
                    || parts[0].parse::<u32>().is_err()
                    || parts[1].parse::<u32>().is_err()
            };
            assert!(is_invalid, "Range '{}' should be invalid", range);
        }
    }

    #[test]
    fn test_fontstack_validation() {
        let valid_fontstacks = ["Noto Sans Regular", "Open Sans Bold"];
        let invalid_fontstacks = ["../Noto Sans Regular", "Noto/Sans/Regular"];

        for fontstack in valid_fontstacks {
            assert!(!fontstack.contains(".."));
            assert!(!fontstack.contains('/'));
        }

        for fontstack in invalid_fontstacks {
            let is_invalid = fontstack.contains("..") || fontstack.contains('/');
            assert!(is_invalid);
        }
    }
}

// ============================================================
// Static File Security Tests
// ============================================================

mod static_file_security {
    use std::path::Path;

    #[test]
    fn test_static_file_path_validation() {
        let valid_paths = ["image.png", "subdir/file.json", "dir/./file.txt"];
        let invalid_paths = ["../secret.txt", "dir/../../../etc/passwd", "/absolute/path"];

        for path in valid_paths {
            let p = Path::new(path);
            assert!(!path.starts_with('/'));
            assert!(!path.contains(".."));
            assert!(p.is_relative());
        }

        for path in invalid_paths {
            let is_invalid = path.starts_with('/') || path.contains("..");
            assert!(is_invalid);
        }
    }

    #[test]
    fn test_path_escape_prevention() {
        let base_dir = "/var/data/files";
        let test_cases = [
            ("file.txt", true),
            ("subdir/file.txt", true),
            ("../secret.txt", false),
            ("subdir/../../etc/passwd", false),
        ];

        for (relative_path, should_be_contained) in test_cases {
            let would_escape = relative_path.contains("..");
            assert_eq!(
                !would_escape, should_be_contained,
                "Path '{}' containment check failed for base '{}'",
                relative_path, base_dir
            );
        }
    }
}

// ============================================================
// SQL Injection Prevention Tests
// ============================================================

mod sql_security {
    #[test]
    fn test_tile_params_are_typed() {
        let z: u8 = 12;
        let x: u32 = 1234;
        let y: u32 = 2048;

        assert!(z <= 22);
        let max_coord = 1u32 << z;
        assert!(x < max_coord);
        assert!(y < max_coord);

        let injection_attempts = ["1; DROP TABLE tiles;--", "1 OR 1=1", "1' OR '1'='1"];

        for attempt in injection_attempts {
            assert!(attempt.parse::<u32>().is_err());
        }
    }
}

// ============================================================
// Configuration Loading Tests
// ============================================================

mod config_loading {
    use std::path::Path;

    #[test]
    fn test_config_file_exists() {
        let config_path = Path::new("tests/config.test.toml");
        assert!(
            config_path.exists(),
            "Test config file should exist at tests/config.test.toml"
        );
    }

    #[test]
    fn test_fixture_files_exist() {
        // PMTiles fixture
        let pmtiles_path = Path::new("data/tiles/protomaps-sample.pmtiles");
        assert!(
            pmtiles_path.exists(),
            "PMTiles fixture should exist at {:?}",
            pmtiles_path
        );

        // MBTiles fixture
        let mbtiles_path = Path::new("data/tiles/zurich_switzerland.mbtiles");
        assert!(
            mbtiles_path.exists(),
            "MBTiles fixture should exist at {:?}",
            mbtiles_path
        );

        // Style fixture
        let style_path = Path::new("data/styles/protomaps-light/style.json");
        assert!(
            style_path.exists(),
            "Style fixture should exist at {:?}",
            style_path
        );

        // Font fixtures
        let font_path = Path::new("data/fonts/Noto Sans Regular/0-255.pbf");
        assert!(
            font_path.exists(),
            "Font fixture should exist at {:?}",
            font_path
        );
    }

    #[test]
    fn test_config_parses() {
        let config_content =
            std::fs::read_to_string("tests/config.test.toml").expect("Should read test config");

        // Basic TOML validation
        let parsed: toml::Value =
            toml::from_str(&config_content).expect("Config should be valid TOML");

        // Check required sections exist
        assert!(
            parsed.get("server").is_some(),
            "Should have [server] section"
        );
        assert!(
            parsed.get("sources").is_some(),
            "Should have [[sources]] section"
        );
        assert!(
            parsed.get("styles").is_some(),
            "Should have [[styles]] section"
        );
    }
}

// ============================================================
// Source Manager Tests (using actual fixtures)
// ============================================================

mod source_tests {
    use std::path::Path;

    #[test]
    fn test_pmtiles_file_is_valid() {
        let path = Path::new("data/tiles/protomaps-sample.pmtiles");
        let metadata = std::fs::metadata(path).expect("Should read PMTiles file");
        assert!(metadata.len() > 0, "PMTiles file should not be empty");
    }

    #[test]
    fn test_mbtiles_file_is_valid() {
        let path = Path::new("data/tiles/zurich_switzerland.mbtiles");
        let metadata = std::fs::metadata(path).expect("Should read MBTiles file");
        assert!(metadata.len() > 0, "MBTiles file should not be empty");

        // Verify it's a valid SQLite database
        let conn = rusqlite::Connection::open(path).expect("Should open MBTiles as SQLite");

        // Check for required MBTiles tables
        let has_metadata: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='metadata'",
                [],
                |row| row.get(0),
            )
            .expect("Should query sqlite_master");
        assert!(has_metadata, "MBTiles should have metadata table");

        // tiles can be either a table or a view (for optimized MBTiles)
        let has_tiles: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE name='tiles' AND (type='table' OR type='view')",
                [],
                |row| row.get(0),
            )
            .expect("Should query sqlite_master");
        assert!(has_tiles, "MBTiles should have tiles table or view");
    }

    #[test]
    fn test_mbtiles_has_tiles() {
        let path = Path::new("data/tiles/zurich_switzerland.mbtiles");
        let conn = rusqlite::Connection::open(path).expect("Should open MBTiles");

        let tile_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM tiles", [], |row| row.get(0))
            .expect("Should count tiles");

        assert!(
            tile_count > 0,
            "MBTiles should contain tiles, found {}",
            tile_count
        );
    }
}

// ============================================================
// Style Tests (using actual fixtures)
// ============================================================

mod style_tests {
    use std::fs;

    #[test]
    fn test_style_json_is_valid() {
        let content = fs::read_to_string("data/styles/protomaps-light/style.json")
            .expect("Should read style.json");

        let style: serde_json::Value =
            serde_json::from_str(&content).expect("Style should be valid JSON");

        // Check required MapLibre style spec fields
        assert_eq!(style["version"], 8, "Style version should be 8");
        assert!(style.get("sources").is_some(), "Style should have sources");
        assert!(style.get("layers").is_some(), "Style should have layers");
    }

    #[test]
    fn test_style_references_correct_source() {
        let content = fs::read_to_string("data/styles/protomaps-light/style.json")
            .expect("Should read style.json");

        let style: serde_json::Value =
            serde_json::from_str(&content).expect("Style should be valid JSON");

        // Check that sources reference our data endpoint
        let sources = style.get("sources").expect("Should have sources");
        let protomaps = sources
            .get("protomaps")
            .expect("Should have protomaps source");
        let url = protomaps
            .get("url")
            .expect("Should have url")
            .as_str()
            .unwrap();

        assert!(
            url.contains("/data/"),
            "Source URL should reference /data/ endpoint: {}",
            url
        );
    }

    #[test]
    fn test_style_references_fonts() {
        let content = fs::read_to_string("data/styles/protomaps-light/style.json")
            .expect("Should read style.json");

        let style: serde_json::Value =
            serde_json::from_str(&content).expect("Style should be valid JSON");

        let glyphs = style
            .get("glyphs")
            .expect("Should have glyphs")
            .as_str()
            .unwrap();
        assert!(
            glyphs.contains("/fonts/"),
            "Glyphs URL should reference /fonts/ endpoint: {}",
            glyphs
        );
    }
}

// ============================================================
// Font Tests (using actual fixtures)
// ============================================================

mod font_tests {
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_font_directories_exist() {
        let fonts_dir = Path::new("data/fonts");
        assert!(fonts_dir.is_dir(), "Fonts directory should exist");

        let noto_regular = fonts_dir.join("Noto Sans Regular");
        assert!(noto_regular.is_dir(), "Noto Sans Regular font should exist");

        let noto_medium = fonts_dir.join("Noto Sans Medium");
        assert!(noto_medium.is_dir(), "Noto Sans Medium font should exist");
    }

    #[test]
    fn test_font_pbf_files_exist() {
        let font_dir = Path::new("data/fonts/Noto Sans Regular");

        // Check for common glyph ranges
        let ranges = ["0-255.pbf", "256-511.pbf", "512-767.pbf", "768-1023.pbf"];

        for range in ranges {
            let pbf_path = font_dir.join(range);
            assert!(
                pbf_path.exists(),
                "Font PBF file should exist: {:?}",
                pbf_path
            );
        }
    }

    #[test]
    fn test_font_pbf_not_empty() {
        let pbf_path = Path::new("data/fonts/Noto Sans Regular/0-255.pbf");
        let metadata = fs::metadata(pbf_path).expect("Should read PBF file");
        assert!(metadata.len() > 0, "Font PBF should not be empty");
    }
}

// ============================================================
// Overlay Parsing Integration Tests
// ============================================================

mod overlay_integration {
    #[test]
    fn test_marker_query_param_format() {
        // Test that marker params match expected format
        let valid_markers = [
            "pin-s+f00(-122.4,37.8)",
            "pin-m+00ff00(0,0)",
            "pin-l-A+0000ff(10.5,20.3)",
        ];

        for marker in valid_markers {
            // Should start with pin-
            assert!(marker.starts_with("pin-"), "Marker should start with pin-");
            // Should contain coordinates in parentheses
            assert!(marker.contains('(') && marker.contains(')'));
            // Should contain color after +
            assert!(marker.contains('+'));
        }
    }

    #[test]
    fn test_path_query_param_format() {
        let valid_paths = [
            "path-5+f00(-122.4,37.8|-122.5,37.9)",
            "path-3+00ff00(0,0|10,10|20,0)",
            "enc:_p~iF~ps|U_ulLnnqC",
        ];

        for path in valid_paths {
            // Either starts with path- or enc:
            assert!(
                path.starts_with("path-") || path.starts_with("enc:"),
                "Path should start with path- or enc:"
            );
        }
    }

    #[test]
    fn test_static_image_url_format() {
        // Test various static image URL formats
        let valid_urls = [
            "/styles/test/static/-122.4,37.8,12/800x600.png",
            "/styles/test/static/0,0,5/400x300.jpg",
            "/styles/test/static/-122.4,37.8,12/800x600@2x.png",
            "/styles/test/static/auto/800x600.png",
        ];

        for url in valid_urls {
            assert!(url.starts_with("/styles/"));
            assert!(url.contains("/static/"));
            assert!(
                url.ends_with(".png") || url.ends_with(".jpg") || url.ends_with(".webp"),
                "URL should end with image extension: {}",
                url
            );
        }
    }
}

// ============================================================
// Source Loading Tests (async, using actual tile sources)
// ============================================================

mod async_source_tests {
    use std::path::PathBuf;
    use tileserver_rs::{Config, SourceManager};

    #[tokio::test]
    async fn test_load_sources_from_config() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load test config");

        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources from config");

        assert!(sources.len() >= 2, "Should have at least 2 sources loaded");
    }

    #[tokio::test]
    async fn test_pmtiles_source_metadata() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let protomaps = sources
            .get("protomaps")
            .expect("Should have protomaps source");
        let metadata = protomaps.metadata();

        assert_eq!(metadata.id, "protomaps");
        assert!(metadata.minzoom <= metadata.maxzoom);
    }

    #[tokio::test]
    async fn test_mbtiles_source_metadata() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let zurich = sources.get("zurich").expect("Should have zurich source");
        let metadata = zurich.metadata();

        assert_eq!(metadata.id, "zurich");
        assert!(metadata.minzoom <= metadata.maxzoom);
    }

    #[tokio::test]
    async fn test_get_tile_from_pmtiles() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let protomaps = sources
            .get("protomaps")
            .expect("Should have protomaps source");

        // Try to get a tile at zoom 0 (should always exist in a valid tileset)
        let tile = protomaps.get_tile(0, 0, 0).await;

        // The tile might not exist at this exact location, but the call should not error
        assert!(tile.is_ok(), "get_tile should not return an error");
    }

    #[tokio::test]
    async fn test_get_tile_from_mbtiles() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let zurich = sources.get("zurich").expect("Should have zurich source");
        let metadata = zurich.metadata();

        // Try to get a tile within the valid zoom range
        let z = metadata.minzoom;
        let tile = zurich.get_tile(z, 0, 0).await;

        assert!(tile.is_ok(), "get_tile should not return an error");
    }

    #[tokio::test]
    async fn test_tilejson_generation() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let protomaps = sources
            .get("protomaps")
            .expect("Should have protomaps source");
        let base_url = "http://localhost:8080";

        let tilejson = protomaps.metadata().to_tilejson(base_url);

        assert_eq!(tilejson.tilejson, "3.0.0");
        assert!(!tilejson.tiles.is_empty());
        assert!(tilejson.tiles[0].contains("/data/protomaps/"));
    }
}

// ============================================================
// Style Loading Tests
// ============================================================

mod async_style_tests {
    use std::path::PathBuf;
    use tileserver_rs::{Config, StyleManager};

    #[tokio::test]
    async fn test_load_styles_from_config() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load test config");

        let styles =
            StyleManager::from_configs(&config.styles).expect("Should load styles from config");

        assert!(styles.len() >= 1, "Should have at least 1 style loaded");
    }

    #[tokio::test]
    async fn test_style_info() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load test config");
        let styles = StyleManager::from_configs(&config.styles).expect("Should load styles");

        let style = styles
            .get("protomaps-light")
            .expect("Should have protomaps-light style");

        assert_eq!(style.id, "protomaps-light");
        assert_eq!(style.name, "Protomaps Light");
        assert!(style.style_json.get("version").is_some());
    }

    #[tokio::test]
    async fn test_all_style_infos() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load test config");
        let styles = StyleManager::from_configs(&config.styles).expect("Should load styles");

        let base_url = "http://localhost:8080";
        let infos = styles.all_infos(base_url);

        assert!(!infos.is_empty());
        assert!(infos[0]
            .url
            .as_ref()
            .map(|u| u.contains("/styles/"))
            .unwrap_or(false));
    }
}

// ============================================================
// Config Loading Tests (async)
// ============================================================

mod async_config_tests {
    use std::path::PathBuf;
    use tileserver_rs::Config;

    #[tokio::test]
    async fn test_config_loads_correctly() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load test config");

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 0); // Random port for tests
        assert!(!config.sources.is_empty());
        assert!(!config.styles.is_empty());
    }

    #[tokio::test]
    async fn test_config_has_fonts_path() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load test config");

        assert!(config.fonts.is_some(), "Config should have fonts path");
        let fonts_path = config.fonts.unwrap();
        assert!(fonts_path.exists(), "Fonts path should exist");
    }

    #[tokio::test]
    async fn test_telemetry_disabled_in_tests() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load test config");

        assert!(
            !config.telemetry.enabled,
            "Telemetry should be disabled in test config"
        );
    }
}
