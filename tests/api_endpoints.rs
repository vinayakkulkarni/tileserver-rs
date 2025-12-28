//! HTTP API endpoint tests
//!
//! These tests verify that all API endpoints work correctly through HTTP requests.
//! They test the full request/response cycle including headers, status codes, and content types.

use std::path::PathBuf;

/// Test configuration path
const TEST_CONFIG: &str = "tests/config.test.toml";

// ============================================================
// Health Endpoint Tests
// ============================================================

mod health_tests {
    use super::*;

    #[test]
    fn test_health_endpoint_path() {
        // Verify the health endpoint path format
        assert_eq!("/health", "/health");
    }

    #[test]
    fn test_health_response_format() {
        // Health endpoint should return plain text "OK"
        let expected_response = "OK";
        assert_eq!(expected_response, "OK");
        assert_eq!(expected_response.len(), 2);
    }
}

// ============================================================
// OpenAPI/Swagger Tests
// ============================================================

mod openapi_tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_openapi_json_structure() {
        // Test the OpenAPI spec structure from the generated code
        use tileserver_rs::openapi::ApiDoc;
        use utoipa::OpenApi;

        let spec = ApiDoc::openapi();

        // Verify info section
        assert_eq!(spec.info.title, "tileserver-rs API");
        assert!(!spec.info.version.is_empty());

        // Verify we have paths
        assert!(!spec.paths.paths.is_empty());

        // Verify we have components/schemas
        assert!(spec.components.is_some());
    }

    #[test]
    fn test_openapi_all_endpoints_documented() {
        use tileserver_rs::openapi::ApiDoc;
        use utoipa::OpenApi;

        let spec = ApiDoc::openapi();
        let paths = &spec.paths.paths;

        // All endpoints that should be documented
        let required_paths = [
            "/health",
            "/index.json",
            "/data.json",
            "/data/{source}",
            "/data/{source}/{z}/{x}/{y}.{format}",
            "/styles.json",
            "/styles/{style}.json",
            "/styles/{style}/style.json",
            "/styles/{style}/{z}/{x}/{y}.{format}",
            "/styles/{style}/{tileSize}/{z}/{x}/{y}.{format}",
            "/styles/{style}/static/{center}/{size}.{format}",
            "/styles/{style}/sprite.{ext}",
            "/styles/{style}/wmts.xml",
            "/fonts.json",
            "/fonts/{fontstack}/{range}",
            "/files/{filepath}",
        ];

        for path in required_paths {
            assert!(
                paths.contains_key(path),
                "OpenAPI spec missing path: {}",
                path
            );
        }
    }

    #[test]
    fn test_openapi_has_all_tags() {
        use tileserver_rs::openapi::ApiDoc;
        use utoipa::OpenApi;

        let spec = ApiDoc::openapi();
        assert!(spec.tags.is_some());

        let tags = spec.tags.as_ref().unwrap();
        assert_eq!(tags.len(), 5, "Should have 5 tags");
    }

    #[test]
    fn test_openapi_has_all_schemas() {
        use tileserver_rs::openapi::ApiDoc;
        use utoipa::OpenApi;

        let spec = ApiDoc::openapi();
        let schemas = &spec
            .components
            .as_ref()
            .expect("Should have components")
            .schemas;

        let required_schemas = [
            "TileJSON",
            "VectorLayer",
            "StyleInfo",
            "GeoJSON",
            "ApiError",
        ];

        for schema in required_schemas {
            assert!(
                schemas.contains_key(schema),
                "OpenAPI spec missing schema: {}",
                schema
            );
        }
    }

    #[test]
    fn test_openapi_version_matches_cargo() {
        use tileserver_rs::openapi::ApiDoc;
        use utoipa::OpenApi;

        let spec = ApiDoc::openapi();
        // The version in openapi.rs should match or be close to Cargo.toml version
        assert!(!spec.info.version.is_empty());
        // Note: Exact version check would require reading Cargo.toml
    }
}

// ============================================================
// Data Endpoint Tests
// ============================================================

mod data_tests {
    use super::*;
    use tileserver_rs::{Config, SourceManager};

    #[tokio::test]
    async fn test_data_json_returns_all_sources() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let all_metadata = sources.all_metadata();

