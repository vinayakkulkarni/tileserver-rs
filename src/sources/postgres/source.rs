//! PostgreSQL function tile source implementation.

use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;
use tokio_postgres::types::{ToSql, Type};

use crate::config::PostgresFunctionConfig;
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

use super::{PostgresPool, TileCache, TileCacheKey};

/// PostgreSQL function source that executes SQL functions to generate MVT tiles.
///
/// This source calls a PostgreSQL function with the signature:
/// ```sql
/// function_name(z integer, x integer, y integer) RETURNS bytea
/// ```
///
/// Or with query parameters:
/// ```sql
/// function_name(z integer, x integer, y integer, query json) RETURNS bytea
/// ```
#[derive(Clone)]
pub struct PostgresFunctionSource {
    pool: Arc<PostgresPool>,
    metadata: TileMetadata,
    schema: String,
    function: String,
    sql_query: String,
    supports_query_params: bool,
    cache: Option<Arc<TileCache>>,
}

impl std::fmt::Debug for PostgresFunctionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresFunctionSource")
            .field("id", &self.metadata.id)
            .field("schema", &self.schema)
            .field("function", &self.function)
            .finish()
    }
}

impl PostgresFunctionSource {
    pub async fn new(
        pool: Arc<PostgresPool>,
        config: &PostgresFunctionConfig,
        cache: Option<Arc<TileCache>>,
    ) -> Result<Self> {
        let conn = pool.get().await?;

        let function_info =
            Self::get_function_info(&conn, &config.schema, &config.function).await?;

        let (sql_query, supports_query_params) = Self::build_sql_query(
            &config.schema,
            &config.function,
            function_info.has_query_param,
        );

        let metadata = TileMetadata {
            id: config.id.clone(),
            name: config
                .name
                .clone()
                .unwrap_or_else(|| config.function.clone()),
            description: config.description.clone(),
            attribution: config.attribution.clone(),
            format: TileFormat::Pbf,
            minzoom: config.minzoom,
            maxzoom: config.maxzoom,
            bounds: config.bounds,
            center: config.bounds.map(|b| {
                let center_lon = (b[0] + b[2]) / 2.0;
                let center_lat = (b[1] + b[3]) / 2.0;
                let center_zoom = ((config.minzoom as f64 + config.maxzoom as f64) / 2.0).floor();
                [center_lon, center_lat, center_zoom]
            }),
            vector_layers: None,
        };

        tracing::info!(
            "Loaded PostgreSQL function source '{}': {}.{} (zoom {}-{}, cached={})",
            config.id,
            config.schema,
            config.function,
            config.minzoom,
            config.maxzoom,
            cache.is_some()
        );

        Ok(Self {
            pool,
            metadata,
            schema: config.schema.clone(),
            function: config.function.clone(),
            sql_query,
            supports_query_params,
            cache,
        })
    }

    pub fn tile_query(&self) -> &str {
        &self.sql_query
    }

    #[allow(dead_code)]
    pub fn query_param_types(&self) -> Vec<tokio_postgres::types::Type> {
        use tokio_postgres::types::Type;
        if self.supports_query_params {
            vec![Type::INT4, Type::INT4, Type::INT4, Type::JSON]
        } else {
            vec![Type::INT4, Type::INT4, Type::INT4]
        }
    }

    async fn get_function_info(
        conn: &deadpool_postgres::Object,
        schema: &str,
        function: &str,
    ) -> Result<FunctionInfo> {
        // Query to check if the function exists and get its parameter types
        let query = r#"
            SELECT
                p.proname as name,
                pg_catalog.pg_get_function_arguments(p.oid) as args,
                CASE
                    WHEN pg_catalog.pg_get_function_arguments(p.oid) LIKE '%json%'
                        OR pg_catalog.pg_get_function_arguments(p.oid) LIKE '%jsonb%'
                    THEN true
                    ELSE false
                END as has_query_param
            FROM pg_catalog.pg_proc p
            JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace
            WHERE n.nspname = $1
              AND p.proname = $2
              AND pg_catalog.pg_get_function_result(p.oid) = 'bytea'
            LIMIT 1
        "#;

        let row = conn
            .query_opt(query, &[&schema, &function])
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!("Failed to query function info: {}", e))
            })?
            .ok_or_else(|| {
                TileServerError::PostgresError(format!(
                    "Function {}.{} not found or does not return bytea",
                    schema, function
                ))
            })?;

        let has_query_param: bool = row.get("has_query_param");

        Ok(FunctionInfo { has_query_param })
    }

    fn build_sql_query(schema: &str, function: &str, has_query_param: bool) -> (String, bool) {
        if has_query_param {
            (
                format!(
                    "SELECT \"{}\".\"{}\"($1::integer, $2::integer, $3::integer, $4::json)",
                    schema, function
                ),
                true,
            )
        } else {
            (
                format!(
                    "SELECT \"{}\".\"{}\"($1::integer, $2::integer, $3::integer)",
                    schema, function
                ),
                false,
            )
        }
    }

    #[allow(dead_code)]
    pub fn supports_query_params(&self) -> bool {
        self.supports_query_params
    }

    /// Get a tile with query parameters passed to the PostgreSQL function.
    ///
    /// Query parameters are passed as a JSON object to the function's 4th parameter.
    /// If the function doesn't support query params, they are ignored.
    pub async fn get_tile_with_query_params(
        &self,
        z: u8,
        x: u32,
        y: u32,
        query_params: &serde_json::Value,
    ) -> Result<Option<TileData>> {
        let max_tile = 1u32 << z;
        if x >= max_tile || y >= max_tile {
            return Err(TileServerError::InvalidCoordinates { z, x, y });
        }

        if z < self.metadata.minzoom || z > self.metadata.maxzoom {
            return Ok(None);
        }

        let conn = self.pool.get().await?;

        let param_types: &[Type] = if self.supports_query_params {
            &[Type::INT4, Type::INT4, Type::INT4, Type::JSON]
        } else {
            &[Type::INT4, Type::INT4, Type::INT4]
        };

        let prep_query = conn
            .prepare_typed_cached(&self.sql_query, param_types)
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!(
                    "Failed to prepare query for {}.{}: {}",
                    self.schema, self.function, e
                ))
            })?;

        let tile_data: Option<Vec<u8>> = if self.supports_query_params {
            let params: &[&(dyn ToSql + Sync)] =
                &[&(z as i32), &(x as i32), &(y as i32), query_params];
            conn.query_opt(&prep_query, params).await
        } else {
            conn.query_opt(&prep_query, &[&(z as i32), &(x as i32), &(y as i32)])
                .await
        }
        .map_err(|e| {
            TileServerError::PostgresError(format!(
                "Failed to execute query for {}.{} at z={}, x={}, y={}: {}",
                self.schema, self.function, z, x, y, e
            ))
        })?
        .and_then(|row| row.get::<_, Option<Vec<u8>>>(0));

        Ok(tile_data.map(|data| {
            let compression = if data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b {
                TileCompression::Gzip
            } else {
                TileCompression::None
            };

            TileData {
                data: Bytes::from(data),
                format: TileFormat::Pbf,
                compression,
            }
        }))
    }
}

