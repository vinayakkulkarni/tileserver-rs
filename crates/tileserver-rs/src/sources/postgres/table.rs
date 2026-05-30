//! PostgreSQL table tile source implementation with optimized spatial filtering.

use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;
use tokio_postgres::types::Type;

use crate::config::PostgresTableConfig;
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

use super::{PostgresPool, TileCache, TileCacheKey};

/// Encodes a `serde_json::Value` from the client payload into a `(sql, param)`
/// pair suitable for splicing into a parameterised `INSERT` / `UPDATE`.
///
/// PostgreSQL cannot implicitly cast `text` (the default `tokio-postgres`
/// bind type for strings) to `bigint`/`bool`/`double precision`, so naïvely
/// binding the JSON literal fails on non-text columns. We sidestep the
/// cast issue by bind-shape — the Rust side passes the strongest matching
/// `tokio_postgres::types::ToSql` implementation per JSON kind:
/// - `Value::Bool`   -> `bool` (pg: `boolean`)
/// - `Value::Number` -> text bound then cast `::numeric` in SQL so the
///   receiving column type (bigint/int/real/double/numeric) decides the
///   final cast — avoids the text-to-bigint and double-to-bigint errors.
/// - everything else -> text/jsonb so arrays and objects also round-trip
///
/// The returned SQL fragment is the placeholder string
/// (`$N` or `$N::text::jsonb`) to substitute into the statement.
fn encode_property_param(
    value: &serde_json::Value,
    idx: u32,
) -> (String, Box<dyn tokio_postgres::types::ToSql + Sync + Send>) {
    match value {
        serde_json::Value::Null => ("NULL".to_string(), Box::new(Option::<String>::None)),
        serde_json::Value::Bool(b) => (format!("${idx}"), Box::new(*b)),
        serde_json::Value::Number(n) => (format!("${idx}::text::numeric"), Box::new(n.to_string())),
        serde_json::Value::String(s) => (format!("${idx}"), Box::new(s.clone())),
        other => (format!("${idx}::jsonb"), Box::new(other.to_string())),
    }
}

/// Reports whether a PostGIS `geometry_columns.type` string describes a
/// point geometry. Accepts dimension suffixes emitted by PostGIS
/// (`POINT Z`, `POINT ZM`, `MULTIPOINT Z`, etc.) and is case-insensitive
/// to survive catalogue quirks across PostGIS versions.
///
/// Used by `build_tile_query` to choose an unbuffered vs. buffered WHERE
/// envelope — see the comment above that site for the rationale.
#[must_use]
pub fn is_point_geometry(geometry_type: &str) -> bool {
    let upper = geometry_type.trim().to_ascii_uppercase();
    upper.starts_with("POINT") || upper.starts_with("MULTIPOINT")
}

/// Maps a PostgreSQL `data_type` (as reported by `information_schema`) to a
/// JSON Schema type object + a boolean indicating whether the column is
/// safely sortable in SQL `ORDER BY` (arrays/jsonb/records are not).
#[must_use]
pub fn pg_type_to_json_schema(data_type: &str) -> (serde_json::Value, bool) {
    let lower = data_type.to_ascii_lowercase();
    match lower.as_str() {
        "integer" | "bigint" | "smallint" => (serde_json::json!({"type": "integer"}), true),
        "real" | "double precision" | "numeric" | "decimal" => {
            (serde_json::json!({"type": "number"}), true)
        }
        "boolean" => (serde_json::json!({"type": "boolean"}), true),
        "text" | "character varying" | "character" | "citext" => {
            (serde_json::json!({"type": "string"}), true)
        }
        "uuid" => (
            serde_json::json!({"type": "string", "format": "uuid"}),
            true,
        ),
        "date" => (
            serde_json::json!({"type": "string", "format": "date"}),
            true,
        ),
        "timestamp with time zone" | "timestamp without time zone" => (
            serde_json::json!({"type": "string", "format": "date-time"}),
            true,
        ),
        "time with time zone" | "time without time zone" => (
            serde_json::json!({"type": "string", "format": "time"}),
            true,
        ),
        "json" | "jsonb" => (serde_json::json!({"type": "object"}), false),
        "array" | "record" => (serde_json::json!({"type": "array"}), false),
        _ => (serde_json::json!({"type": "string"}), false),
    }
}

