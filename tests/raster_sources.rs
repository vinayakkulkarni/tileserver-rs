//! E2E tests for raster/COG tile sources
//!
//! These tests verify the COG (Cloud Optimized GeoTIFF) source implementation including:
//! - Source loading and metadata extraction
//! - Raster tile generation
//! - Colormap application (continuous and discrete)
//! - Resampling methods
//! - CRS reprojection

#![cfg(feature = "raster")]

use std::path::{Path, PathBuf};

const RASTER_TEST_CONFIG: &str = "tests/config.raster.toml";

mod fixture_validation {
    use super::*;

    #[test]
    fn test_rgb_cog_fixture_exists() {
        let path = Path::new("data/raster/test-rgb.cog.tif");
        assert!(path.exists(), "RGB COG fixture should exist at {:?}", path);
    }

    #[test]
    fn test_dem_cog_fixture_exists() {
        let path = Path::new("data/raster/test-dem.cog.tif");
        assert!(path.exists(), "DEM COG fixture should exist at {:?}", path);
    }

    #[test]
    fn test_raster_config_exists() {
        let path = Path::new(RASTER_TEST_CONFIG);
        assert!(
            path.exists(),
            "Raster test config should exist at {:?}",
            path
        );
    }

    #[test]
    fn test_raster_config_parses() {
        let config_content =
            std::fs::read_to_string(RASTER_TEST_CONFIG).expect("Should read raster config");

        let parsed: toml::Value =
            toml::from_str(&config_content).expect("Config should be valid TOML");

        assert!(
            parsed.get("sources").is_some(),
            "Should have [[sources]] section"
        );
        assert!(
            parsed.get("raster").is_some(),
            "Should have [raster] section"
        );
    }
}

mod cog_source_loading {
    use super::*;
    use tileserver_rs::{Config, SourceManager};

    #[tokio::test]
    async fn test_load_cog_sources_from_config() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");

        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load COG sources from config");

        assert!(
            sources.len() >= 2,
            "Should have at least 2 COG sources loaded"
        );
    }

    #[tokio::test]
    async fn test_dem_cog_source_metadata() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let dem_source = sources
            .get("test-dem")
            .expect("Should have test-dem source");
        let metadata = dem_source.metadata();

        assert_eq!(metadata.id, "test-dem");
        assert_eq!(metadata.name, "Test DEM COG");
        assert!(metadata.bounds.is_some(), "Should have bounds");
        assert!(metadata.center.is_some(), "Should have center");
    }

    #[tokio::test]
    async fn test_rgb_cog_source_metadata() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let rgb_source = sources
            .get("test-rgb")
            .expect("Should have test-rgb source");
        let metadata = rgb_source.metadata();

        assert_eq!(metadata.id, "test-rgb");
        assert_eq!(metadata.name, "Test RGB COG");
        assert!(metadata.bounds.is_some(), "Should have bounds from GeoTIFF");

        let bounds = metadata.bounds.unwrap();
        assert!(
            bounds[0] >= -180.0 && bounds[0] <= 180.0,
            "West bound valid"
        );
        assert!(bounds[1] >= -90.0 && bounds[1] <= 90.0, "South bound valid");
        assert!(
            bounds[2] >= -180.0 && bounds[2] <= 180.0,
            "East bound valid"
        );
        assert!(bounds[3] >= -90.0 && bounds[3] <= 90.0, "North bound valid");
    }

    #[tokio::test]
    async fn test_cog_tilejson_generation() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let rgb_source = sources
            .get("test-rgb")
            .expect("Should have test-rgb source");
        let base_url = "http://localhost:8080";

        let tilejson = rgb_source.metadata().to_tilejson(base_url);

        assert_eq!(tilejson.tilejson, "3.0.0");
        assert!(!tilejson.tiles.is_empty());
        assert!(tilejson.tiles[0].contains("/data/test-rgb/"));
        assert!(tilejson.tiles[0].contains("{z}/{x}/{y}"));
    }
}

mod raster_tile_retrieval {
    use super::*;
    use tileserver_rs::{Config, SourceManager};

    #[tokio::test]
    async fn test_get_rgb_tile_z0() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let rgb_source = sources
            .get("test-rgb")
            .expect("Should have test-rgb source");

        let tile = rgb_source.get_tile(0, 0, 0).await;
        assert!(tile.is_ok(), "get_tile should not error");

        let tile_data = tile.unwrap();
        assert!(tile_data.is_some(), "Should return tile data at z0");

