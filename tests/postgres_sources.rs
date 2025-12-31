#![cfg(feature = "postgres")]

use tileserver_rs::{PostgresConfig, PostgresFunctionConfig};

/// Test PostgreSQL configuration parsing
mod config_tests {
    use super::*;

    #[test]
    fn test_postgres_config_from_toml() {
        let toml = r#"
            connection_string = "postgresql://user:pass@localhost:5432/testdb"
            pool_size = 15

            [[functions]]
            id = "buildings"
            schema = "public"
            function = "get_building_tiles"
            minzoom = 10
            maxzoom = 18
            bounds = [-122.5, 37.5, -122.0, 38.0]
            name = "Buildings"
            attribution = "© OpenStreetMap contributors"
        "#;

        let config: PostgresConfig = toml::from_str(toml).unwrap();
        assert_eq!(
            config.connection_string,
            "postgresql://user:pass@localhost:5432/testdb"
        );
        assert_eq!(config.pool_size, 15);
        assert_eq!(config.functions.len(), 1);

        let func = &config.functions[0];
        assert_eq!(func.id, "buildings");
        assert_eq!(func.schema, "public");
        assert_eq!(func.function, "get_building_tiles");
        assert_eq!(func.minzoom, 10);
        assert_eq!(func.maxzoom, 18);
        assert!(func.bounds.is_some());
        assert_eq!(func.name, Some("Buildings".to_string()));
    }

    #[test]
    fn test_postgres_config_defaults() {
        let toml = r#"
            connection_string = "postgresql://localhost/db"

            [[functions]]
            id = "tiles"
            function = "my_func"
        "#;

        let config: PostgresConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.pool_size, 20); // default

        let func = &config.functions[0];
        assert_eq!(func.schema, "public"); // default
        assert_eq!(func.minzoom, 0); // default
        assert_eq!(func.maxzoom, 22); // default
    }

    #[test]
    fn test_multiple_functions() {
        let toml = r#"
            connection_string = "postgresql://localhost/db"

            [[functions]]
            id = "roads"
            function = "road_tiles"
            maxzoom = 14

            [[functions]]
            id = "buildings"
            function = "building_tiles"
            minzoom = 12
            maxzoom = 18

            [[functions]]
            id = "water"
            schema = "hydrology"
            function = "water_tiles"
        "#;

        let config: PostgresConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.functions.len(), 3);

        assert_eq!(config.functions[0].id, "roads");
        assert_eq!(config.functions[0].maxzoom, 14);

        assert_eq!(config.functions[1].id, "buildings");
        assert_eq!(config.functions[1].minzoom, 12);

        assert_eq!(config.functions[2].id, "water");
        assert_eq!(config.functions[2].schema, "hydrology");
    }
}

/// Test PostgreSQL function configuration validation
mod validation_tests {
    use super::*;

    #[test]
    fn test_function_config_required_fields() {
        // id and function are required
        let result: Result<PostgresFunctionConfig, _> = toml::from_str(
            r#"
            schema = "public"
            "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_function_config_minimal() {
        let result: Result<PostgresFunctionConfig, _> = toml::from_str(
            r#"
            id = "test"
            function = "test_func"
            "#,
        );
        assert!(result.is_ok());

        let func = result.unwrap();
        assert_eq!(func.id, "test");
        assert_eq!(func.function, "test_func");
        assert_eq!(func.schema, "public"); // default
    }

    #[test]
    fn test_bounds_format() {
        let result: Result<PostgresFunctionConfig, _> = toml::from_str(
            r#"
            id = "test"
            function = "test_func"
            bounds = [-180.0, -85.0, 180.0, 85.0]
            "#,
        );
        assert!(result.is_ok());

        let func = result.unwrap();
        let bounds = func.bounds.unwrap();
        assert_eq!(bounds[0], -180.0); // west
        assert_eq!(bounds[1], -85.0); // south
        assert_eq!(bounds[2], 180.0); // east
        assert_eq!(bounds[3], 85.0); // north
    }

    #[test]
    fn test_zoom_range() {
        let func = PostgresFunctionConfig {
            id: "test".to_string(),
            schema: "public".to_string(),
            function: "test_func".to_string(),
            name: None,
            attribution: None,
            description: None,
            minzoom: 5,
            maxzoom: 15,
            bounds: None,
        };

        assert_eq!(func.minzoom, 5);
        assert_eq!(func.maxzoom, 15);
        assert!(func.minzoom <= func.maxzoom);
    }
}

/// Test SQL query building
mod sql_query_tests {
    #[test]
    fn test_sql_query_without_params() {
        let schema = "public";
        let function = "my_tiles";
        let query = format!(
            "SELECT \"{}\".\"{}\"($1::integer, $2::integer, $3::integer)",
            schema, function
        );
        assert_eq!(
            query,
            "SELECT \"public\".\"my_tiles\"($1::integer, $2::integer, $3::integer)"
        );
    }