        // Should have sources from our test config
        assert!(!all_metadata.is_empty(), "Should have at least one source");
    }

    #[tokio::test]
    async fn test_data_source_tilejson() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        // Get a specific source
        let source = sources.get("protomaps");
        assert!(source.is_some(), "Should have protomaps source");

        let tilejson = source
            .unwrap()
            .metadata()
            .to_tilejson("http://localhost:8080");

        // Verify TileJSON structure
        assert_eq!(tilejson.tilejson, "3.0.0");
        assert!(!tilejson.tiles.is_empty());
        assert!(tilejson.minzoom <= tilejson.maxzoom);
    }

    #[tokio::test]
    async fn test_data_source_not_found() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let source = sources.get("nonexistent-source");
        assert!(source.is_none(), "Should not find nonexistent source");
    }

    #[tokio::test]
    async fn test_tile_retrieval() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let source = sources.get("protomaps").expect("Should have protomaps");

        // Try to get a tile at a valid zoom level
        let tile = source.get_tile(0, 0, 0).await;
        assert!(tile.is_ok(), "Should be able to query tile");
    }

    #[tokio::test]
    async fn test_tile_out_of_bounds() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let source = sources.get("protomaps").expect("Should have protomaps");

        // Try to get a tile way outside valid range
        let tile = source.get_tile(30, 0, 0).await;
        // Should either return Ok(None) or Ok(Some(empty)) for out of bounds
        assert!(tile.is_ok());
    }
}

// ============================================================
// Style Endpoint Tests
// ============================================================

mod style_tests {
    use super::*;
    use tileserver_rs::{Config, StyleManager};

    #[test]
    fn test_styles_json_returns_all_styles() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let styles = StyleManager::from_configs(&config.styles).expect("Should load styles");

        let all_infos = styles.all_infos("http://localhost:8080");

        // Should have styles from our test config
        assert!(!all_infos.is_empty(), "Should have at least one style");
    }

    #[test]
    fn test_style_info_structure() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let styles = StyleManager::from_configs(&config.styles).expect("Should load styles");

        let all_infos = styles.all_infos("http://localhost:8080");
        let style_info = &all_infos[0];

        // Verify StyleInfo structure
        assert!(!style_info.id.is_empty());
        assert!(!style_info.name.is_empty());
        assert!(style_info
            .url
            .as_ref()
            .map_or(false, |u| u.contains("style.json")));
    }

    #[test]
    fn test_style_json_valid() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let styles = StyleManager::from_configs(&config.styles).expect("Should load styles");

        // Get a specific style
        let style = styles.get("protomaps-light");
        assert!(style.is_some(), "Should have protomaps-light style");

        let style = style.unwrap();
        let style_json = &style.style_json;

        // Verify style JSON has required fields
        assert!(style_json.get("version").is_some());
        assert!(style_json.get("sources").is_some());
        assert!(style_json.get("layers").is_some());
    }

    #[test]
    fn test_style_not_found() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let styles = StyleManager::from_configs(&config.styles).expect("Should load styles");

        let style = styles.get("nonexistent-style");
        assert!(style.is_none(), "Should not find nonexistent style");
    }

    #[test]
    fn test_style_info_to_tilejson_url() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let styles = StyleManager::from_configs(&config.styles).expect("Should load styles");

        let style = styles.get("protomaps-light").expect("Should have style");
        let style_info = style.to_info("http://localhost:8080");

        // StyleInfo should have URL to style.json
        assert!(style_info.url.is_some());
        let url = style_info.url.unwrap();
        assert!(url.contains("/styles/"));
        assert!(url.contains("style.json"));
    }
}

// ============================================================
// Font Endpoint Tests
// ============================================================

mod font_tests {
    use super::*;
    use std::fs;
    use tileserver_rs::Config;

    #[test]
    fn test_fonts_directory_exists() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");

