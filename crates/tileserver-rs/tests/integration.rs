//! Integration tests for tileserver-rs HTTP endpoints
//!
//! These tests use actual HTTP requests against a test server with real fixtures
//! from the data/ directory.

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
        assert!(
            infos[0]
                .url
                .as_ref()
                .map(|u| u.contains("/styles/"))
                .unwrap_or(false)
        );
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

// ============================================================
// MLT Format Integration Tests
// ============================================================

mod mlt_format_tests {
    use tileserver_rs::{TileFormat, detect_mlt_format};

    #[test]
    fn test_mlt_format_properties() {
        assert_eq!(
            TileFormat::Mlt.content_type(),
            "application/vnd.maplibre-vector-tile"
        );
        assert_eq!(TileFormat::Mlt.extension(), "mlt");
        assert!(TileFormat::Mlt.is_vector());
    }

    #[test]
    fn test_mlt_format_from_str() {
        let format: TileFormat = "mlt".parse().unwrap();
        assert_eq!(format, TileFormat::Mlt);
    }

    #[test]
    fn test_detect_mlt_valid_tiles() {
        // Minimal valid MLT: size=1 (tag only), tag=0x01
        assert!(detect_mlt_format(&[0x01, 0x01]));

        // MLT with 3-byte payload: size=4 (tag + 3 bytes)
        assert!(detect_mlt_format(&[0x04, 0x01, 0xAA, 0xBB, 0xCC]));
    }

    #[test]
    fn test_detect_mlt_rejects_non_mlt() {
        assert!(!detect_mlt_format(&[]));
        assert!(!detect_mlt_format(&[0x00]));
        // gzip magic
        assert!(!detect_mlt_format(&[0x1f, 0x8b]));
        // protobuf field 1, wire type 2
        assert!(!detect_mlt_format(&[0x0a]));
    }

    #[test]
    fn test_mlt_tilejson_encoding() {
        use tileserver_rs::sources::TileMetadata;

        let metadata = TileMetadata {
            id: "test-mlt".to_string(),
            name: "Test MLT".to_string(),
            description: None,
            attribution: None,
            format: TileFormat::Mlt,
            minzoom: 0,
            maxzoom: 14,
            bounds: None,
            center: None,
            vector_layers: None,
        };

        let tilejson = metadata.to_tilejson("http://localhost:8080");
        assert_eq!(tilejson.encoding.as_deref(), Some("mlt"));
        assert!(tilejson.tiles[0].ends_with("/{z}/{x}/{y}.mlt"));
    }

    #[test]
    fn test_pbf_tilejson_no_encoding() {
        use tileserver_rs::sources::TileMetadata;

        let metadata = TileMetadata {
            id: "test-pbf".to_string(),
            name: "Test PBF".to_string(),
            description: None,
            attribution: None,
            format: TileFormat::Pbf,
            minzoom: 0,
            maxzoom: 14,
            bounds: None,
            center: None,
            vector_layers: None,
        };

        let tilejson = metadata.to_tilejson("http://localhost:8080");
        assert!(tilejson.encoding.is_none());
        assert!(tilejson.tiles[0].ends_with("/{z}/{x}/{y}.pbf"));
    }
}

mod mlt_mbtiles_tests {
    use rusqlite::Connection;
    use tempfile::NamedTempFile;
    use tileserver_rs::config::{SourceConfig, SourceType};
    use tileserver_rs::{TileFormat, TileSource};

    fn create_mlt_mbtiles() -> NamedTempFile {
        let tmp = NamedTempFile::new().expect("Should create temp file");
        let conn = Connection::open(tmp.path()).expect("Should open SQLite");

        conn.execute_batch(
            "CREATE TABLE metadata (name TEXT, value TEXT);
             INSERT INTO metadata VALUES ('name', 'Test MLT');
             INSERT INTO metadata VALUES ('format', 'mlt');
             INSERT INTO metadata VALUES ('minzoom', '0');
             INSERT INTO metadata VALUES ('maxzoom', '2');
             INSERT INTO metadata VALUES ('bounds', '-180,-85,180,85');
             INSERT INTO metadata VALUES ('center', '0,0,0');
             CREATE TABLE tiles (zoom_level INTEGER, tile_column INTEGER, tile_row INTEGER, tile_data BLOB);
             INSERT INTO tiles VALUES (0, 0, 0, x'0101');",
        )
        .expect("Should create MLT MBTiles schema");

        tmp
    }