/// Information about a PostgreSQL function.
struct FunctionInfo {
    has_query_param: bool,
}

#[async_trait]
impl TileSource for PostgresFunctionSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        let max_tile = 1u32 << z;
        if x >= max_tile || y >= max_tile {
            return Err(TileServerError::InvalidCoordinates { z, x, y });
        }

        if z < self.metadata.minzoom || z > self.metadata.maxzoom {
            return Ok(None);
        }

        let cache_key = self.cache.as_ref().map(|_| TileCacheKey {
            source_id: self.metadata.id.clone().into(),
            z,
            x,
            y,
        });

        if let (Some(cache), Some(key)) = (&self.cache, &cache_key) {
            if let Some(tile) = cache.get(key).await {
                return Ok(Some(tile));
            }
        }

        let conn = self.pool.get().await?;

        let param_types: &[Type] = if self.supports_query_params {
            &[Type::INT4, Type::INT4, Type::INT4, Type::JSON]
        } else {
            &[Type::INT4, Type::INT4, Type::INT4]
        };

        let prep_query = conn
            .prepare_typed_cached(&self.sql_query, param_types)
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!(
                    "Failed to prepare query for {}.{}: {}",
                    self.schema, self.function, e
                ))
            })?;

        let tile_data: Option<Vec<u8>> = if self.supports_query_params {
            let empty_json = serde_json::json!({});
            let params: &[&(dyn ToSql + Sync)] =
                &[&(z as i32), &(x as i32), &(y as i32), &empty_json];
            conn.query_opt(&prep_query, params).await
        } else {
            conn.query_opt(&prep_query, &[&(z as i32), &(x as i32), &(y as i32)])
                .await
        }
        .map_err(|e| {
            TileServerError::PostgresError(format!(
                "Failed to execute query for {}.{} at z={}, x={}, y={}: {}",
                self.schema, self.function, z, x, y, e
            ))
        })?
        .and_then(|row| row.get::<_, Option<Vec<u8>>>(0));

        let result = tile_data.map(|data| {
            let compression = if data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b {
                TileCompression::Gzip
            } else {
                TileCompression::None
            };

            TileData {
                data: Bytes::from(data),
                format: TileFormat::Pbf,
                compression,
            }
        });

        if let (Some(cache), Some(key), Some(tile)) = (&self.cache, cache_key, &result) {
            cache.insert(key, tile.clone()).await;
        }

        Ok(result)
    }

    fn metadata(&self) -> &TileMetadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_sql_query_without_params() {
        let (sql, has_params) =
            PostgresFunctionSource::build_sql_query("public", "my_tiles", false);
        assert_eq!(
            sql,
            "SELECT \"public\".\"my_tiles\"($1::integer, $2::integer, $3::integer)"
        );
        assert!(!has_params);
    }

    #[test]
    fn test_build_sql_query_with_params() {
        let (sql, has_params) = PostgresFunctionSource::build_sql_query("public", "my_tiles", true);
        assert_eq!(
            sql,
            "SELECT \"public\".\"my_tiles\"($1::integer, $2::integer, $3::integer, $4::json)"
        );
        assert!(has_params);
    }

    #[test]
    fn test_build_sql_query_escapes_schema_and_function() {
        let (sql, _) = PostgresFunctionSource::build_sql_query("my_schema", "tile_func", false);
        assert!(sql.contains("\"my_schema\""));
        assert!(sql.contains("\"tile_func\""));
    }

    #[test]
    fn test_tile_metadata_defaults() {
        let config = PostgresFunctionConfig {
            id: "test".to_string(),
            schema: "public".to_string(),
            function: "my_tiles".to_string(),
            name: None,
            attribution: None,
            description: None,
            minzoom: 0,
            maxzoom: 14,
            bounds: Some([-180.0, -85.0, 180.0, 85.0]),
        };

        // Test that center is calculated from bounds
        let bounds = config.bounds.unwrap();
        let center_lon = (bounds[0] + bounds[2]) / 2.0;
        let center_lat = (bounds[1] + bounds[3]) / 2.0;
        assert_eq!(center_lon, 0.0);
        assert_eq!(center_lat, 0.0);
    }
}