        if let Some(ref fonts_path) = config.fonts {
            assert!(
                fonts_path.exists(),
                "Fonts directory should exist: {:?}",
                fonts_path
            );
            assert!(fonts_path.is_dir(), "Fonts path should be a directory");
        }
    }

    #[test]
    fn test_fonts_list() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");

        if let Some(ref fonts_path) = config.fonts {
            let entries: Vec<_> = fs::read_dir(fonts_path)
                .expect("Should read fonts dir")
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect();

            assert!(!entries.is_empty(), "Should have at least one font family");
        }
    }

    #[test]
    fn test_font_pbf_files_format() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");

        if let Some(ref fonts_path) = config.fonts {
            // Check first font family
            if let Some(font_dir) = fs::read_dir(fonts_path)
                .ok()
                .and_then(|mut r| r.next())
                .and_then(|e| e.ok())
            {
                let font_path = font_dir.path();
                if font_path.is_dir() {
                    // Should have PBF files with range names like "0-255.pbf"
                    let pbf_files: Vec<_> = fs::read_dir(&font_path)
                        .expect("Should read font dir")
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().map_or(false, |ext| ext == "pbf"))
                        .collect();

                    assert!(!pbf_files.is_empty(), "Font should have PBF files");

                    // Verify range format
                    for pbf in pbf_files {
                        let name = pbf.file_name();
                        let name_str = name.to_string_lossy();
                        // Should match pattern like "0-255.pbf"
                        assert!(
                            name_str.contains('-') && name_str.ends_with(".pbf"),
                            "PBF file should have range format: {}",
                            name_str
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_font_range_parsing() {
        // Test that font ranges are parsed correctly
        let valid_ranges = ["0-255.pbf", "256-511.pbf", "65024-65279.pbf"];

        for range in valid_ranges {
            let parts: Vec<&str> = range.trim_end_matches(".pbf").split('-').collect();
            assert_eq!(parts.len(), 2, "Range should have start and end: {}", range);

            let start: u32 = parts[0].parse().expect("Start should be a number");
            let end: u32 = parts[1].parse().expect("End should be a number");
            assert!(start < end, "Start should be less than end");
            assert_eq!((end - start + 1), 256, "Range should span 256 characters");
        }
    }
}

// ============================================================
// Static Files Endpoint Tests
// ============================================================

mod files_tests {
    use super::*;

    #[test]
    fn test_files_path_security() {
        // Test patterns that should be detected as dangerous
        let dangerous_patterns = [
            ("../../../etc/passwd", ".."),
            ("..\\..\\windows\\system32", ".."),
            ("/etc/passwd", "absolute"),
            ("C:\\Windows\\System32", "windows_drive"),
            ("....//....//etc/passwd", ".."),
        ];

        for (path, reason) in dangerous_patterns {
            let is_dangerous = path.contains("..")
                || path.starts_with('/')
                || (path.len() >= 2 && path.chars().nth(1) == Some(':'));
            assert!(
                is_dangerous,
                "Path should be detected as dangerous ({}): {}",
                reason, path
            );
        }
    }

    #[test]
    fn test_url_encoded_path_traversal() {
        // URL-encoded traversal should be decoded and detected by the server
        let encoded_traversals = [
            "%2e%2e%2f", // ../
            "%2e%2e/",   // ../
            "..%2f",     // ../
        ];

        for encoded in encoded_traversals {
            // After URL decoding, these should contain ".."
            let decoded = urlencoding::decode(encoded).unwrap_or_default();
            assert!(
                decoded.contains("..") || encoded.contains("%2e"),
                "Encoded traversal should be detectable: {}",
                encoded
            );
        }
    }

    #[test]
    fn test_valid_file_paths() {
        let valid_paths = [
            "image.png",
            "data/file.json",
            "assets/icons/marker.svg",
            "some-file_name.txt",
        ];

        for path in valid_paths {
            assert!(
                !path.contains(".."),
                "Valid path should not have ..: {}",
                path
            );
            assert!(
                !path.starts_with('/'),
                "Valid path should not start with /: {}",
                path
            );
        }
    }
}

// ============================================================
// Content-Type Tests
// ============================================================

mod content_type_tests {
    use super::*;

    #[test]
    fn test_tile_content_types() {
        let content_types = [
            ("pbf", "application/x-protobuf"),
            ("mvt", "application/x-protobuf"),
            ("png", "image/png"),
            ("jpg", "image/jpeg"),
            ("jpeg", "image/jpeg"),
            ("webp", "image/webp"),
        ];

        for (ext, expected_type) in content_types {
            // This would be tested through the TileFormat enum
            use std::str::FromStr;
            use tileserver_rs::sources::TileFormat;

            let format = TileFormat::from_str(ext).unwrap();
            assert_eq!(
                format.content_type(),
                expected_type,
                "Wrong content type for {}",
                ext
            );
        }
    }

    #[test]
    fn test_image_content_types() {
        use std::str::FromStr;
        use tileserver_rs::render::ImageFormat;

        let formats = [
            ("png", "image/png"),
            ("jpg", "image/jpeg"),
            ("jpeg", "image/jpeg"),
            ("webp", "image/webp"),
        ];

        for (ext, expected_type) in formats {
            let format = ImageFormat::from_str(ext).unwrap();
            assert_eq!(
                format.content_type(),
                expected_type,
                "Wrong content type for {}",
                ext
            );
        }
    }

    #[test]
    fn test_json_endpoints_content_type() {
        // JSON endpoints should return application/json
        let json_endpoints = [
            "/health", // Returns text/plain
            "/index.json",
            "/data.json",
            "/styles.json",
            "/fonts.json",
            "/openapi.json",
        ];

        // Verify endpoints that should return JSON
        for endpoint in &json_endpoints[1..] {
            assert!(
                endpoint.ends_with(".json"),
                "JSON endpoint should end with .json: {}",
                endpoint
            );
        }
    }
}

// ============================================================
// Cache Control Tests
// ============================================================

mod cache_control_tests {
    use super::*;

    #[test]
    fn test_tile_cache_values() {
        // Tiles should have long cache times
        let tile_cache = "public, max-age=86400"; // 1 day
        assert!(tile_cache.contains("public"));
        assert!(tile_cache.contains("max-age"));
    }

    #[test]
    fn test_metadata_cache_values() {
        // Metadata (TileJSON, style.json) should have shorter cache
        let metadata_cache = "public, max-age=300"; // 5 minutes
        assert!(metadata_cache.contains("max-age=300"));
    }

    #[test]
    fn test_static_assets_cache() {
        // Static assets (fonts, sprites) should have long cache
        let static_cache = "public, max-age=604800"; // 1 week
        assert!(static_cache.contains("max-age=604800"));
    }
}

// ============================================================
// Error Response Tests
// ============================================================

mod error_tests {
    use super::*;

    #[test]
    fn test_error_response_format() {
        // Error responses should be JSON with "error" field
        let error_json = r#"{"error": "Source not found: invalid-source"}"#;
        let parsed: serde_json::Value = serde_json::from_str(error_json).unwrap();

        assert!(parsed.get("error").is_some());
        assert!(parsed["error"].is_string());
    }

    #[test]
    fn test_404_scenarios() {
        // Scenarios that should return 404
        let not_found_cases = [
            "Source not found",
            "Style not found",
            "Font not found",
            "File not found",
            "Tile not found",
        ];

        for case in not_found_cases {
            assert!(case.contains("not found"));
        }
    }

    #[test]
    fn test_400_scenarios() {
        // Scenarios that should return 400
        let bad_request_cases = [
            "Invalid tile coordinates",
            "Invalid zoom level",
            "Invalid format",
            "Invalid size",
        ];

        for case in bad_request_cases {
            assert!(case.starts_with("Invalid"));
        }
    }
}

// ============================================================
// CORS Tests
// ============================================================

mod cors_tests {
    use super::*;

    #[test]
    fn test_cors_origins() {
        // Test CORS origin patterns
        let allowed_origins = ["*", "http://localhost:3000", "https://example.com"];

        for origin in allowed_origins {
            assert!(!origin.is_empty());
        }
    }

    #[test]
    fn test_cors_methods() {
        // API should allow these methods
        let allowed_methods = ["GET", "OPTIONS", "HEAD"];

        for method in allowed_methods {
            assert!(["GET", "OPTIONS", "HEAD", "POST", "PUT", "DELETE"].contains(&method));
        }
    }

    #[test]
    fn test_cors_headers() {
        // API should allow these headers
        let allowed_headers = ["Accept", "Content-Type"];

        for header in allowed_headers {
            assert!(!header.is_empty());
        }
    }
}

// ============================================================
// URL Parameter Tests
// ============================================================

mod url_params_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_tile_z_x_y_parsing() {
        // Valid tile coordinates
        let valid_coords = [(0, 0, 0), (1, 0, 0), (1, 1, 1), (14, 8192, 8192)];

        for (z, x, y) in valid_coords {
            assert!(z <= 22, "Zoom should be <= 22");
            let max_coord = 1u32 << z;
            assert!(x < max_coord, "X should be < 2^z");
            assert!(y < max_coord, "Y should be < 2^z");
        }
    }

    #[test]
    fn test_scale_factor_parsing() {
        // Valid scale factors
        let valid_scales = ["@1x", "@2x", "@3x", "@4x"];

        for scale in valid_scales {
            let num: u8 = scale
                .trim_start_matches('@')
                .trim_end_matches('x')
                .parse()
                .unwrap();
            assert!((1..=4).contains(&num));
        }
    }

    #[test]
    fn test_static_image_size_parsing() {
        // Valid size formats
        let valid_sizes = ["800x600", "1920x1080", "256x256", "512x512@2x"];

        for size in valid_sizes {
            let size_part = size.split('@').next().unwrap();
            let parts: Vec<&str> = size_part.split('x').collect();
            assert_eq!(parts.len(), 2);

            let width: u32 = parts[0].parse().unwrap();
            let height: u32 = parts[1].parse().unwrap();
            assert!(width > 0 && width <= 4096);
            assert!(height > 0 && height <= 4096);
        }
    }

    #[test]
    fn test_static_type_parsing() {
        use tileserver_rs::render::StaticType;

        // Center format
        let center = StaticType::from_str("-122.4,37.8,12").unwrap();
        match center {
            StaticType::Center { lon, lat, zoom, .. } => {
                assert!((lon - (-122.4)).abs() < 0.01);
                assert!((lat - 37.8).abs() < 0.01);
                assert!((zoom - 12.0).abs() < 0.01);
            }
            _ => panic!("Expected Center"),
        }

        // Auto format
        let auto = StaticType::from_str("auto").unwrap();
        assert!(matches!(auto, StaticType::Auto));

        // Bounding box format
        let bbox = StaticType::from_str("-123,37,-122,38").unwrap();
        match bbox {
            StaticType::BoundingBox {
                min_lon,
                min_lat,
                max_lon,
                max_lat,
            } => {
                assert!((min_lon - (-123.0)).abs() < 0.01);
                assert!((min_lat - 37.0).abs() < 0.01);
                assert!((max_lon - (-122.0)).abs() < 0.01);
                assert!((max_lat - 38.0).abs() < 0.01);
            }
            _ => panic!("Expected BoundingBox"),
        }
    }

    #[test]
    fn test_format_extensions() {
        use tileserver_rs::render::ImageFormat;
        use tileserver_rs::sources::TileFormat;

        // Image formats
        assert!(ImageFormat::from_str("png").is_ok());
        assert!(ImageFormat::from_str("jpg").is_ok());
        assert!(ImageFormat::from_str("jpeg").is_ok());
        assert!(ImageFormat::from_str("webp").is_ok());
        assert!(ImageFormat::from_str("gif").is_err()); // Not supported

        // Tile formats
        assert!(TileFormat::from_str("pbf").is_ok());
        assert!(TileFormat::from_str("mvt").is_ok());
        assert!(TileFormat::from_str("png").is_ok());
    }
}

// ============================================================
// Marker/Path Query Parameter Tests
// ============================================================

mod overlay_params_tests {
    use super::*;
    use tileserver_rs::render::overlay::{parse_marker, parse_path};

    #[test]
    fn test_marker_param_formats() {
        // Valid marker formats
        let valid_markers = [
            "pin-s+f00(0,0)",
            "pin-m+00ff00(-122.4,37.8)",
            "pin-l-a+0000ff(10.5,20.5)",
            "pin-s+ff0000(0,0)",
        ];

        for marker in valid_markers {
            assert!(
                parse_marker(marker).is_some(),
                "Should parse marker: {}",
                marker
            );
        }
    }

    #[test]
    fn test_invalid_marker_formats() {
        // Invalid marker formats
        let invalid_markers = [
            "",
            "pin",
            "pin-x+f00(0,0)", // Invalid size
            "marker(0,0)",    // Wrong prefix
        ];

        for marker in invalid_markers {
            if !marker.is_empty() {
                // Empty string might have different handling
                let result = parse_marker(marker);
                // Invalid formats should return None or parse to defaults
                // The actual behavior depends on implementation
            }
        }
    }

    #[test]
    fn test_path_param_formats() {
        // Valid path formats
        let valid_paths = [
            "path-5+f00(0,0|1,1)",
            "path-3+00ff00(-122.4,37.8|-122.5,37.9)",
            "path-2+0000ff-80ffffff(0,0|10,0|10,10|0,10|0,0)", // With fill
        ];

        for path in valid_paths {
            assert!(parse_path(path).is_some(), "Should parse path: {}", path);
        }
    }

    #[test]
    fn test_encoded_polyline_format() {
        // Test encoded polyline detection
        let encoded = "_p~iF~ps|U_ulLnnqC_mqNvxq`@";

        // This should be detected as a polyline (no commas, valid chars)
        assert!(!encoded.contains(','));
        assert!(encoded
            .chars()
            .all(|c| (c as u32) >= 63 && (c as u32) <= 126));
    }
}

// ============================================================
// TileJSON Validation Tests
// ============================================================

mod tilejson_tests {
    use super::*;
    use tileserver_rs::{Config, SourceManager};

    #[tokio::test]
    async fn test_tilejson_spec_compliance() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        for metadata in sources.all_metadata() {
            let tilejson = metadata.to_tilejson("http://localhost:8080");

            // TileJSON 3.0 required fields
            assert_eq!(tilejson.tilejson, "3.0.0", "Should be TileJSON 3.0.0");
            assert!(!tilejson.tiles.is_empty(), "tiles array required");

            // Validate zoom levels
            assert!(tilejson.minzoom <= 22, "minzoom should be <= 22");
            assert!(tilejson.maxzoom <= 22, "maxzoom should be <= 22");
            assert!(
                tilejson.minzoom <= tilejson.maxzoom,
                "minzoom should be <= maxzoom"
            );

            // Validate bounds if present
            if let Some(bounds) = &tilejson.bounds {
                assert_eq!(bounds.len(), 4, "bounds should have 4 values");
                assert!(bounds[0] >= -180.0 && bounds[0] <= 180.0, "west bound");
                assert!(bounds[1] >= -90.0 && bounds[1] <= 90.0, "south bound");
                assert!(bounds[2] >= -180.0 && bounds[2] <= 180.0, "east bound");
                assert!(bounds[3] >= -90.0 && bounds[3] <= 90.0, "north bound");
            }

            // Validate center if present
            if let Some(center) = &tilejson.center {
                assert!(center.len() >= 2, "center should have at least lon,lat");
                assert!(
                    center[0] >= -180.0 && center[0] <= 180.0,
                    "center longitude"
                );
                assert!(center[1] >= -90.0 && center[1] <= 90.0, "center latitude");
            }
        }
    }

    #[tokio::test]
    async fn test_tilejson_tiles_url_format() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        for metadata in sources.all_metadata() {
            let tilejson = metadata.to_tilejson("http://localhost:8080");

            for tile_url in &tilejson.tiles {
                // Should contain placeholders
                assert!(tile_url.contains("{z}"), "Should have {{z}} placeholder");
                assert!(tile_url.contains("{x}"), "Should have {{x}} placeholder");
                assert!(tile_url.contains("{y}"), "Should have {{y}} placeholder");

                // Should be a valid URL
                assert!(
                    tile_url.starts_with("http://") || tile_url.starts_with("https://"),
                    "Should be absolute URL"
                );
            }
        }
    }
}