/// Extracts a single property column from a PostgreSQL row as a JSON value.
///
/// Shared between `query_features_geojson` (collection listing) and
/// `query_single_feature_geojson` (single feature) so both paths coerce
/// column types identically. The type-probe order prefers structured JSON
/// first (preserves `jsonb`), then integer/float/bool/string scalars.
/// Columns that don't match any supported type deserialize to `null`.
fn extract_property(row: &tokio_postgres::Row, column: &str) -> serde_json::Value {
    if let Ok(val) = row.try_get::<_, Option<serde_json::Value>>(column) {
        return val.unwrap_or(serde_json::Value::Null);
    }
    if let Ok(val) = row.try_get::<_, Option<i32>>(column) {
        return val.map_or(serde_json::Value::Null, |v| {
            serde_json::Value::Number(v.into())
        });
    }
    if let Ok(val) = row.try_get::<_, Option<i64>>(column) {
        return val.map_or(serde_json::Value::Null, |v| {
            serde_json::Value::Number(v.into())
        });
    }
    if let Ok(val) = row.try_get::<_, Option<f64>>(column) {
        return val.map_or(serde_json::Value::Null, |v| serde_json::json!(v));
    }
    if let Ok(val) = row.try_get::<_, Option<bool>>(column) {
        return val.map_or(serde_json::Value::Null, serde_json::Value::Bool);
    }
    if let Ok(val) = row.try_get::<_, Option<String>>(column) {
        return val.map_or(serde_json::Value::Null, serde_json::Value::String);
    }
    serde_json::Value::Null
}

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
    /// Whether this table opts in to OGC API Features Part 4 transactions.
    pub writable: bool,
}

#[derive(Clone)]
pub struct PostgresTableSource {
    pool: Arc<PostgresPool>,
    metadata: TileMetadata,
    table_info: TableInfo,
    tile_query: String,
    #[allow(dead_code)]
    // Config value baked into tile_query SQL; kept for future tile-query regeneration
    extent: u32,
    #[allow(dead_code)]
    // Config value baked into tile_query SQL; kept for future tile-query regeneration
    buffer: u32,
    #[allow(dead_code)] // Config value baked into tile_query SQL; kept for future LIMIT injection
    max_features: Option<u32>,
    #[allow(dead_code)] // Detected at pool init; reserved for margin-aware ST_TileEnvelope queries
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