        let data = tile_data.unwrap();
        assert!(!data.data.is_empty(), "Tile data should not be empty");
        assert_eq!(
            data.format,
            tileserver_rs::sources::TileFormat::Png,
            "Should be PNG format"
        );
    }

    #[tokio::test]
    async fn test_get_rgb_tile_higher_zoom() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let rgb_source = sources
            .get("test-rgb")
            .expect("Should have test-rgb source");

        // Zoom level 10, tile coordinates covering San Francisco area
        let tile = rgb_source.get_tile(10, 163, 395).await;
        assert!(tile.is_ok(), "get_tile should not error at z10");
    }

    #[tokio::test]
    async fn test_get_dem_tile() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let dem_source = sources
            .get("test-dem")
            .expect("Should have test-dem source");

        let tile = dem_source.get_tile(0, 0, 0).await;
        assert!(tile.is_ok(), "get_tile should not error for DEM");

        let tile_data = tile.unwrap();
        assert!(tile_data.is_some(), "Should return DEM tile data");
    }

    #[tokio::test]
    async fn test_tile_is_valid_png() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let rgb_source = sources
            .get("test-rgb")
            .expect("Should have test-rgb source");

        let tile = rgb_source
            .get_tile(0, 0, 0)
            .await
            .expect("Should get tile")
            .expect("Should have tile data");

        // PNG magic bytes
        let png_signature = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(
            tile.data.len() >= 8,
            "PNG should have at least 8 bytes for header"
        );
        assert_eq!(
            &tile.data[0..8],
            &png_signature,
            "Should have valid PNG signature"
        );
    }

    #[tokio::test]
    async fn test_tile_dimensions() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let rgb_source = sources
            .get("test-rgb")
            .expect("Should have test-rgb source");

        let tile = rgb_source
            .get_tile(0, 0, 0)
            .await
            .expect("Should get tile")
            .expect("Should have tile data");

        // Decode PNG to verify dimensions
        let img = image::load_from_memory(&tile.data).expect("Should decode PNG");
        assert_eq!(img.width(), 256, "Tile width should be 256");
        assert_eq!(img.height(), 256, "Tile height should be 256");
    }
}

mod colormap_tests {
    use super::*;
    use tileserver_rs::{Config, SourceManager};

    #[tokio::test]
    async fn test_continuous_colormap_applied() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let dem_source = sources
            .get("test-dem")
            .expect("Should have test-dem source");

        let tile = dem_source
            .get_tile(0, 0, 0)
            .await
            .expect("Should get tile")
            .expect("Should have tile data");

        // Decode and verify the image has color (not grayscale)
        let img = image::load_from_memory(&tile.data).expect("Should decode PNG");
        let rgb = img.to_rgb8();

        // Sample some pixels to verify colormap was applied
        // With continuous colormap, we should see a gradient of colors
        let mut has_non_gray = false;
        for pixel in rgb.pixels() {
            if pixel[0] != pixel[1] || pixel[1] != pixel[2] {
                has_non_gray = true;
                break;
            }
        }
        assert!(has_non_gray, "Colormap should produce non-grayscale output");
    }

    #[tokio::test]
    async fn test_discrete_colormap_applied() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let dem_discrete = sources
            .get("test-dem-discrete")
            .expect("Should have test-dem-discrete source");

        let tile = dem_discrete
            .get_tile(0, 0, 0)
            .await
            .expect("Should get tile")
            .expect("Should have tile data");

        let img = image::load_from_memory(&tile.data).expect("Should decode PNG");
        assert_eq!(img.width(), 256);
        assert_eq!(img.height(), 256);
    }

    #[tokio::test]
    async fn test_rgb_no_colormap() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let rgb_source = sources
            .get("test-rgb")
            .expect("Should have test-rgb source");

        let tile = rgb_source
            .get_tile(0, 0, 0)
            .await
            .expect("Should get tile")
            .expect("Should have tile data");

        // RGB source should render without colormap
        let img = image::load_from_memory(&tile.data).expect("Should decode PNG");
        assert!(img.color().has_color(), "RGB should have color channels");
    }
}

mod resampling_tests {
    use super::*;
    use tileserver_rs::config::ResamplingMethod;
    use tileserver_rs::sources::cog::CogSource;
    use tileserver_rs::{Config, SourceManager};

    #[tokio::test]
    async fn test_default_resampling_from_config() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let rgb_source = sources
            .get("test-rgb")
            .expect("Should have test-rgb source");