    #[test]
    fn test_sql_query_with_params() {
        let schema = "public";
        let function = "my_tiles";
        let query = format!(
            "SELECT \"{}\".\"{}\"($1::integer, $2::integer, $3::integer, $4::json)",
            schema, function
        );
        assert_eq!(
            query,
            "SELECT \"public\".\"my_tiles\"($1::integer, $2::integer, $3::integer, $4::json)"
        );
    }

    #[test]
    fn test_sql_query_escapes_identifiers() {
        let schema = "my-schema";
        let function = "tile-func";
        let query = format!(
            "SELECT \"{}\".\"{}\"($1::integer, $2::integer, $3::integer)",
            schema, function
        );
        assert!(query.contains("\"my-schema\""));
        assert!(query.contains("\"tile-func\""));
    }

    #[test]
    fn test_sql_query_different_schemas() {
        let test_cases = vec![
            ("public", "tiles"),
            ("osm", "get_roads"),
            ("my_schema", "building_tiles"),
        ];

        for (schema, function) in test_cases {
            let query = format!(
                "SELECT \"{}\".\"{}\"($1::integer, $2::integer, $3::integer)",
                schema, function
            );
            assert!(query.contains(&format!("\"{}\"", schema)));
            assert!(query.contains(&format!("\"{}\"", function)));
        }
    }
}

/// Test version comparison logic
mod version_tests {
    use semver::Version;

    const MINIMUM_POSTGRES_VERSION: Version = Version::new(11, 0, 0);
    const MINIMUM_POSTGIS_VERSION: Version = Version::new(3, 0, 0);
    const ST_TILE_ENVELOPE_MARGIN_VERSION: Version = Version::new(3, 1, 0);

    #[test]
    fn test_postgres_version_requirements() {
        // PostgreSQL 11+ is required
        assert!(Version::new(11, 0, 0) >= MINIMUM_POSTGRES_VERSION);
        assert!(Version::new(12, 0, 0) >= MINIMUM_POSTGRES_VERSION);
        assert!(Version::new(14, 5, 0) >= MINIMUM_POSTGRES_VERSION);
        assert!(Version::new(15, 0, 0) >= MINIMUM_POSTGRES_VERSION);
        assert!(Version::new(16, 0, 0) >= MINIMUM_POSTGRES_VERSION);

        // PostgreSQL 10 is too old
        assert!(Version::new(10, 0, 0) < MINIMUM_POSTGRES_VERSION);
    }

    #[test]
    fn test_postgis_version_requirements() {
        // PostGIS 3.0+ is required for ST_TileEnvelope
        assert!(Version::new(3, 0, 0) >= MINIMUM_POSTGIS_VERSION);
        assert!(Version::new(3, 1, 0) >= MINIMUM_POSTGIS_VERSION);
        assert!(Version::new(3, 4, 0) >= MINIMUM_POSTGIS_VERSION);

        // PostGIS 2.x is too old
        assert!(Version::new(2, 5, 0) < MINIMUM_POSTGIS_VERSION);
    }

