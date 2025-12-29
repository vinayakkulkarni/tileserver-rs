//! PostgreSQL table tile source implementation with optimized spatial filtering.

use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;
use tokio_postgres::types::Type;

use crate::config::PostgresTableConfig;
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

use super::{PostgresPool, TileCache, TileCacheKey};

#[derive(Debug, Clone)]
pub struct TableInfo {
    pub schema: String,
    pub table: String,
    pub geometry_column: String,
    pub srid: i32,
    pub geometry_type: String,
    pub id_column: Option<String>,
    pub properties: Vec<String>,
    pub bounds: Option<[f64; 4]>,
    pub has_spatial_index: bool,
}

#[derive(Clone)]
pub struct PostgresTableSource {
    pool: Arc<PostgresPool>,
    metadata: TileMetadata,
    table_info: TableInfo,
    tile_query: String,
    extent: u32,
    buffer: u32,
    max_features: Option<u32>,
    /// Whether ST_TileEnvelope supports margin parameter (PostGIS >= 3.1)
    supports_tile_margin: bool,
    cache: Option<Arc<TileCache>>,
}

impl std::fmt::Debug for PostgresTableSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresTableSource")
            .field("id", &self.metadata.id)
            .field("schema", &self.table_info.schema)
            .field("table", &self.table_info.table)
            .field("geometry_column", &self.table_info.geometry_column)
            .field("srid", &self.table_info.srid)
            .finish()
    }
}

impl PostgresTableSource {
    pub async fn new(
        pool: Arc<PostgresPool>,
        config: &PostgresTableConfig,
        cache: Option<Arc<TileCache>>,
    ) -> Result<Self> {
        let conn = pool.get().await?;
        let supports_tile_margin = pool.supports_tile_margin();

        let table_info = Self::discover_table(&conn, config).await?;

        if !table_info.has_spatial_index {
            tracing::warn!(
                "Table {}.{} has no spatial index on column '{}'. Performance will be degraded.",
                table_info.schema,
                table_info.table,
                table_info.geometry_column
            );
        }

        let tile_query = Self::build_tile_query(&table_info, config, supports_tile_margin);

        let bounds = config.bounds.or(table_info.bounds);
        let metadata = TileMetadata {
            id: config.id.clone(),
            name: config.name.clone().unwrap_or_else(|| config.table.clone()),
            description: config.description.clone(),
            attribution: config.attribution.clone(),
            format: TileFormat::Pbf,
            minzoom: config.minzoom,
            maxzoom: config.maxzoom,
            bounds,
            center: bounds.map(|b| {
                let center_lon = (b[0] + b[2]) / 2.0;
                let center_lat = (b[1] + b[3]) / 2.0;
                let center_zoom = ((config.minzoom as f64 + config.maxzoom as f64) / 2.0).floor();
                [center_lon, center_lat, center_zoom]
            }),
            vector_layers: None,
        };

        tracing::info!(
            "Loaded PostgreSQL table source '{}': {}.{} (srid={}, zoom {}-{}, margin={}, cached={})",
            config.id,
            table_info.schema,
            table_info.table,
            table_info.srid,
            config.minzoom,
            config.maxzoom,
            supports_tile_margin,
            cache.is_some()
        );

        Ok(Self {
            pool,
            metadata,
            table_info,
            tile_query,
            extent: config.extent,
            buffer: config.buffer,
            max_features: config.max_features,
            supports_tile_margin,
            cache,
        })
    }