        // Access the COG source to check resampling method
        if let Some(cog) = rgb_source.as_any().downcast_ref::<CogSource>() {
            assert_eq!(
                cog.resampling(),
                ResamplingMethod::Bilinear,
                "RGB source should use bilinear resampling"
            );
        } else {
            panic!("test-rgb should be a CogSource");
        }
    }

    #[tokio::test]
    async fn test_cubic_resampling_from_config() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let dem_source = sources
            .get("test-dem")
            .expect("Should have test-dem source");

        if let Some(cog) = dem_source.as_any().downcast_ref::<CogSource>() {
            assert_eq!(
                cog.resampling(),
                ResamplingMethod::Cubic,
                "DEM source should use cubic resampling"
            );
        } else {
            panic!("test-dem should be a CogSource");
        }
    }

    #[tokio::test]
    async fn test_resampling_with_override() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let rgb_source = sources
            .get("test-rgb")
            .expect("Should have test-rgb source");

        if let Some(cog) = rgb_source.as_any().downcast_ref::<CogSource>() {
            // Test with different resampling methods
            let tile_bilinear = cog
                .get_tile_with_resampling(0, 0, 0, 256, ResamplingMethod::Bilinear)
                .await
                .expect("Should get tile with bilinear");

            let tile_nearest = cog
                .get_tile_with_resampling(0, 0, 0, 256, ResamplingMethod::Nearest)
                .await
                .expect("Should get tile with nearest");

            assert!(tile_bilinear.is_some());
            assert!(tile_nearest.is_some());

            // Both should produce valid PNG tiles
            let data_bilinear = tile_bilinear.unwrap();
            let data_nearest = tile_nearest.unwrap();

            assert!(!data_bilinear.data.is_empty());
            assert!(!data_nearest.data.is_empty());
        }
    }

    #[tokio::test]
    async fn test_custom_tile_size() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let rgb_source = sources
            .get("test-rgb")
            .expect("Should have test-rgb source");

        if let Some(cog) = rgb_source.as_any().downcast_ref::<CogSource>() {
            // Test with 512x512 tile size
            let tile_512 = cog
                .get_tile_with_resampling(0, 0, 0, 512, ResamplingMethod::Bilinear)
                .await
                .expect("Should get 512px tile")
                .expect("Should have data");

            let img = image::load_from_memory(&tile_512.data).expect("Should decode PNG");
            assert_eq!(img.width(), 512, "Tile width should be 512");
            assert_eq!(img.height(), 512, "Tile height should be 512");
        }
    }
}

mod reprojection_tests {
    use super::*;
    use tileserver_rs::{Config, SourceManager};

    #[tokio::test]
    async fn test_wgs84_to_web_mercator_reprojection() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        // Our test COGs are in WGS84 (EPSG:4326)
        // Tiles should be served in Web Mercator (EPSG:3857)
        let rgb_source = sources
            .get("test-rgb")
            .expect("Should have test-rgb source");

        // Get a tile - this exercises the reprojection path
        let tile = rgb_source.get_tile(0, 0, 0).await.expect("Should get tile");

        assert!(tile.is_some(), "Reprojected tile should be available");
    }

    #[tokio::test]
    async fn test_bounds_are_wgs84() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let rgb_source = sources
            .get("test-rgb")
            .expect("Should have test-rgb source");
        let metadata = rgb_source.metadata();
        let bounds = metadata.bounds.expect("Should have bounds");

        assert!(
            bounds[0] >= -180.0 && bounds[0] <= 180.0,
            "West bound in WGS84 range"
        );
        assert!(
            bounds[1] >= -90.0 && bounds[1] <= 90.0,
            "South bound in WGS84 range"
        );
        assert!(
            bounds[2] >= -180.0 && bounds[2] <= 180.0,
            "East bound in WGS84 range"
        );
        assert!(
            bounds[3] >= -90.0 && bounds[3] <= 90.0,
            "North bound in WGS84 range"
        );
    }
}

mod config_parsing {
    use tileserver_rs::config::{ColorMapConfig, ColorMapType, ResamplingMethod, RescaleMode};

    #[test]
    fn test_resampling_method_parsing() {
        let methods = [
            ("nearest", ResamplingMethod::Nearest),
            ("bilinear", ResamplingMethod::Bilinear),
            ("cubic", ResamplingMethod::Cubic),
            ("cubicspline", ResamplingMethod::CubicSpline),
            ("lanczos", ResamplingMethod::Lanczos),
            ("average", ResamplingMethod::Average),
            ("mode", ResamplingMethod::Mode),
        ];

        for (name, expected) in methods {
            let parsed: ResamplingMethod = name.parse().expect("Should parse");
            assert_eq!(parsed, expected, "Failed for {}", name);
        }
    }