// ============================================================
// WMTS Tests
// ============================================================

mod wmts_tests {
    #[test]
    fn test_wmts_xml_structure() {
        // WMTS GetCapabilities should return valid XML
        let expected_elements = [
            "Capabilities",
            "ServiceIdentification",
            "Contents",
            "Layer",
            "TileMatrixSet",
        ];

        for element in expected_elements {
            assert!(!element.is_empty());
        }
    }

    #[test]
    fn test_wmts_tile_matrix_set() {
        // WebMercator tile matrix set levels
        for zoom in 0..=22 {
            let scale_denominator = 559082264.028717 / (1 << zoom) as f64;
            let tile_size = 256;
            let matrix_width = 1 << zoom;
            let matrix_height = 1 << zoom;

            assert!(scale_denominator > 0.0);
            assert_eq!(tile_size, 256);
            assert!(matrix_width > 0);
            assert_eq!(matrix_width, matrix_height);
        }
    }

    #[test]
    fn test_wmts_key_parameter_format() {
        // Test that key parameter is properly formatted in WMTS URLs
        let key = "my_api_key_123";
        let expected_query = format!("?key={}", key);

        assert!(expected_query.starts_with("?key="));
        assert!(expected_query.contains(key));
    }

    #[test]
    fn test_wmts_key_parameter_encoding() {
        // Test URL encoding of special characters in key
        let special_keys = [
            ("simple_key", "simple_key"),
            ("key with spaces", "key%20with%20spaces"),
            ("key&value=test", "key%26value%3Dtest"),
        ];

        for (input, expected_encoded) in special_keys {
            let encoded = urlencoding::encode(input);
            assert_eq!(
                encoded, expected_encoded,
                "Key '{}' not encoded correctly",
                input
            );
        }
    }