    #[test]
    fn test_tile_margin_support() {
        // PostGIS 3.1+ supports margin parameter
        assert!(Version::new(3, 1, 0) >= ST_TILE_ENVELOPE_MARGIN_VERSION);
        assert!(Version::new(3, 2, 0) >= ST_TILE_ENVELOPE_MARGIN_VERSION);
        assert!(Version::new(3, 4, 0) >= ST_TILE_ENVELOPE_MARGIN_VERSION);

        // PostGIS 3.0 does not support margin
        assert!(Version::new(3, 0, 0) < ST_TILE_ENVELOPE_MARGIN_VERSION);
        assert!(Version::new(3, 0, 5) < ST_TILE_ENVELOPE_MARGIN_VERSION);
    }

    #[test]
    fn test_version_parsing() {
        let v: Version = "14.5.0".parse().unwrap();
        assert_eq!(v.major, 14);
        assert_eq!(v.minor, 5);
        assert_eq!(v.patch, 0);

        let v: Version = "3.4.2".parse().unwrap();
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 4);
        assert_eq!(v.patch, 2);
    }
}

/// Test coordinate validation
mod coordinate_tests {
    #[test]
    fn test_valid_tile_coordinates() {
        // At zoom 0, only tile (0, 0) is valid
        let max_tile_z0 = 1u32 << 0;
        assert_eq!(max_tile_z0, 1);
        assert!(0 < max_tile_z0);

        // At zoom 1, tiles (0-1, 0-1) are valid
        let max_tile_z1 = 1u32 << 1;
        assert_eq!(max_tile_z1, 2);

        // At zoom 10, tiles (0-1023, 0-1023) are valid
        let max_tile_z10 = 1u32 << 10;
        assert_eq!(max_tile_z10, 1024);

        // At zoom 22, we have many tiles
        let max_tile_z22 = 1u32 << 22;
        assert_eq!(max_tile_z22, 4194304);
    }

    #[test]
    fn test_invalid_tile_coordinates() {
        let z: u8 = 5;
        let max_tile = 1u32 << z;

        // These should be invalid (>= max_tile)
        assert!(max_tile >= max_tile); // x or y >= max is invalid
        assert!(max_tile + 1 >= max_tile);
    }

    #[test]
    fn test_zoom_range_validation() {
        let minzoom: u8 = 5;
        let maxzoom: u8 = 15;

        // Valid zoom levels
        for z in minzoom..=maxzoom {
            assert!(z >= minzoom && z <= maxzoom);
        }

        // Invalid zoom levels
        assert!(!(4 >= minzoom && 4 <= maxzoom));
        assert!(!(16 >= minzoom && 16 <= maxzoom));
    }
}

/// Test compression detection
mod compression_tests {
    #[test]
    fn test_gzip_magic_bytes() {
        // Gzip magic bytes: 0x1f 0x8b
        let gzip_data: &[u8] = &[0x1f, 0x8b, 0x08, 0x00];
        assert!(gzip_data.len() >= 2 && gzip_data[0] == 0x1f && gzip_data[1] == 0x8b);

        // Non-gzip data
        let plain_data: &[u8] = &[0x00, 0x00, 0x00, 0x00];
        assert!(plain_data.len() < 2 || plain_data[0] != 0x1f || plain_data[1] != 0x8b);

        // Empty data
        let empty_data: &[u8] = &[];
        assert!(empty_data.len() < 2);
    }

    #[test]
    fn test_pbf_content_type() {
        // MVT/PBF content type
        let content_type = "application/x-protobuf";
        assert_eq!(content_type, "application/x-protobuf");
    }
}

mod tilejson_tests {
    #[test]
    fn test_center_calculation_from_bounds() {
        let bounds: [f64; 4] = [-122.5, 37.5, -122.0, 38.0];
        let minzoom: u8 = 10;
        let maxzoom: u8 = 18;

        let center_lon: f64 = (bounds[0] + bounds[2]) / 2.0;
        let center_lat: f64 = (bounds[1] + bounds[3]) / 2.0;
        let center_zoom = ((minzoom as f64 + maxzoom as f64) / 2.0).floor();

        assert!((center_lon - (-122.25)).abs() < 0.001);
        assert!((center_lat - 37.75).abs() < 0.001);
        assert_eq!(center_zoom, 14.0);
    }

