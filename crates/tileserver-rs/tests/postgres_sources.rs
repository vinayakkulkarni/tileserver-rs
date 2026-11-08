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
            filter: None,
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
            filter: None,
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
            filter: None,
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
            filter: None,
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
            filter: None,
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
            filter: None,
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