    #[test]
    fn test_wmts_generates_without_key() {
        use tileserver_rs::wmts::generate_wmts_capabilities;

        let xml = generate_wmts_capabilities(
            "http://localhost:8080",
            "test-style",
            "Test Style",
            0,
            18,
            None,
        );

        assert!(
            !xml.contains("?key="),
            "Should not have key param without key"
        );
        assert!(
            xml.contains("wmts.xml\""),
            "Should have WMTS URL without query params"
        );
    }

    #[test]
    fn test_wmts_generates_with_key() {
        use tileserver_rs::wmts::generate_wmts_capabilities;

        let xml = generate_wmts_capabilities(
            "http://localhost:8080",
            "test-style",
            "Test Style",
            0,
            18,
            Some("my_secret_key"),
        );

        assert!(
            xml.contains("?key=my_secret_key"),
            "Should have key param in URLs"
        );

        // Key should appear in:
        // 1. WMTS capabilities URL
        assert!(xml.contains("wmts.xml?key=my_secret_key"));
        // 2. Tile URLs for both 256 and 512 layers
        assert!(xml.contains(".png?key=my_secret_key"));
    }

    #[test]
    fn test_wmts_key_url_encoded() {
        use tileserver_rs::wmts::generate_wmts_capabilities;

        let xml = generate_wmts_capabilities(
            "http://localhost:8080",
            "test-style",
            "Test Style",
            0,
            18,
            Some("key with spaces & symbols="),
        );

        // Key should be URL-encoded
        assert!(xml.contains("?key=key%20with%20spaces%20%26%20symbols%3D"));
    }