    fn mlt_source_config(path: &str) -> SourceConfig {
        SourceConfig {
            id: "test-mlt".to_string(),
            source_type: SourceType::MBTiles,
            path: path.to_string(),
            name: Some("Test MLT Source".to_string()),
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
            collection: None,
            asset_role: "data".to_string(),
            dynamic: false,
            max_items: 500,
            stac_bbox: None,
            pixel_selection: tileserver_rs::config::PixelSelectionMethod::First,
            tile_path_template: None,
            tms: false,
            #[cfg(feature = "dem")]
            input_source: None,
            #[cfg(feature = "dem")]
            dem_encoding: tileserver_rs::config::DemEncoding::Terrarium,
            #[cfg(feature = "dem")]
            dem_scale: None,
            #[cfg(feature = "dem")]
            dem_offset: None,
            #[cfg(feature = "dem")]
            dem_band: 1,
            #[cfg(feature = "dem")]
            dem_nodata_color: None,
            filter: None,
        }
    }

    #[tokio::test]
    async fn test_mbtiles_detects_mlt_format() {
        let tmp = create_mlt_mbtiles();
        let config = mlt_source_config(tmp.path().to_str().unwrap());

        let source = tileserver_rs::sources::mbtiles::MbTilesSource::from_file(&config)
            .await
            .expect("Should load MLT MBTiles source");

        assert_eq!(source.metadata().format, TileFormat::Mlt);
    }

    #[tokio::test]
    async fn test_mbtiles_mlt_tile_serving() {
        let tmp = create_mlt_mbtiles();
        let config = mlt_source_config(tmp.path().to_str().unwrap());

        let source = tileserver_rs::sources::mbtiles::MbTilesSource::from_file(&config)
            .await
            .expect("Should load MLT MBTiles source");

        let tile = source
            .get_tile(0, 0, 0)
            .await
            .expect("Should not error")
            .expect("Tile 0/0/0 should exist");

        assert_eq!(tile.format, TileFormat::Mlt);
        assert_eq!(&*tile.data, &[0x01, 0x01]);
    }

    #[tokio::test]
    async fn test_mbtiles_mlt_tilejson_has_encoding() {
        let tmp = create_mlt_mbtiles();
        let config = mlt_source_config(tmp.path().to_str().unwrap());

        let source = tileserver_rs::sources::mbtiles::MbTilesSource::from_file(&config)
            .await
            .expect("Should load MLT MBTiles source");

        let tilejson = source.metadata().to_tilejson("http://localhost:8080");

        assert_eq!(tilejson.encoding.as_deref(), Some("mlt"));
        assert!(tilejson.tiles[0].ends_with("/{z}/{x}/{y}.mlt"));
    }

    #[tokio::test]
    async fn test_mbtiles_mlt_missing_tile_returns_none() {
        let tmp = create_mlt_mbtiles();
        let config = mlt_source_config(tmp.path().to_str().unwrap());

        let source = tileserver_rs::sources::mbtiles::MbTilesSource::from_file(&config)
            .await
            .expect("Should load MLT MBTiles source");

        let tile = source.get_tile(1, 0, 0).await.expect("Should not error");

        assert!(tile.is_none(), "Non-existent tile should return None");
    }

    #[tokio::test]
    async fn test_mbtiles_mime_type_string_detects_mlt() {
        let tmp = NamedTempFile::new().expect("Should create temp file");
        let conn = Connection::open(tmp.path()).expect("Should open SQLite");

        conn.execute_batch(
            "CREATE TABLE metadata (name TEXT, value TEXT);
             INSERT INTO metadata VALUES ('name', 'MIME MLT');
             INSERT INTO metadata VALUES ('format', 'application/vnd.maplibre-vector-tile');
             INSERT INTO metadata VALUES ('minzoom', '0');
             INSERT INTO metadata VALUES ('maxzoom', '0');
             CREATE TABLE tiles (zoom_level INTEGER, tile_column INTEGER, tile_row INTEGER, tile_data BLOB);
             INSERT INTO tiles VALUES (0, 0, 0, x'0101');",
        )
        .expect("Should create schema");

        let config = mlt_source_config(tmp.path().to_str().unwrap());
        let source = tileserver_rs::sources::mbtiles::MbTilesSource::from_file(&config)
            .await
            .expect("Should load source with MIME type format");

        assert_eq!(source.metadata().format, TileFormat::Mlt);
    }
}