    #[test]
    fn test_colormap_type_serialization() {
        let continuous = ColorMapType::Continuous;
        let discrete = ColorMapType::Discrete;

        let json_cont = serde_json::to_string(&continuous).unwrap();
        let json_disc = serde_json::to_string(&discrete).unwrap();

        assert_eq!(json_cont, "\"continuous\"");
        assert_eq!(json_disc, "\"discrete\"");
    }

    #[test]
    fn test_colormap_color_parsing() {
        let color = ColorMapConfig::parse_color("#ff0000");
        assert_eq!(color, Some([255, 0, 0, 255]));

        let color_alpha = ColorMapConfig::parse_color("#ff000080");
        assert_eq!(color_alpha, Some([255, 0, 0, 128]));

        let color_no_hash = ColorMapConfig::parse_color("00ff00");
        assert_eq!(color_no_hash, Some([0, 255, 0, 255]));

        let invalid = ColorMapConfig::parse_color("invalid");
        assert_eq!(invalid, None);
    }

    #[test]
    fn test_rescale_mode_serialization() {
        let static_mode = RescaleMode::Static;
        let dynamic_mode = RescaleMode::Dynamic;

        let json_static = serde_json::to_string(&static_mode).unwrap();
        let json_dynamic = serde_json::to_string(&dynamic_mode).unwrap();

        assert_eq!(json_static, "\"static\"");
        assert_eq!(json_dynamic, "\"dynamic\"");
    }

    #[test]
    fn test_rescale_mode_deserialization() {
        let static_mode: RescaleMode = serde_json::from_str("\"static\"").unwrap();
        let dynamic_mode: RescaleMode = serde_json::from_str("\"dynamic\"").unwrap();

        assert_eq!(static_mode, RescaleMode::Static);
        assert_eq!(dynamic_mode, RescaleMode::Dynamic);
    }

    #[test]
    fn test_rescale_mode_default() {
        assert_eq!(RescaleMode::default(), RescaleMode::Static);
    }
}

mod error_handling {
    use super::*;
    use tileserver_rs::config::{SourceConfig, SourceType};
    use tileserver_rs::sources::cog::CogSource;
    use tileserver_rs::{Config, SourceManager};

    #[tokio::test]
    async fn test_invalid_cog_path() {
        let config = SourceConfig {
            id: "invalid".to_string(),
            source_type: SourceType::Cog,
            path: "nonexistent/path/to/file.tif".to_string(),
            name: None,
            attribution: None,
            resampling: None,
            colormap: None,
        };

        let result = CogSource::from_file(&config).await;
        assert!(result.is_err(), "Should error on invalid path");
    }

    #[tokio::test]
    async fn test_tile_outside_bounds() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let rgb_source = sources
            .get("test-rgb")
            .expect("Should have test-rgb source");

        // Request a tile far outside the data bounds (Antarctica)
        let tile = rgb_source.get_tile(10, 500, 900).await;

        // Should not error, but tile data might be empty/nodata
        assert!(tile.is_ok(), "Should handle out-of-bounds gracefully");
    }
}

mod source_manager_integration {
    use super::*;
    use tileserver_rs::config::ResamplingMethod;
    use tileserver_rs::{Config, SourceManager};

    #[tokio::test]
    async fn test_get_raster_tile_via_manager() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        // Test the manager's get_raster_tile method
        let tile = sources
            .get_raster_tile("test-rgb", 0, 0, 0, 256, Some(ResamplingMethod::Bilinear))
            .await;

        assert!(tile.is_ok(), "Manager should return tile");
        assert!(tile.unwrap().is_some(), "Should have tile data");
    }

    #[tokio::test]
    async fn test_get_raster_tile_nonexistent_source() {
        let config = Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG)))
            .expect("Should load raster test config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let tile = sources
            .get_raster_tile("nonexistent", 0, 0, 0, 256, None)
            .await;

        assert!(tile.is_err(), "Should error on nonexistent source");
    }

    #[tokio::test]
    async fn test_mixed_source_types() {
        // Load config with both vector and raster sources
        let config =
            Config::load(Some(PathBuf::from(RASTER_TEST_CONFIG))).expect("Should load config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        // All COG sources should be accessible
        assert!(sources.get("test-rgb").is_some());
        assert!(sources.get("test-dem").is_some());
        assert!(sources.get("test-dem-discrete").is_some());
    }
}