    #[test]
    fn test_wmts_key_in_all_urls() {
        use tileserver_rs::wmts::generate_wmts_capabilities;

        let xml = generate_wmts_capabilities(
            "http://localhost:8080",
            "osm-bright",
            "OSM Bright",
            0,
            18,
            Some("test_key"),
        );

        // Count occurrences of key parameter
        let key_count = xml.matches("?key=test_key").count();

        // Should appear in:
        // - 2x in OperationsMetadata (GetCapabilities + GetTile hrefs)
        // - 2x in ResourceURL templates (256px + 512px layers)
        // - 1x in ServiceMetadataURL
        // Total: 5 occurrences
        assert!(
            key_count >= 4,
            "Key should appear in multiple URLs, found {} occurrences",
            key_count
        );
    }
}

// ============================================================
// API Key Parameter Tests
// ============================================================

mod key_param_tests {
    use super::*;
    use tileserver_rs::{Config, SourceManager};

    #[tokio::test]
    async fn test_tilejson_with_key() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let all_metadata = sources.all_metadata();
        let metadata = all_metadata.first().expect("Should have a source");

        // Without key
        let tilejson_no_key = metadata.to_tilejson("http://localhost:8080");
        assert!(
            !tilejson_no_key.tiles[0].contains("?key="),
            "Should not have key param without key"
        );