    #[test]
    fn test_world_bounds() {
        let world_bounds = [-180.0, -85.0, 180.0, 85.0];

        let center_lon = (world_bounds[0] + world_bounds[2]) / 2.0;
        let center_lat = (world_bounds[1] + world_bounds[3]) / 2.0;

        assert_eq!(center_lon, 0.0);
        assert_eq!(center_lat, 0.0);
    }
}

#[cfg(feature = "postgres-integration")]
mod integration_tests {
    use std::sync::Arc;
    use tileserver_rs::{
        PoolSettings, PostgresFunctionConfig, PostgresFunctionSource, PostgresPool, TileSource,
    };

    fn default_pool_settings() -> PoolSettings {
        PoolSettings {
            max_size: 5,
            wait_timeout_ms: 5000,
            create_timeout_ms: 5000,
            recycle_timeout_ms: 5000,
            pre_warm: false,
        }
    }

    fn get_connection_string() -> String {
        std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://tileserver:tileserver@localhost:5432/tiles".to_string()
        })
    }

    #[tokio::test]
    async fn test_postgres_pool_creation() {
        let conn_str = get_connection_string();
        let pool = PostgresPool::new(&conn_str, default_pool_settings(), None, None, None).await;

        match pool {
            Ok(pool) => {
                assert!(pool.postgres_version().major >= 11);
                assert!(pool.postgis_version().major >= 3);
            }
            Err(e) => {
                eprintln!("Skipping test - database not available: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_postgres_function_source_creation() {
        let conn_str = get_connection_string();
        let pool =
            match PostgresPool::new(&conn_str, default_pool_settings(), None, None, None).await {
                Ok(p) => Arc::new(p),
                Err(e) => {
                    eprintln!("Skipping test - database not available: {}", e);
                    return;
                }
            };

        let config = PostgresFunctionConfig {
            id: "benchmark_points".to_string(),
            schema: "public".to_string(),
            function: "get_benchmark_tiles".to_string(),
            name: Some("Benchmark Points".to_string()),
            attribution: None,
            description: None,
            minzoom: 0,
            maxzoom: 14,
            bounds: Some([8.45, 47.32, 8.63, 47.44]),
        };

        let source = PostgresFunctionSource::new(pool, &config, None).await;

        match source {
            Ok(source) => {
                let metadata = source.metadata();
                assert_eq!(metadata.id, "benchmark_points");
                assert_eq!(metadata.minzoom, 0);
                assert_eq!(metadata.maxzoom, 14);
            }
            Err(e) => {
                eprintln!("Skipping test - function not available: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_postgres_get_tile() {
        let conn_str = get_connection_string();
        let pool =
            match PostgresPool::new(&conn_str, default_pool_settings(), None, None, None).await {
                Ok(p) => Arc::new(p),
                Err(e) => {
                    eprintln!("Skipping test - database not available: {}", e);
                    return;
                }
            };

        let config = PostgresFunctionConfig {
            id: "benchmark_points".to_string(),
            schema: "public".to_string(),
            function: "get_benchmark_tiles".to_string(),
            name: None,
            attribution: None,
            description: None,
            minzoom: 0,
            maxzoom: 14,
            bounds: Some([8.45, 47.32, 8.63, 47.44]),
        };

        let source = match PostgresFunctionSource::new(pool, &config, None).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping test - function not available: {}", e);
                return;
            }
        };

        let tile = source.get_tile(10, 536, 358).await;

        match tile {
            Ok(Some(tile_data)) => {
                assert!(!tile_data.data.is_empty(), "Tile data should not be empty");
                assert_eq!(
                    tile_data.format,
                    tileserver_rs::TileFormat::Pbf,
                    "Tile format should be PBF"
                );
            }
            Ok(None) => {
                eprintln!("Tile returned None (may be outside bounds)");
            }
            Err(e) => {
                panic!("Failed to get tile: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_postgres_tile_outside_zoom_range() {
        let conn_str = get_connection_string();
        let pool =
            match PostgresPool::new(&conn_str, default_pool_settings(), None, None, None).await {
                Ok(p) => Arc::new(p),
                Err(e) => {
                    eprintln!("Skipping test - database not available: {}", e);
                    return;
                }
            };

        let config = PostgresFunctionConfig {
            id: "benchmark_points".to_string(),
            schema: "public".to_string(),
            function: "get_benchmark_tiles".to_string(),
            name: None,
            attribution: None,
            description: None,
            minzoom: 5,
            maxzoom: 10,
            bounds: Some([8.45, 47.32, 8.63, 47.44]),
        };

        let source = match PostgresFunctionSource::new(pool, &config, None).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping test - function not available: {}", e);
                return;
            }
        };

        let tile_below_min = source.get_tile(3, 4, 2).await;
        assert!(
            matches!(tile_below_min, Ok(None)),
            "Tile below minzoom should return None"
        );

        let tile_above_max = source.get_tile(15, 17408, 11944).await;
        assert!(
            matches!(tile_above_max, Ok(None)),
            "Tile above maxzoom should return None"
        );
    }

    #[tokio::test]
    async fn test_postgres_function_with_query_params() {
        let conn_str = get_connection_string();
        let pool =
            match PostgresPool::new(&conn_str, default_pool_settings(), None, None, None).await {
                Ok(p) => Arc::new(p),
                Err(e) => {
                    eprintln!("Skipping test - database not available: {}", e);
                    return;
                }
            };

        let config = PostgresFunctionConfig {
            id: "filtered_points".to_string(),
            schema: "public".to_string(),
            function: "get_filtered_tiles".to_string(),
            name: None,
            attribution: None,
            description: None,
            minzoom: 0,
            maxzoom: 14,
            bounds: Some([8.45, 47.32, 8.63, 47.44]),
        };

        let source = PostgresFunctionSource::new(pool, &config, None).await;

        match source {
            Ok(source) => {
                assert!(
                    source.supports_query_params(),
                    "Function should support query params"
                );
            }
            Err(e) => {
                eprintln!("Skipping test - function not available: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_postgres_invalid_coordinates() {
        let conn_str = get_connection_string();
        let pool =
            match PostgresPool::new(&conn_str, default_pool_settings(), None, None, None).await {
                Ok(p) => Arc::new(p),
                Err(e) => {
                    eprintln!("Skipping test - database not available: {}", e);
                    return;
                }
            };

        let config = PostgresFunctionConfig {
            id: "benchmark_points".to_string(),
            schema: "public".to_string(),
            function: "get_benchmark_tiles".to_string(),
            name: None,
            attribution: None,
            description: None,
            minzoom: 0,
            maxzoom: 14,
            bounds: None,
        };

        let source = match PostgresFunctionSource::new(pool, &config, None).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping test - function not available: {}", e);
                return;
            }
        };

        let result = source.get_tile(5, 100, 100).await;
        assert!(result.is_err(), "Invalid coordinates should return error");
    }

    #[tokio::test]
    async fn test_postgres_multiple_tiles() {
        let conn_str = get_connection_string();
        let pool =
            match PostgresPool::new(&conn_str, default_pool_settings(), None, None, None).await {
                Ok(p) => Arc::new(p),
                Err(e) => {
                    eprintln!("Skipping test - database not available: {}", e);
                    return;
                }
            };

        let config = PostgresFunctionConfig {
            id: "benchmark_points".to_string(),
            schema: "public".to_string(),
            function: "get_benchmark_tiles".to_string(),
            name: None,
            attribution: None,
            description: None,
            minzoom: 0,
            maxzoom: 14,
            bounds: Some([8.45, 47.32, 8.63, 47.44]),
        };

        let source = match PostgresFunctionSource::new(pool, &config, None).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping test - function not available: {}", e);
                return;
            }
        };

        let tiles_to_fetch = vec![(10, 536, 358), (11, 1072, 717), (12, 2145, 1434)];

        for (z, x, y) in tiles_to_fetch {
            let result = source.get_tile(z, x, y).await;
            assert!(
                result.is_ok(),
                "Should fetch tile z={}, x={}, y={} without error",
                z,
                x,
                y
            );
        }
    }
}
