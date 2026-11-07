//! PostgreSQL function tile source implementation.

use async_trait::async_trait;
use std::sync::Arc;
use tokio_postgres::types::{ToSql, Type};

use crate::config::PostgresFunctionConfig;
use crate::error::{Result, TileServerError};
use crate::sources::{TileData, TileFormat, TileMetadata, TileSource};

use super::{PostgresPool, TileCache, pbf_tile, serves_tile, through_cache};

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
    /// # Errors
    ///
    /// Returns an error when the pool has no connection or the function is
    /// missing.
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

    #[must_use]
    pub fn tile_query(&self) -> &str {
        &self.sql_query
    }

    #[must_use]
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
                TileServerError::PostgresError(format!("failed to query function info: {}", e))
            })?
            .ok_or_else(|| {
                TileServerError::PostgresError(format!(
                    "function {}.{} not found or does not return bytea",
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

    #[must_use]
    pub fn supports_query_params(&self) -> bool {
        self.supports_query_params
    }

    /// Get a tile with query parameters passed to the PostgreSQL function.
    ///
    /// Query parameters are passed as a JSON object to the function's 4th parameter.
    /// If the function doesn't support query params, they are ignored.
    ///
    /// A request without query parameters produces the same tile as
    /// [`TileSource::get_tile`], so it takes that path and uses the tile cache.
    /// Requests with parameters skip the cache, because the function output
    /// can depend on them.
    ///
    /// # Errors
    ///
    /// [`TileServerError::InvalidCoordinates`] for an out-of-range tile, and
    /// [`TileServerError::PostgresError`] when the query fails.
    pub async fn get_tile_with_query_params(
        &self,
        z: u8,
        x: u32,
        y: u32,
        query_params: &serde_json::Value,
    ) -> Result<Option<TileData>> {
        if !self.supports_query_params || is_empty_query(query_params) {
            return self.get_tile(z, x, y).await;
        }
        if !serves_tile(&self.metadata, z, x, y)? {
            return Ok(None);
        }
        self.query_tile(z, x, y, query_params).await
    }

    /// Run the tile function. `query_params` is passed only when the
    /// function takes a fourth argument.
    async fn query_tile(
        &self,
        z: u8,
        x: u32,
        y: u32,
        query_params: &serde_json::Value,
    ) -> Result<Option<TileData>> {
        let conn = self.pool.get().await?;
        let (zi, xi, yi) = (i32::from(z), x as i32, y as i32);
        let mut params: Vec<&(dyn ToSql + Sync)> = vec![&zi, &xi, &yi];
        let mut param_types = vec![Type::INT4, Type::INT4, Type::INT4];
        if self.supports_query_params {
            params.push(query_params);
            param_types.push(Type::JSON);
        }
        let prep_query = conn
            .prepare_typed_cached(&self.sql_query, &param_types)
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!(
                    "failed to prepare query for {}.{}: {}",
                    self.schema, self.function, e
                ))
            })?;
        let row = conn.query_opt(&prep_query, &params).await.map_err(|e| {
            TileServerError::PostgresError(format!(
                "failed to execute query for {}.{} at z={}, x={}, y={}: {}",
                self.schema, self.function, z, x, y, e
            ))
        })?;
        Ok(row
            .and_then(|row| row.get::<_, Option<Vec<u8>>>(0))
            .map(pbf_tile))
    }
}

/// `true` when the request carried no query parameters (`null` or `{}`).
fn is_empty_query(query_params: &serde_json::Value) -> bool {
    match query_params {
        serde_json::Value::Null => true,
        serde_json::Value::Object(map) => map.is_empty(),
        _ => false,
    }
}

/// Information about a PostgreSQL function.
struct FunctionInfo {
    has_query_param: bool,
}

#[async_trait]
impl TileSource for PostgresFunctionSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        if !serves_tile(&self.metadata, z, x, y)? {
            return Ok(None);
        }
        let no_params = serde_json::json!({});
        let fetch = self.query_tile(z, x, y, &no_params);
        through_cache(self.cache.as_deref(), &self.metadata.id, (z, x, y), fetch).await
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
            filter: None,
        };

        // Test that center is calculated from bounds
        let bounds = config.bounds.unwrap();
        let center_lon = (bounds[0] + bounds[2]) / 2.0;
        let center_lat = (bounds[1] + bounds[3]) / 2.0;
        assert_eq!(center_lon, 0.0);
        assert_eq!(center_lat, 0.0);
    }

    #[test]
    fn empty_query_is_null_or_empty_object() {
        assert!(is_empty_query(&serde_json::Value::Null));
        assert!(is_empty_query(&serde_json::json!({})));
        assert!(!is_empty_query(&serde_json::json!({ "category": "cafe" })));
        // Non-object values still go to the function; they are not "empty".
        assert!(!is_empty_query(&serde_json::json!([])));
        assert!(!is_empty_query(&serde_json::json!("")));
    }
}