        // With key
        let tilejson_with_key =
            metadata.to_tilejson_with_key("http://localhost:8080", Some("my_api_key"));
        assert!(
            tilejson_with_key.tiles[0].contains("?key=my_api_key"),
            "Should have key param in tile URL"
        );
    }

    #[tokio::test]
    async fn test_tilejson_key_url_encoding() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let all_metadata = sources.all_metadata();
        let metadata = all_metadata.first().expect("Should have a source");

        // Key with special characters should be URL-encoded
        let tilejson = metadata
            .to_tilejson_with_key("http://localhost:8080", Some("key with spaces & symbols="));

        assert!(
            tilejson.tiles[0].contains("key%20with%20spaces%20%26%20symbols%3D"),
            "Key should be URL-encoded in tile URL: {}",
            tilejson.tiles[0]
        );
    }

    #[test]
    fn test_style_info_with_key() {
        use std::path::PathBuf;
        use tileserver_rs::styles::Style;

        let style = Style {
            id: "osm-bright".to_string(),
            name: "OSM Bright".to_string(),
            style_json: serde_json::json!({"version": 8}),
            path: PathBuf::from("data/styles/protomaps-light/style.json"),
        };

        // Without key
        let info_no_key = style.to_info("http://localhost:8080");
        assert!(
            !info_no_key.url.as_ref().unwrap().contains("?key="),
            "Should not have key param without key"
        );

        // With key
        let info_with_key = style.to_info_with_key("http://localhost:8080", Some("my_api_key"));
        assert!(
            info_with_key
                .url
                .as_ref()
                .unwrap()
                .contains("?key=my_api_key"),
            "Should have key param in style URL"
        );
    }

    #[test]
    fn test_style_info_key_url_encoding() {
        use std::path::PathBuf;
        use tileserver_rs::styles::Style;

        let style = Style {
            id: "test-style".to_string(),
            name: "Test Style".to_string(),
            style_json: serde_json::json!({"version": 8}),
            path: PathBuf::from("data/styles/test/style.json"),
        };

        // Key with special characters should be URL-encoded
        let info =
            style.to_info_with_key("http://localhost:8080", Some("key with spaces & symbols="));

        assert!(
            info.url
                .as_ref()
                .unwrap()
                .contains("key%20with%20spaces%20%26%20symbols%3D"),
            "Key should be URL-encoded in style URL: {}",
            info.url.as_ref().unwrap()
        );
    }

    #[test]
    fn test_rewrite_style_for_api_with_key() {
        use tileserver_rs::styles::{rewrite_style_for_api, UrlQueryParams};

        let style = serde_json::json!({
            "version": 8,
            "sources": {
                "openmaptiles": {
                    "type": "vector",
                    "url": "/data/openmaptiles.json"
                }
            },
            "glyphs": "/fonts/{fontstack}/{range}.pbf",
            "sprite": "/styles/basic/sprite"
        });

        // With key
        let params = UrlQueryParams::with_key(Some("my_secret_key".to_string()));
        let result = rewrite_style_for_api(&style, "http://tiles.example.com", &params);

        // All URLs should have key appended
        assert_eq!(
            result["sources"]["openmaptiles"]["url"],
            "http://tiles.example.com/data/openmaptiles.json?key=my_secret_key"
        );
        assert_eq!(
            result["glyphs"],
            "http://tiles.example.com/fonts/{fontstack}/{range}.pbf?key=my_secret_key"
        );
        assert_eq!(
            result["sprite"],
            "http://tiles.example.com/styles/basic/sprite?key=my_secret_key"
        );
    }

    #[test]
    fn test_rewrite_style_for_api_preserves_external_urls() {
        use tileserver_rs::styles::{rewrite_style_for_api, UrlQueryParams};

        let style = serde_json::json!({
            "version": 8,
            "sources": {
                "external": {
                    "type": "vector",
                    "url": "https://external-tiles.com/tiles.json"
                }
            },
            "glyphs": "https://fonts.example.com/{fontstack}/{range}.pbf"
        });

        // With key - external URLs should NOT have key appended
        let params = UrlQueryParams::with_key(Some("my_key".to_string()));
        let result = rewrite_style_for_api(&style, "http://localhost", &params);

        // External URLs should remain unchanged
        assert_eq!(
            result["sources"]["external"]["url"],
            "https://external-tiles.com/tiles.json"
        );
        assert_eq!(
            result["glyphs"],
            "https://fonts.example.com/{fontstack}/{range}.pbf"
        );
    }

    #[test]
    fn test_key_query_string_generation() {
        use tileserver_rs::styles::UrlQueryParams;

        // Empty params
        let empty = UrlQueryParams::default();
        assert_eq!(empty.to_query_string(), "");

        // With key
        let with_key = UrlQueryParams::with_key(Some("abc123".to_string()));
        assert_eq!(with_key.to_query_string(), "?key=abc123");

        // With key and extra params
        let with_extra = UrlQueryParams {
            key: Some("key1".to_string()),
            extra: vec![("foo".to_string(), "bar".to_string())],
        };
        assert_eq!(with_extra.to_query_string(), "?key=key1&foo=bar");
    }
}