        if table_info.id_column.is_none() {
            tracing::warn!(
                table = %format!("{}.{}", table_info.schema, table_info.table),
                "No id_column configured; OGC API Features will fall back to \
                 PostgreSQL ctid as the feature identifier. ctid values are \
                 not stable across VACUUM FULL / CLUSTER and should not be \
                 used for bookmarkable /items/{{fid}} URLs."
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

    #[must_use]
    pub fn tile_query(&self) -> &str {
        &self.tile_query
    }

    #[must_use]
    pub fn table_info(&self) -> &TableInfo {
        &self.table_info
    }

    #[must_use]
    pub fn pool(&self) -> &Arc<PostgresPool> {
        &self.pool
    }

    /// Fetches a single feature by its OGC feature id.
    ///
    /// `output_srid` is the EPSG code the caller wants the geometry emitted
    /// in (OGC API Features Part 2 `crs` query parameter). Passing the
    /// storage SRID avoids the `ST_Transform` round-trip; passing any other
    /// SRID emits `ST_Transform("geom", $output_srid)` in the SELECT.
    ///
    /// Delegates to the same `ST_AsGeoJSON` SQL pipeline as
    /// [`Self::query_features_geojson`] so the two handlers never drift. The
    /// returned value is `None` when no row matches the id, mapped by the
    /// caller to `404 Not Found`.
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::PostgresError`] if the pool is exhausted
    /// or the SQL query fails to execute.
    pub async fn query_single_feature_geojson(
        &self,
        feature_id: &str,
        output_srid: i32,
    ) -> Result<
        Option<(
            String,
            serde_json::Value,
            serde_json::Map<String, serde_json::Value>,
        )>,
    > {
        let conn = self.pool.get().await?;
        let info = &self.table_info;

        let id_col = info.id_column.as_deref().unwrap_or("ctid");

        let geom_expr = if info.srid == output_srid {
            format!(
                r#"ST_AsGeoJSON("{}")::jsonb AS __ogc_geom"#,
                info.geometry_column
            )
        } else {
            format!(
                r#"ST_AsGeoJSON(ST_Transform("{}", {output_srid}))::jsonb AS __ogc_geom"#,
                info.geometry_column
            )
        };

        let prop_cols: Vec<String> = info
            .properties
            .iter()
            .map(|p| format!(r#""{p}""#))
            .collect();
        let prop_select = if prop_cols.is_empty() {
            String::new()
        } else {
            format!(", {}", prop_cols.join(", "))
        };

        let sql = format!(
            r#"SELECT {geom_expr}{prop_select} FROM "{}"."{}" WHERE "{}"::text = $1 LIMIT 1"#,
            info.schema, info.table, id_col
        );

        let row = conn
            .query_opt(&sql, &[&feature_id])
            .await
            .map_err(|e| TileServerError::PostgresError(format!("feature query failed: {e}")))?;

        let Some(row) = row else { return Ok(None) };

        let geom: serde_json::Value = row.get("__ogc_geom");
        let mut properties = serde_json::Map::new();
        for prop in &info.properties {
            properties.insert(prop.clone(), extract_property(&row, prop));
        }

        Ok(Some((feature_id.to_string(), geom, properties)))
    }

    /// Queries features as GeoJSON with OGC API Features Part 2/3 support.
    ///
    /// - `bbox_srid` — EPSG code of the caller-supplied bbox (spec `bbox-crs`).
    ///   The envelope is reprojected to the storage SRID inside PostGIS so
    ///   spatial-index lookups stay cheap.
    /// - `output_srid` — EPSG code the caller wants back (spec `crs`). The
    ///   identity transform is elided when it matches the storage SRID.
    /// - `filter_sql` — pre-validated PostgreSQL `WHERE` fragment produced by
    ///   [`crate::routes::ogc_filter::translate_filter_to_sql`]. Spliced in as
    ///   raw SQL; upstream safety relies on a **property allow-list** (bare
    ///   identifiers are only emitted for columns in `info.properties`) plus
    ///   a round-trip canonicalisation gate that rejects trailing tokens.
    ///   Literals are wrapped as standard CQL2 quoted strings.
    /// - `filter_srid` — EPSG code of geometry literals inside the filter
    ///   (spec `filter-crs`). Currently informational; the CQL2 crate emits
    ///   `ST_GeomFromText` / `ST_GeomFromGeoJSON` without SRID metadata, so
    ///   callers that use a non-storage filter CRS must set the SRID on the
    ///   geometry literal themselves (or use `bbox` as the spatial filter).
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::PostgresError`] when the pool or SQL
    /// execution fails.
    #[allow(clippy::too_many_arguments)]
    pub async fn query_features_geojson(
        &self,
        bbox: Option<[f64; 4]>,
        bbox_srid: i32,
        output_srid: i32,
        filter_sql: Option<&str>,
        filter_srid: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<serde_json::Value>, i64)> {
        let _ = filter_srid;
        let conn = self.pool.get().await?;
        let info = &self.table_info;

        let id_expr = info
            .id_column
            .as_ref()
            .map(|col| format!(r#""{col}"::text AS __ogc_fid"#))
            .unwrap_or_else(|| "ctid::text AS __ogc_fid".to_string());

        let geom_expr = if info.srid == output_srid {
            format!(
                r#"ST_AsGeoJSON("{}")::jsonb AS __ogc_geom"#,
                info.geometry_column
            )
        } else {
            format!(
                r#"ST_AsGeoJSON(ST_Transform("{}", {output_srid}))::jsonb AS __ogc_geom"#,
                info.geometry_column
            )
        };

        let prop_cols: Vec<String> = info
            .properties
            .iter()
            .map(|p| format!(r#""{p}""#))
            .collect();
        let prop_select = if prop_cols.is_empty() {
            String::new()
        } else {
            format!(", {}", prop_cols.join(", "))
        };

        let mut where_clauses = Vec::new();
        let mut param_idx = 1u32;
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();

        if let Some(bb) = bbox {
            let make_envelope = format!(
                "ST_MakeEnvelope(${}, ${}, ${}, ${}, {bbox_srid})",
                param_idx,
                param_idx + 1,
                param_idx + 2,
                param_idx + 3,
            );
            let envelope = if info.srid == bbox_srid {
                make_envelope
            } else {
                format!("ST_Transform({make_envelope}, {})", info.srid)
            };
            where_clauses.push(format!(r#""{}" && {}"#, info.geometry_column, envelope));
            params.push(Box::new(bb[0]));
            params.push(Box::new(bb[1]));
            params.push(Box::new(bb[2]));
            params.push(Box::new(bb[3]));
            param_idx += 4;
        }

        if let Some(fragment) = filter_sql {
            let trimmed = fragment.trim();
            if !trimmed.is_empty() {
                where_clauses.push(format!("({trimmed})"));
            }
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let count_sql = format!(
            r#"SELECT COUNT(*)::bigint FROM "{}"."{}" {}"#,
            info.schema, info.table, where_sql
        );

        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let count_row = conn
            .query_one(&count_sql, &param_refs)
            .await
            .map_err(|e| TileServerError::PostgresError(format!("count query failed: {e}")))?;
        let total_count: i64 = count_row.get(0);

        let data_sql = format!(
            r#"SELECT {id_expr}, {geom_expr}{prop_select} FROM "{}"."{}" {where_sql} LIMIT ${param_idx} OFFSET ${}"#,
            info.schema,
            info.table,
            param_idx + 1
        );
        params.push(Box::new(limit));
        params.push(Box::new(offset));

        let param_refs2: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let rows = conn
            .query(&data_sql, &param_refs2)
            .await
            .map_err(|e| TileServerError::PostgresError(format!("features query failed: {e}")))?;

        let features: Vec<serde_json::Value> = rows
            .iter()
            .map(|row| {
                let fid: String = row.get("__ogc_fid");
                let geom: serde_json::Value = row.get("__ogc_geom");

                let mut properties = serde_json::Map::new();
                for prop in &info.properties {
                    properties.insert(prop.clone(), extract_property(row, prop));
                }

                serde_json::json!({
                    "type": "Feature",
                    "id": fid,
                    "geometry": geom,
                    "properties": properties
                })
            })
            .collect();

        Ok((features, total_count))
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
            writable: config.writable,
        })
    }

    /// Inserts a new feature into the underlying PostGIS table.
    ///
    /// Uses parameterised `INSERT` — geometry is bound via
    /// `ST_GeomFromGeoJSON($N)::geometry(SRID)` so user-supplied GeoJSON is
    /// parsed by PostGIS and cannot leak into surrounding SQL. Properties are
    /// bound as `jsonb` values and cast by PostgreSQL to the destination
    /// column types. Returns the new feature id as text (or the ctid if the
    /// table has no configured id column).
    ///
    /// # Errors
    ///
    /// - [`TileServerError::MethodNotAllowed`] if the table is not marked
    ///   `writable = true` in the config.
    /// - [`TileServerError::InvalidTileRequest`] if the feature is missing a
    ///   geometry or properties shape.
    /// - [`TileServerError::PostgresError`] if the INSERT fails (bad CRS,
    ///   constraint violation, etc.).
    pub async fn insert_feature(
        &self,
        geometry: &serde_json::Value,
        properties: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<String> {
        let info = &self.table_info;
        if !info.writable {
            return Err(TileServerError::MethodNotAllowed(format!(
                "collection '{}.{}' is read-only; set writable = true in config to enable transactions",
                info.schema, info.table
            )));
        }

        let geom_json = geometry.to_string();

        let mut columns: Vec<String> = Vec::with_capacity(info.properties.len() + 1);
        let mut placeholders: Vec<String> = Vec::with_capacity(info.properties.len() + 1);
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> =
            Vec::with_capacity(info.properties.len() + 1);

        columns.push(format!(r#""{}""#, info.geometry_column));
        placeholders.push(format!("ST_SetSRID(ST_GeomFromGeoJSON($1), {})", info.srid));
        params.push(Box::new(geom_json));

        let mut idx: u32 = 2;
        for prop in &info.properties {
            if let Some(value) = properties.get(prop) {
                columns.push(format!(r#""{prop}""#));
                let (sql_literal, owned_param) = encode_property_param(value, idx);
                placeholders.push(sql_literal);
                params.push(owned_param);
                idx += 1;
            }
        }

        let id_col = info.id_column.as_deref().unwrap_or("ctid");
        let sql = format!(
            r#"INSERT INTO "{}"."{}" ({}) VALUES ({}) RETURNING "{}"::text"#,
            info.schema,
            info.table,
            columns.join(", "),
            placeholders.join(", "),
            id_col
        );

        let conn = self.pool.get().await?;
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();
        let row = conn
            .query_one(&sql, &param_refs)
            .await
            .map_err(|e| TileServerError::PostgresError(format!("insert_feature failed: {e}")))?;
        Ok(row.get::<_, String>(0))
    }

    /// Replaces a feature (PUT semantics): geometry + all configured property
    /// columns are overwritten. Missing properties in the payload are set to
    /// NULL — this is the `api-parse-dont-validate` contract of PUT.
    ///
    /// # Errors
    ///
    /// See [`Self::insert_feature`]. Additionally returns
    /// [`TileServerError::NotFound`] if no row matches `feature_id`.
    pub async fn replace_feature(
        &self,
        feature_id: &str,
        geometry: &serde_json::Value,
        properties: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<()> {
        self.write_feature(
            feature_id,
            Some(geometry),
            properties,
            /* partial */ false,
        )
        .await
    }

    /// Updates a feature in place (PATCH semantics, RFC 7396 merge): only
    /// properties/geometry present in the payload are touched; everything
    /// else is preserved.
    ///
    /// # Errors
    ///
    /// See [`Self::insert_feature`]. Returns [`TileServerError::NotFound`]
    /// if no row matches `feature_id`.
    pub async fn patch_feature(
        &self,
        feature_id: &str,
        geometry: Option<&serde_json::Value>,
        properties: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<()> {
        self.write_feature(feature_id, geometry, properties, /* partial */ true)
            .await
    }

    async fn write_feature(
        &self,
        feature_id: &str,
        geometry: Option<&serde_json::Value>,
        properties: &serde_json::Map<String, serde_json::Value>,
        partial: bool,
    ) -> Result<()> {
        let info = &self.table_info;
        if !info.writable {
            return Err(TileServerError::MethodNotAllowed(format!(
                "collection '{}.{}' is read-only",
                info.schema, info.table
            )));
        }

        let mut assignments: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();
        let mut idx: u32 = 1;

        if let Some(geom) = geometry {
            assignments.push(format!(
                r#""{}" = ST_SetSRID(ST_GeomFromGeoJSON(${idx}), {})"#,
                info.geometry_column, info.srid
            ));
            params.push(Box::new(geom.to_string()));
            idx += 1;
        }

        for prop in &info.properties {
            match properties.get(prop) {
                Some(value) => {
                    let (sql_literal, owned_param) = encode_property_param(value, idx);
                    assignments.push(format!(r#""{prop}" = {sql_literal}"#));
                    params.push(owned_param);
                    idx += 1;
                }
                None if !partial => {
                    assignments.push(format!(r#""{prop}" = NULL"#));
                }
                None => {}
            }
        }

        if assignments.is_empty() {
            return Err(TileServerError::InvalidTileRequest);
        }

        let id_col = info.id_column.as_deref().unwrap_or("ctid");
        let sql = format!(
            r#"UPDATE "{}"."{}" SET {} WHERE "{}"::text = ${idx}"#,
            info.schema,
            info.table,
            assignments.join(", "),
            id_col
        );
        params.push(Box::new(feature_id.to_string()));

        let conn = self.pool.get().await?;
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let rows = conn
            .execute(&sql, &param_refs)
            .await
            .map_err(|e| TileServerError::PostgresError(format!("write_feature failed: {e}")))?;
        if rows == 0 {
            return Err(TileServerError::NotFound(format!(
                "feature '{feature_id}' not found"
            )));
        }
        Ok(())
    }

    /// Returns the JSON-Schema descriptor for every non-geometry column on
    /// the table, used by the OGC Features Part 5 `/queryables`, `/sortables`
    /// and `/schema` endpoints.
    ///
    /// PostgreSQL types are mapped to JSON Schema primitives per the table
    /// below. Arrays/records that don't fit are reported as `{"type":"string"}`
    /// so QGIS still displays the column.
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::PostgresError`] when the
    /// `information_schema.columns` introspection query fails.
    pub async fn column_schemas(&self) -> Result<Vec<(String, serde_json::Value, bool)>> {
        let info = &self.table_info;
        let conn = self.pool.get().await?;
        let query = r#"
            SELECT column_name::text, data_type::text, is_nullable::text
            FROM information_schema.columns
            WHERE table_schema = $1 AND table_name = $2 AND column_name != $3
            ORDER BY ordinal_position
        "#;
        let rows = conn
            .query(query, &[&info.schema, &info.table, &info.geometry_column])
            .await
            .map_err(|e| TileServerError::PostgresError(format!("column_schemas failed: {e}")))?;

        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let name: String = row.get(0);
            let data_type: String = row.get(1);
            let nullable: String = row.get(2);
            let (schema, sortable) = pg_type_to_json_schema(&data_type);
            let mut schema = schema;
            if let Some(obj) = schema.as_object_mut() {
                obj.insert("title".to_string(), serde_json::Value::String(name.clone()));
                if nullable == "YES"
                    && let Some(serde_json::Value::String(t)) = obj.get("type").cloned()
                {
                    obj.insert("type".to_string(), serde_json::json!([t, "null"]));
                }
            }
            result.push((name, schema, sortable));
        }
        Ok(result)
    }

    /// Deletes a feature by id.
    ///
    /// # Errors
    ///
    /// See [`Self::insert_feature`]. Returns [`TileServerError::NotFound`]
    /// if no row matches.
    pub async fn delete_feature(&self, feature_id: &str) -> Result<()> {
        let info = &self.table_info;
        if !info.writable {
            return Err(TileServerError::MethodNotAllowed(format!(
                "collection '{}.{}' is read-only",
                info.schema, info.table
            )));
        }
        let id_col = info.id_column.as_deref().unwrap_or("ctid");
        let sql = format!(
            r#"DELETE FROM "{}"."{}" WHERE "{}"::text = $1"#,
            info.schema, info.table, id_col
        );
        let conn = self.pool.get().await?;
        let rows = conn
            .execute(&sql, &[&feature_id])
            .await
            .map_err(|e| TileServerError::PostgresError(format!("delete_feature failed: {e}")))?;
        if rows == 0 {
            return Err(TileServerError::NotFound(format!(
                "feature '{feature_id}' not found"
            )));
        }
        Ok(())
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

        // Buffered envelope — used inside ST_AsMVTGeom so line/polygon geometry
        // near tile edges is fetched and clipped to the buffered region,
        // producing seamless rendering across tile boundaries.
        let buffered_envelope = if supports_tile_margin {
            format!("ST_TileEnvelope($1, $2, $3, margin => {})", margin)
        } else {
            "ST_TileEnvelope($1, $2, $3)".to_string()
        };

        // WHERE envelope — decides which rows are *fetched* from the table.
        // For POINT/MULTIPOINT we use the UNBUFFERED tile envelope so that a
        // point near a tile boundary is returned in exactly one tile. With the
        // buffered envelope, ST_AsMVTGeom would keep the point (no clipping
        // happens for points) and the client would render duplicate markers
        // and labels on both sides of the seam.
        //
        // For lines/polygons/generic `GEOMETRY` we keep the buffered envelope
        // because ST_AsMVTGeom *does* clip those types, so overlap is invisible
        // but missing geometry at tile edges would produce visible seams.
        let use_unbuffered_where = is_point_geometry(&table_info.geometry_type);
        let where_envelope = if use_unbuffered_where {
            "ST_TileEnvelope($1, $2, $3)".to_string()
        } else {
            buffered_envelope.clone()
        };

        // WHERE clause: transform envelope to table SRID for spatial index usage
        let where_clause = if table_info.srid == 3857 {
            format!(r#""{}" && {}"#, table_info.geometry_column, where_envelope)
        } else {
            format!(
                r#""{}" && ST_Transform({}, {})"#,
                table_info.geometry_column, where_envelope, table_info.srid
            )
        };
        let tile_envelope = buffered_envelope;

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
            encoding: TileCompression::None,
        });

        if let (Some(cache), Some(key)) = (&self.cache, &cache_key)
            && let Some(tile) = cache.get(key).await
        {
            return Ok(Some(tile));
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
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
            writable: false,
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
            writable: false,
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
        let mut table_info = make_table_info();
        table_info.geometry_type = "MULTIPOLYGON".to_string();
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

    #[test]
    fn test_build_tile_query_no_id_with_properties() {
        let mut table_info = make_table_info();
        table_info.id_column = None;
        let mut config = make_config();
        config.id_column = None;
        let query = PostgresTableSource::build_tile_query(&table_info, &config, false);

        assert!(query.contains(r#""name""#));
        assert!(query.contains(r#""category""#));
        assert!(!query.contains(r#"::bigint"#));
    }

    #[test]
    fn test_build_tile_query_srid_4326_with_margin_contains_extent() {
        let table_info = make_table_info();
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, true);

        assert!(query.contains("4096"));
        assert!(query.contains("64"));
        assert!(query.contains("ST_AsMVTGeom"));
    }

    #[test]
    fn test_build_tile_query_custom_extent_and_buffer() {
        let table_info = make_table_info();
        let mut config = make_config();
        config.extent = 512;
        config.buffer = 32;
        let query = PostgresTableSource::build_tile_query(&table_info, &config, false);

        assert!(query.contains("ST_AsMVT(tile, 'test_layer', 512, 'geom')"));
        assert!(query.contains("512"));
    }

    #[test]
    fn test_build_tile_query_srid_2056() {
        let mut table_info = make_table_info();
        table_info.srid = 2056;
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, false);

        assert!(query.contains("ST_Transform(ST_TileEnvelope($1, $2, $3), 2056)"));
    }

    #[test]
    fn test_build_tile_query_3857_with_margin() {
        let mut table_info = make_table_info();
        table_info.srid = 3857;
        table_info.geometry_type = "MULTIPOLYGON".to_string();
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, true);

        assert!(query.contains("margin =>"));
        assert!(query.contains(r#""geom" && ST_TileEnvelope($1, $2, $3, margin =>"#));
    }

    #[test]
    fn test_build_tile_query_point_uses_unbuffered_where() {
        let mut table_info = make_table_info();
        table_info.srid = 3857;
        table_info.geometry_type = "POINT".to_string();
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, true);

        assert!(
            query.contains(r#""geom" && ST_TileEnvelope($1, $2, $3)"#),
            "POINT WHERE must use unbuffered envelope to avoid duplicates; got:\n{query}"
        );
        assert!(
            !query.contains(r#""geom" && ST_TileEnvelope($1, $2, $3, margin =>"#),
            "POINT WHERE must not use margin; got:\n{query}"
        );
        assert!(
            query.contains("ST_AsMVTGeom(") && query.contains("margin =>"),
            "ST_AsMVTGeom must still use buffered envelope; got:\n{query}"
        );
    }

    #[test]
    fn test_build_tile_query_multipoint_uses_unbuffered_where() {
        let mut table_info = make_table_info();
        table_info.srid = 3857;
        table_info.geometry_type = "MULTIPOINT".to_string();
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, true);

        assert!(query.contains(r#""geom" && ST_TileEnvelope($1, $2, $3)"#));
        assert!(!query.contains(r#""geom" && ST_TileEnvelope($1, $2, $3, margin =>"#));
    }

    #[test]
    fn test_build_tile_query_point_z_suffix_treated_as_point() {
        let mut table_info = make_table_info();
        table_info.srid = 3857;
        table_info.geometry_type = "POINT Z".to_string();
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, true);

        assert!(query.contains(r#""geom" && ST_TileEnvelope($1, $2, $3)"#));
    }

    #[test]
    fn test_build_tile_query_polygon_uses_buffered_where() {
        let mut table_info = make_table_info();
        table_info.srid = 3857;
        table_info.geometry_type = "POLYGON".to_string();
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, true);

        assert!(query.contains(r#""geom" && ST_TileEnvelope($1, $2, $3, margin =>"#));
    }

    #[test]
    fn test_build_tile_query_generic_geometry_defaults_to_buffered() {
        let mut table_info = make_table_info();
        table_info.srid = 3857;
        table_info.geometry_type = "GEOMETRY".to_string();
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, true);

        assert!(
            query.contains(r#""geom" && ST_TileEnvelope($1, $2, $3, margin =>"#),
            "Generic GEOMETRY must default to buffered WHERE to avoid seams in mixed layers; got:\n{query}"
        );
    }

    #[test]
    fn test_is_point_geometry_accepts_suffixed_and_mixed_case() {
        assert!(is_point_geometry("POINT"));
        assert!(is_point_geometry("point"));
        assert!(is_point_geometry("POINT Z"));
        assert!(is_point_geometry("POINT ZM"));
        assert!(is_point_geometry("MULTIPOINT"));
        assert!(is_point_geometry("multipoint z"));
        assert!(!is_point_geometry("LINESTRING"));
        assert!(!is_point_geometry("POLYGON"));
        assert!(!is_point_geometry("MULTIPOLYGON"));
        assert!(!is_point_geometry("GEOMETRY"));
        assert!(!is_point_geometry("GEOMETRYCOLLECTION"));
    }

    #[test]
    fn test_table_info_geometry_type_stored() {
        let info = make_table_info();
        assert_eq!(info.geometry_type, "POINT");
    }

    #[test]
    fn test_table_info_has_spatial_index_true() {
        let info = make_table_info();
        assert!(info.has_spatial_index);
    }

    #[test]
    fn test_table_info_no_spatial_index() {
        let mut info = make_table_info();
        info.has_spatial_index = false;
        assert!(!info.has_spatial_index);
    }

    #[test]
    fn test_table_info_bounds_present() {
        let info = make_table_info();
        assert_eq!(info.bounds, Some([8.0, 47.0, 9.0, 48.0]));
    }

    #[test]
    fn test_table_info_no_bounds() {
        let mut info = make_table_info();
        info.bounds = None;
        assert!(info.bounds.is_none());
    }

    #[test]
    fn test_table_info_no_id_column() {
        let mut info = make_table_info();
        info.id_column = None;
        assert!(info.id_column.is_none());
    }

    #[test]
    fn test_table_info_empty_properties() {
        let mut info = make_table_info();
        info.properties = vec![];
        assert!(info.properties.is_empty());
    }

    #[test]
    fn test_table_info_polygon_geometry_type() {
        let mut info = make_table_info();
        info.geometry_type = "MULTIPOLYGON".to_string();
        assert_eq!(info.geometry_type, "MULTIPOLYGON");
    }

    #[test]
    fn test_table_info_debug_format() {
        let info = make_table_info();
        let debug = format!("{:?}", info);
        assert!(debug.contains("public"));
        assert!(debug.contains("points"));
        assert!(debug.contains("geom"));
    }

    #[test]
    fn test_table_info_clone() {
        let info = make_table_info();
        let cloned = info.clone();
        assert_eq!(cloned.schema, info.schema);
        assert_eq!(cloned.table, info.table);
        assert_eq!(cloned.srid, info.srid);
        assert_eq!(cloned.geometry_column, info.geometry_column);
    }

    #[test]
    fn test_build_tile_query_layer_name_matches_config_id() {
        let table_info = make_table_info();
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, false);

        assert!(query.contains("ST_AsMVT(tile, 'test_layer',"));
    }

    #[test]
    fn test_build_tile_query_references_correct_schema_table() {
        let table_info = make_table_info();
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, false);

        assert!(query.contains(r#"FROM "public"."points""#));
    }

    #[test]
    fn test_build_tile_query_geom_is_not_null_filter() {
        let table_info = make_table_info();
        let config = make_config();
        let query = PostgresTableSource::build_tile_query(&table_info, &config, false);

        assert!(query.contains("WHERE geom IS NOT NULL"));
    }
}