    async fn discover_table(
        conn: &deadpool_postgres::Object,
        config: &PostgresTableConfig,
    ) -> Result<TableInfo> {
        let geometry_column = if let Some(ref col) = config.geometry_column {
            col.clone()
        } else {
            Self::find_geometry_column(conn, &config.schema, &config.table).await?
        };

        let (srid, geometry_type) =
            Self::get_geometry_info(conn, &config.schema, &config.table, &geometry_column).await?;

        let properties = if let Some(ref props) = config.properties {
            props.clone()
        } else {
            Self::discover_properties(conn, &config.schema, &config.table, &geometry_column).await?
        };

        let has_spatial_index =
            Self::check_spatial_index(conn, &config.schema, &config.table, &geometry_column)
                .await?;

        let bounds = if config.bounds.is_some() {
            config.bounds
        } else {
            Self::estimate_bounds(conn, &config.schema, &config.table, &geometry_column, srid)
                .await
                .ok()
        };

        Ok(TableInfo {
            schema: config.schema.clone(),
            table: config.table.clone(),
            geometry_column,
            srid,
            geometry_type,
            id_column: config.id_column.clone(),
            properties,
            bounds,
            has_spatial_index,
        })
    }

    async fn find_geometry_column(
        conn: &deadpool_postgres::Object,
        schema: &str,
        table: &str,
    ) -> Result<String> {
        let query = r#"
            SELECT f_geometry_column::text
            FROM geometry_columns
            WHERE f_table_schema = $1 AND f_table_name = $2
            LIMIT 1
        "#;

        conn.query_opt(query, &[&schema, &table])
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!("Failed to find geometry column: {}", e))
            })?
            .map(|row| row.get(0))
            .ok_or_else(|| {
                TileServerError::PostgresError(format!(
                    "No geometry column found in {}.{}",
                    schema, table
                ))
            })
    }

    async fn get_geometry_info(
        conn: &deadpool_postgres::Object,
        schema: &str,
        table: &str,
        geometry_column: &str,
    ) -> Result<(i32, String)> {
        let query = r#"
            SELECT srid, type
            FROM geometry_columns
            WHERE f_table_schema = $1 
              AND f_table_name = $2 
              AND f_geometry_column = $3
        "#;

        conn.query_opt(query, &[&schema, &table, &geometry_column])
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!("Failed to get geometry info: {}", e))
            })?
            .map(|row| (row.get::<_, i32>(0), row.get::<_, String>(1)))
            .ok_or_else(|| {
                TileServerError::PostgresError(format!(
                    "Geometry column '{}' not found in {}.{}",
                    geometry_column, schema, table
                ))
            })
    }

    async fn discover_properties(
        conn: &deadpool_postgres::Object,
        schema: &str,
        table: &str,
        geometry_column: &str,
    ) -> Result<Vec<String>> {
        let query = r#"
            SELECT column_name::text
            FROM information_schema.columns
            WHERE table_schema = $1 
              AND table_name = $2
              AND column_name != $3
              AND data_type IN (
                  'integer', 'bigint', 'smallint', 
                  'real', 'double precision', 'numeric',
                  'text', 'character varying', 'character',
                  'boolean', 'json', 'jsonb'
              )
            ORDER BY ordinal_position
        "#;

        let rows = conn
            .query(query, &[&schema, &table, &geometry_column])
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!("Failed to discover properties: {}", e))
            })?;

        Ok(rows.iter().map(|row| row.get(0)).collect())
    }

    async fn check_spatial_index(
        conn: &deadpool_postgres::Object,
        schema: &str,
        table: &str,
        geometry_column: &str,
    ) -> Result<bool> {
        let query = r#"
            SELECT EXISTS (
                SELECT 1
                FROM pg_index i
                JOIN pg_class c ON c.oid = i.indexrelid
                JOIN pg_class t ON t.oid = i.indrelid
                JOIN pg_namespace n ON n.oid = t.relnamespace
                JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(i.indkey)
                JOIN pg_opclass oc ON oc.oid = i.indclass[0]
                WHERE n.nspname = $1
                  AND t.relname = $2
                  AND a.attname = $3
                  AND oc.opcname IN ('gist_geometry_ops_2d', 'gist_geometry_ops_nd', 
                                     'spgist_geometry_ops_2d', 'spgist_geometry_ops_nd',
                                     'brin_geometry_inclusion_ops_2d', 'brin_geometry_inclusion_ops_nd')
            )
        "#;

        conn.query_one(query, &[&schema, &table, &geometry_column])
            .await
            .map(|row| row.get(0))
            .map_err(|e| {
                TileServerError::PostgresError(format!("Failed to check spatial index: {}", e))
            })
    }

    async fn estimate_bounds(
        conn: &deadpool_postgres::Object,
        schema: &str,
        table: &str,
        geometry_column: &str,
        srid: i32,
    ) -> Result<[f64; 4]> {
        let query = if srid == 4326 {
            format!(
                r#"
                SELECT 
                    ST_XMin(ext)::float8, ST_YMin(ext)::float8,
                    ST_XMax(ext)::float8, ST_YMax(ext)::float8
                FROM (
                    SELECT ST_EstimatedExtent('{}', '{}', '{}') AS ext
                ) sub
                "#,
                schema, table, geometry_column
            )
        } else {
            format!(
                r#"
                SELECT 
                    ST_XMin(ext)::float8, ST_YMin(ext)::float8,
                    ST_XMax(ext)::float8, ST_YMax(ext)::float8
                FROM (
                    SELECT ST_Transform(ST_EstimatedExtent('{}', '{}', '{}'), 4326) AS ext
                ) sub
                "#,
                schema, table, geometry_column
            )
        };

        conn.query_one(&query, &[])
            .await
            .map(|row| {
                [
                    row.get::<_, f64>(0),
                    row.get::<_, f64>(1),
                    row.get::<_, f64>(2),
                    row.get::<_, f64>(3),
                ]
            })
            .map_err(|e| {
                TileServerError::PostgresError(format!("Failed to estimate bounds: {}", e))
            })
    }

    fn build_tile_query(
        table_info: &TableInfo,
        config: &PostgresTableConfig,
        supports_tile_margin: bool,
    ) -> String {
        let extent = config.extent;
        let buffer = config.buffer;
        let layer_name = &config.id;

        let id_expr = table_info
            .id_column
            .as_ref()
            .map(|col| format!(r#""{}"::bigint"#, col))
            .unwrap_or_default();

        let props_expr = if table_info.properties.is_empty() {
            String::new()
        } else {
            let props: Vec<String> = table_info
                .properties
                .iter()
                .map(|p| format!(r#""{}""#, p))
                .collect();
            if id_expr.is_empty() {
                props.join(", ")
            } else {
                format!(", {}", props.join(", "))
            }
        };

        let select_cols = if id_expr.is_empty() && props_expr.is_empty() {
            String::new()
        } else if id_expr.is_empty() {
            format!(", {}", props_expr)
        } else {
            format!(", {}{}", id_expr, props_expr)
        };

        let limit_clause = config
            .max_features
            .map(|n| format!(" LIMIT {}", n))
            .unwrap_or_default();

        // Calculate margin as fraction of tile extent for buffer
        // PostGIS 3.1+ margin parameter: fraction of tile size (e.g., 64/4096 = 0.015625)
        let margin = buffer as f64 / extent as f64;

        // ST_TileEnvelope call - with margin if PostGIS >= 3.1
        let tile_envelope = if supports_tile_margin {
            format!("ST_TileEnvelope($1, $2, $3, margin => {})", margin)
        } else {
            "ST_TileEnvelope($1, $2, $3)".to_string()
        };

        // WHERE clause: transform envelope to table SRID for spatial index usage
        let where_clause = if table_info.srid == 3857 {
            format!(r#""{}" && {}"#, table_info.geometry_column, tile_envelope)
        } else {
            format!(
                r#""{}" && ST_Transform({}, {})"#,
                table_info.geometry_column, tile_envelope, table_info.srid
            )
        };

        format!(
            r#"
            SELECT ST_AsMVT(tile, '{}', {}, 'geom') FROM (
                SELECT
                    ST_AsMVTGeom(
                        ST_Transform("{}"::geometry, 3857),
                        {},
                        {},
                        {},
                        true
                    ) AS geom{}
                FROM "{}"."{}"
                WHERE {}{}
            ) AS tile
            WHERE geom IS NOT NULL
            "#,
            layer_name,
            extent,
            table_info.geometry_column,
            tile_envelope,
            extent,
            buffer,
            select_cols,
            table_info.schema,
            table_info.table,
            where_clause,
            limit_clause
        )
    }
}

#[async_trait]
impl TileSource for PostgresTableSource {
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

        let prep_query = conn
            .prepare_typed_cached(&self.tile_query, &[Type::INT4, Type::INT4, Type::INT4])
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!(
                    "Failed to prepare tile query for {}.{}: {}",
                    self.table_info.schema, self.table_info.table, e
                ))
            })?;

        let tile_data: Option<Vec<u8>> = conn
            .query_opt(&prep_query, &[&(z as i32), &(x as i32), &(y as i32)])
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!(
                    "Failed to execute tile query for {}.{} at z={}, x={}, y={}: {}",
                    self.table_info.schema, self.table_info.table, z, x, y, e
                ))
            })?
            .and_then(|row| row.get::<_, Option<Vec<u8>>>(0));

        let result = tile_data.filter(|d| !d.is_empty()).map(|data| {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PostgresTableConfig;

    fn make_table_info() -> TableInfo {
        TableInfo {
            schema: "public".to_string(),
            table: "points".to_string(),
            geometry_column: "geom".to_string(),
            srid: 4326,
            geometry_type: "POINT".to_string(),
            id_column: Some("id".to_string()),
            properties: vec!["name".to_string(), "category".to_string()],
            bounds: Some([8.0, 47.0, 9.0, 48.0]),
            has_spatial_index: true,
        }
    }

    fn make_config() -> PostgresTableConfig {
        PostgresTableConfig {
            id: "test_layer".to_string(),
            schema: "public".to_string(),
            table: "points".to_string(),
            geometry_column: Some("geom".to_string()),
            id_column: Some("id".to_string()),
            properties: Some(vec!["name".to_string(), "category".to_string()]),
            name: None,
            attribution: None,
            description: None,
            minzoom: 0,
            maxzoom: 14,
            bounds: None,
            extent: 4096,
            buffer: 64,
            max_features: None,
        }
    }

    #[test]
    fn test_build_tile_query_srid_4326_no_margin() {
        let table_info = make_table_info();
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, false);

        assert!(query.contains("ST_Transform(ST_TileEnvelope($1, $2, $3), 4326)"));
        assert!(query.contains(r#""geom" && ST_Transform"#));
        assert!(query.contains(r#"ST_AsMVT(tile, 'test_layer', 4096, 'geom')"#));
        assert!(query.contains(r#""id"::bigint"#));
        assert!(query.contains(r#""name""#));
        assert!(query.contains(r#""category""#));
    }

    #[test]
    fn test_build_tile_query_srid_4326_with_margin() {
        let table_info = make_table_info();
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, true);

        assert!(query.contains("margin =>"));
        assert!(query.contains("ST_Transform(ST_TileEnvelope($1, $2, $3, margin =>"));
    }

    #[test]
    fn test_build_tile_query_srid_3857() {
        let mut table_info = make_table_info();
        table_info.srid = 3857;
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, false);

        assert!(query.contains(r#""geom" && ST_TileEnvelope($1, $2, $3)"#));
        assert!(!query.contains("ST_Transform(ST_TileEnvelope"));
    }

    #[test]
    fn test_build_tile_query_with_limit() {
        let table_info = make_table_info();
        let mut config = make_config();
        config.max_features = Some(1000);
        let query = PostgresTableSource::build_tile_query(&table_info, &config, false);

        assert!(query.contains("LIMIT 1000"));
    }

    #[test]
    fn test_build_tile_query_no_properties() {
        let mut table_info = make_table_info();
        table_info.properties = vec![];
        table_info.id_column = None;
        let mut config = make_config();
        config.properties = Some(vec![]);
        config.id_column = None;
        let query = PostgresTableSource::build_tile_query(&table_info, &config, false);

        assert!(!query.contains(r#""name""#));
        assert!(!query.contains(r#""id"::bigint"#));
    }
}