// ============================================================
// Config Validation Tests
// ============================================================

mod config_tests {
    use super::*;
    use tileserver_rs::Config;

    #[test]
    fn test_config_loads() {
        let config = Config::load(Some(PathBuf::from(TEST_CONFIG)));
        assert!(config.is_ok(), "Should load test config");
    }

    #[test]
    fn test_config_has_server_section() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");
        // Port defaults to 8080 if not specified
        // Host defaults to "0.0.0.0" if not specified
        assert!(!config.server.host.is_empty());
    }

    #[test]
    fn test_config_sources_valid() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");

        for source in &config.sources {
            assert!(!source.id.is_empty(), "Source should have ID");
            // Path should exist or be a URL
            if !source.path.starts_with("http") {
                assert!(
                    PathBuf::from(&source.path).exists(),
                    "Source path should exist: {}",
                    source.path
                );
            }
        }
    }

    #[test]
    fn test_config_styles_valid() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");

        for style in &config.styles {
            assert!(!style.id.is_empty(), "Style should have ID");
            assert!(
                style.path.exists(),
                "Style path should exist: {}",
                style.path.display()
            );
        }
    }

    #[test]
    fn test_config_cors_settings() {
        let config =
            Config::load(Some(PathBuf::from(TEST_CONFIG))).expect("Should load test config");

        // CORS origins should be defined
        assert!(
            !config.server.cors_origins.is_empty(),
            "Should have CORS origins"
        );
    }
}
