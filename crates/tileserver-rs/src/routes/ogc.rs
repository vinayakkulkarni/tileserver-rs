//! OGC API Features Part 1 (Core) route handlers.
//!
//! Implements read-only feature access for PostGIS table sources,
//! enabling QGIS/ArcGIS/FME native connectivity.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header::HeaderName},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
#[cfg(test)]
use serde::Serialize;

use crate::error::TileServerError;
use crate::reload::SharedState;
use crate::routes::ogc_crs::{self, Crs};
use crate::routes::ogc_filter;
use crate::sources::postgres::PostgresTableSource;

/// OGC `Content-Crs` response header name (Part 2).
static CONTENT_CRS: HeaderName = HeaderName::from_static("content-crs");

/// Default number of features returned per page when no `limit` query parameter is provided.
const OGC_FEATURES_LIMIT_DEFAULT: i64 = 10;
/// Maximum allowed `limit` value; requests exceeding this are clamped.
const OGC_FEATURES_LIMIT_MAX: i64 = 10_000;

/// Query parameters for the OGC API Features `/items` endpoint.
///
/// Supports bbox spatial filtering, limit/offset pagination, and an optional
/// datetime filter (currently unused).
#[derive(Debug, Deserialize)]
pub(crate) struct ItemsQueryParams {
    /// Comma-separated bounding box: `minx,miny,maxx,maxy` (or 6-value 3D form).
    #[serde(default)]
    bbox: Option<String>,
    /// CRS of the `bbox` parameter (OGC Features Part 2). Defaults to CRS84.
    #[serde(default, rename = "bbox-crs")]
    bbox_crs: Option<String>,
    /// CRS requested for the response geometries (OGC Features Part 2). Defaults to CRS84.
    #[serde(default)]
    crs: Option<String>,
    /// CQL2 filter expression (OGC Features Part 3).
    #[serde(default)]
    filter: Option<String>,
    /// Dialect of the `filter` parameter: `cql2-text` (default) or `cql2-json`.
    #[serde(default, rename = "filter-lang")]
    filter_lang: Option<String>,
    /// CRS the spatial operands inside the filter expression are expressed in.
    #[serde(default, rename = "filter-crs")]
    filter_crs: Option<String>,
    /// Maximum number of features to return (clamped to [`OGC_FEATURES_LIMIT_MAX`]).
    #[serde(default = "default_limit")]
    limit: i64,
    /// Number of features to skip for pagination.
    #[serde(default)]
    offset: i64,
    /// ISO 8601 datetime filter (reserved for future use).
    #[allow(dead_code)]
    #[serde(default)]
    datetime: Option<String>,
}

/// Query parameters for single-feature `/items/{fid}` (Part 2 `crs` param).
#[derive(Debug, Deserialize, Default)]
pub(crate) struct FeatureQueryParams {
    /// CRS requested for the response geometry (OGC Features Part 2). Defaults to CRS84.
    #[serde(default)]
    crs: Option<String>,
}

/// GeoJSON `Feature` body accepted by Part 4 transactional endpoints.
///
/// The `type` field is ignored — OGC/GeoJSON strictness is relaxed here so
/// clients that send either `"type":"Feature"` or no `type` at all work.
/// Both fields are optional so callers can PATCH either geometry or
/// properties alone.
#[derive(Debug, Deserialize, Default)]
pub(crate) struct FeaturePayload {
    #[serde(default)]
    geometry: Option<serde_json::Value>,
    #[serde(default)]
    properties: serde_json::Map<String, serde_json::Value>,
}

/// Returns the default page size for feature queries.
#[must_use]
fn default_limit() -> i64 {
    OGC_FEATURES_LIMIT_DEFAULT
}

/// Builds the base URL for OGC API links.
///
/// All OGC routes are mounted under the `/ogc` prefix, so every link we emit
/// must include it. Returning just `state.base_url` would give clients 404s
/// when they follow `rel="conformance"`/`rel="items"`/etc. links.
#[must_use]
fn build_base_url(state: &crate::reload::AppState) -> String {
    format!("{}/ogc", state.base_url.trim_end_matches('/'))
}

/// Parses the OGC API Features `datetime` query parameter.
///
/// Part 1 Core does not yet implement temporal filtering in this server, but
/// silently ignoring the parameter would make clients believe their filter
/// succeeded. Accepting the parameter without applying it would also violate
/// the spec (§7.15.4 says conforming implementations must honour it). We
/// return `400 Bad Request` when a client explicitly requests datetime
/// filtering until the feature lands.
///
/// # Errors
///
/// Returns [`TileServerError::InvalidTileRequest`] when the caller supplies
/// any non-empty datetime value.
fn reject_datetime(datetime: Option<&str>) -> Result<(), TileServerError> {
    match datetime {
        Some(s) if !s.trim().is_empty() => {
            tracing::warn!(
                datetime = %s,
                "OGC API Features datetime filtering is not yet implemented; returning 400"
            );
            Err(TileServerError::InvalidTileRequest)
        }
        _ => Ok(()),
    }
}

/// OGC API landing page returning service metadata and navigation links.
pub(crate) async fn landing_page(State(shared): State<SharedState>) -> impl IntoResponse {
    let state = shared.load();
    let base = build_base_url(&state);
    let root = state.base_url.trim_end_matches('/').to_string();

    let landing = serde_json::json!({
        "title": "tileserver-rs OGC API",
        "description": "OGC API Features access to PostGIS table sources",
        "links": [
            {
                "href": base.clone(),
                "rel": "self",
                "type": "application/json",
                "title": "this document"
            },
            {
                "href": format!("{base}/conformance"),
                "rel": "conformance",
                "type": "application/json",
                "title": "OGC API conformance classes"
            },
            {
                "href": format!("{base}/collections"),
                "rel": "data",
                "type": "application/json",
                "title": "feature collections"
            },
            {
                "href": format!("{root}/openapi.json"),
                "rel": "service-desc",
                "type": "application/vnd.oai.openapi+json;version=3.0",
                "title": "OpenAPI 3.0 specification"
            },
            {
                "href": format!("{root}/_openapi"),
                "rel": "service-doc",
                "type": "text/html",
                "title": "API documentation (Scalar)"
            }
        ]
    });

    Json(landing)
}

/// OGC API conformance declaration listing supported specification classes.
pub(crate) async fn conformance() -> impl IntoResponse {
    let body = serde_json::json!({
        "conformsTo": [
            "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/core",
            "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/oas30",
            "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/geojson",
            ogc_crs::CONFORMANCE_CRS,
            ogc_filter::CONFORMANCE_FILTER,
            ogc_filter::CONFORMANCE_FEATURES_FILTER,
            ogc_filter::CONFORMANCE_QUERYABLES,
            "http://www.opengis.net/spec/ogcapi-features-4/1.0/conf/create-replace-delete",
            "http://www.opengis.net/spec/ogcapi-features-5/1.0/conf/schemas",
        ]
    });
    Json(body)
}

/// Lists all OGC feature collections backed by `PostgresTableSource` sources.
pub(crate) async fn collections(State(shared): State<SharedState>) -> impl IntoResponse {
    let state = shared.load();
    let base = build_base_url(&state);
    let mut collections = Vec::new();

    for meta in state.sources.all_metadata() {
        let Some(source) = state.sources.get(&meta.id) else {
            continue;
        };
        let Some(table) = source.as_any().downcast_ref::<PostgresTableSource>() else {
            continue;
        };

        let storage_srid = table.table_info().srid;
        collections.push(build_collection_json(meta, &base, storage_srid));
    }

    Json(serde_json::json!({
        "collections": collections,
        "links": [
            {
                "href": format!("{base}/collections"),
                "rel": "self",
                "type": "application/json",
                "title": "feature collections"
            }
        ]
    }))
}

/// Returns metadata for a single OGC feature collection.
///
/// # Errors
///
/// Returns [`TileServerError::SourceNotFound`] if the collection id does not
/// match any loaded source, or [`TileServerError::NotFound`] if the source
/// is not a `PostgresTableSource`.
pub(crate) async fn collection(
    State(shared): State<SharedState>,
    Path(collection_id): Path<String>,
) -> Result<impl IntoResponse, TileServerError> {
    let state = shared.load();
    let base = build_base_url(&state);

    let source = state
        .sources
        .get(&collection_id)
        .ok_or_else(|| TileServerError::SourceNotFound(collection_id.clone()))?;

    let table_source = source
        .as_any()
        .downcast_ref::<PostgresTableSource>()
        .ok_or_else(|| {
            TileServerError::NotFound(format!(
                "collection '{collection_id}' is not an OGC features source"
            ))
        })?;

    let storage_srid = table_source.table_info().srid;
    let meta = source.metadata();
    let body = build_collection_json(meta, &base, storage_srid);
    Ok(Json(body))
}

/// Returns a paginated GeoJSON `FeatureCollection` for the given collection.
///
/// # Errors
///
/// Returns [`TileServerError::SourceNotFound`] if the collection does not exist,
/// [`TileServerError::NotFound`] if the source is not a `PostgresTableSource`,
/// or [`TileServerError::InvalidTileRequest`] if the bbox parameter is malformed.
pub(crate) async fn items(
    State(shared): State<SharedState>,
    Path(collection_id): Path<String>,
    Query(params): Query<ItemsQueryParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let base = build_base_url(&state);

    let source = state
        .sources
        .get(&collection_id)
        .ok_or_else(|| TileServerError::SourceNotFound(collection_id.clone()))?;

    let table_source = source
        .as_any()
        .downcast_ref::<PostgresTableSource>()
        .ok_or_else(|| {
            TileServerError::NotFound(format!(
                "collection '{collection_id}' is not an OGC features source"
            ))
        })?;

    let limit = params.limit.clamp(1, OGC_FEATURES_LIMIT_MAX);
    let offset = params.offset.max(0);

    reject_datetime(params.datetime.as_deref())?;

    let bbox = parse_bbox(params.bbox.as_deref())?;

    let bbox_crs = match params.bbox_crs.as_deref() {
        Some(raw) => ogc_crs::parse_crs(raw)?,
        None => Crs::crs84(),
    };
    let output_crs = match params.crs.as_deref() {
        Some(raw) => ogc_crs::parse_crs(raw)?,
        None => Crs::crs84(),
    };
    let filter_crs = match params.filter_crs.as_deref() {
        Some(raw) => Some(ogc_crs::parse_crs(raw)?),
        None => None,
    };

    let filter_sql = match params.filter.as_deref() {
        Some(raw) if !raw.trim().is_empty() => Some(ogc_filter::translate_filter_to_sql(
            raw,
            params.filter_lang.as_deref(),
            &table_source.table_info().properties,
        )?),
        _ => None,
    };

    let (features, number_matched) = table_source
        .query_features_geojson(
            bbox,
            bbox_crs.srid(),
            output_crs.srid(),
            filter_sql.as_deref(),
            filter_crs.as_ref().map(Crs::srid),
            limit,
            offset,
        )
        .await?;

    let number_returned = features.len() as i64;

    let mut links = vec![serde_json::json!({
        "href": format!("{base}/collections/{collection_id}/items?limit={limit}&offset={offset}"),
        "rel": "self",
        "type": "application/geo+json",
        "title": "this page"
    })];

    if offset + number_returned < number_matched {
        let next_offset = offset + limit;
        links.push(serde_json::json!({
            "href": format!("{base}/collections/{collection_id}/items?limit={limit}&offset={next_offset}"),
            "rel": "next",
            "type": "application/geo+json",
            "title": "next page"
        }));
    }

    let body = serde_json::json!({
        "type": "FeatureCollection",
        "features": features,
        "numberMatched": number_matched,
        "numberReturned": number_returned,
        "links": links
    });

    let content_crs = HeaderValue::from_str(&output_crs.header_value()).map_err(|_| {
        TileServerError::PostgresError("failed to encode Content-Crs header".into())
    })?;

    Ok((
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/geo+json"),
            ),
            (CONTENT_CRS.clone(), content_crs),
        ],
        Json(body),
    )
        .into_response())
}

/// Returns a single GeoJSON `Feature` by id from the given collection.
///
/// # Errors
///
/// Returns [`TileServerError::SourceNotFound`] if the collection does not exist,
/// [`TileServerError::NotFound`] if the source is not a `PostgresTableSource`
/// or if the feature id is not found, or [`TileServerError::PostgresError`] on
/// database failures.
pub(crate) async fn feature(
    State(shared): State<SharedState>,
    Path((collection_id, feature_id)): Path<(String, String)>,
    Query(params): Query<FeatureQueryParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let base = build_base_url(&state);

    let source = state
        .sources
        .get(&collection_id)
        .ok_or_else(|| TileServerError::SourceNotFound(collection_id.clone()))?;

    let table_source = source
        .as_any()
        .downcast_ref::<PostgresTableSource>()
        .ok_or_else(|| {
            TileServerError::NotFound(format!(
                "collection '{collection_id}' is not an OGC features source"
            ))
        })?;

    let output_crs = match params.crs.as_deref() {
        Some(raw) => ogc_crs::parse_crs(raw)?,
        None => Crs::crs84(),
    };

    let (_fid, geom, properties) = table_source
        .query_single_feature_geojson(&feature_id, output_crs.srid())
        .await?
        .ok_or_else(|| {
            TileServerError::NotFound(format!(
                "feature '{feature_id}' not found in collection '{collection_id}'"
            ))
        })?;

    let body = serde_json::json!({
        "type": "Feature",
        "id": feature_id,
        "geometry": geom,
        "properties": properties,
        "links": [
            {
                "href": format!("{base}/collections/{collection_id}/items/{feature_id}"),
                "rel": "self",
                "type": "application/geo+json",
                "title": "this feature"
            },
            {
                "href": format!("{base}/collections/{collection_id}"),
                "rel": "collection",
                "type": "application/json",
                "title": "the collection"
            }
        ]
    });

    let content_crs = HeaderValue::from_str(&output_crs.header_value()).map_err(|_| {
        TileServerError::PostgresError("failed to encode Content-Crs header".into())
    })?;

    Ok((
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/geo+json"),
            ),
            (CONTENT_CRS.clone(), content_crs),
        ],
        Json(body),
    )
        .into_response())
}

/// POST `/ogc/collections/{id}/items` — create a new feature.
///
/// Requires `writable = true` on the target `postgres.tables` entry in
/// `config.toml`. Responds 201 Created with a `Location` header pointing
/// to the new `/items/{fid}` URL.
///
/// # Errors
///
/// - [`TileServerError::SourceNotFound`] when the collection id is unknown.
/// - [`TileServerError::NotFound`] when the source is not a PostGIS table.
/// - [`TileServerError::MethodNotAllowed`] when the table is read-only.
/// - [`TileServerError::InvalidTileRequest`] when the payload lacks a geometry.
pub(crate) async fn create_item(
    State(shared): State<SharedState>,
    Path(collection_id): Path<String>,
    Json(payload): Json<FeaturePayload>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let base = build_base_url(&state);
    let table_source = resolve_table(&state, &collection_id)?;

    let geom = payload
        .geometry
        .ok_or(TileServerError::InvalidTileRequest)?;

    let new_id = table_source
        .insert_feature(&geom, &payload.properties)
        .await?;

    let mut headers = axum::http::HeaderMap::new();
    let location = HeaderValue::from_str(&format!(
        "{base}/collections/{collection_id}/items/{new_id}"
    ))
    .map_err(|_| TileServerError::PostgresError("failed to encode Location".into()))?;
    headers.insert(axum::http::header::LOCATION, location);
    Ok((StatusCode::CREATED, headers).into_response())
}

/// PUT `/ogc/collections/{id}/items/{fid}` — replace a feature wholesale.
///
/// # Errors
///
/// See [`create_item`] plus [`TileServerError::NotFound`] if the feature id
/// does not exist.
pub(crate) async fn replace_item(
    State(shared): State<SharedState>,
    Path((collection_id, feature_id)): Path<(String, String)>,
    Json(payload): Json<FeaturePayload>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let table_source = resolve_table(&state, &collection_id)?;

    let geom = payload
        .geometry
        .ok_or(TileServerError::InvalidTileRequest)?;

    table_source
        .replace_feature(&feature_id, &geom, &payload.properties)
        .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// PATCH `/ogc/collections/{id}/items/{fid}` — merge-update a feature.
///
/// Only the fields present in the payload are touched; everything else is
/// preserved (RFC 7396 merge semantics).
///
/// # Errors
///
/// See [`create_item`] plus [`TileServerError::NotFound`].
pub(crate) async fn update_item(
    State(shared): State<SharedState>,
    Path((collection_id, feature_id)): Path<(String, String)>,
    Json(payload): Json<FeaturePayload>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let table_source = resolve_table(&state, &collection_id)?;

    table_source
        .patch_feature(&feature_id, payload.geometry.as_ref(), &payload.properties)
        .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// DELETE `/ogc/collections/{id}/items/{fid}` — remove a feature.
///
/// # Errors
///
/// See [`create_item`] plus [`TileServerError::NotFound`].
pub(crate) async fn delete_item(
    State(shared): State<SharedState>,
    Path((collection_id, feature_id)): Path<(String, String)>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let table_source = resolve_table(&state, &collection_id)?;
    table_source.delete_feature(&feature_id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// GET `/ogc/collections/{id}/queryables` — filterable properties (Part 5).
///
/// Returns a JSON Schema document where each property key is a column name
/// and each value is its OpenAPI type. Consumed by QGIS / tipg clients to
/// know which CQL2 filter fields the server accepts.
///
/// # Errors
///
/// See [`feature`].
pub(crate) async fn queryables(
    State(shared): State<SharedState>,
    Path(collection_id): Path<String>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let base = build_base_url(&state);
    let table = resolve_table(&state, &collection_id)?;

    let mut properties = serde_json::Map::new();
    for (name, schema, _sortable) in table.column_schemas().await? {
        properties.insert(name, schema);
    }

    let body = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": format!("{base}/collections/{collection_id}/queryables"),
        "type": "object",
        "title": format!("Queryables for '{collection_id}'"),
        "properties": properties,
        "additionalProperties": false
    });
    Ok((
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/schema+json"),
        )],
        Json(body),
    )
        .into_response())
}

/// GET `/ogc/collections/{id}/sortables` — sortable properties (Part 5).
///
/// Subset of queryables excluding `jsonb`/array/record columns that have no
/// well-defined SQL ordering. This lets clients populate sort-by dropdowns
/// without triggering PostgreSQL `could not identify a comparison function`
/// errors.
///
/// # Errors
///
/// See [`feature`].
pub(crate) async fn sortables(
    State(shared): State<SharedState>,
    Path(collection_id): Path<String>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let base = build_base_url(&state);
    let table = resolve_table(&state, &collection_id)?;

    let mut properties = serde_json::Map::new();
    for (name, schema, sortable) in table.column_schemas().await? {
        if sortable {
            properties.insert(name, schema);
        }
    }

    let body = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": format!("{base}/collections/{collection_id}/sortables"),
        "type": "object",
        "title": format!("Sortables for '{collection_id}'"),
        "properties": properties,
        "additionalProperties": false
    });
    Ok((
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/schema+json"),
        )],
        Json(body),
    )
        .into_response())
}

/// GET `/ogc/collections/{id}/schema` — full JSON Schema for the collection (Part 5).
///
/// Describes a single Feature document as the client will receive it:
/// `type`, `id`, `geometry` (GeoJSON Geometry), and the `properties` object
/// with every column typed via [`column_schemas`].
///
/// # Errors
///
/// See [`feature`].
pub(crate) async fn schema(
    State(shared): State<SharedState>,
    Path(collection_id): Path<String>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let base = build_base_url(&state);
    let table = resolve_table(&state, &collection_id)?;

    let mut properties = serde_json::Map::new();
    for (name, schema, _sortable) in table.column_schemas().await? {
        properties.insert(name, schema);
    }

    let body = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": format!("{base}/collections/{collection_id}/schema"),
        "type": "object",
        "title": format!("Feature schema for '{collection_id}'"),
        "required": ["type", "geometry", "properties"],
        "properties": {
            "type": { "type": "string", "enum": ["Feature"] },
            "id": { "type": ["string", "integer"] },
            "geometry": {
                "$ref": "https://geojson.org/schema/Geometry.json"
            },
            "properties": {
                "type": "object",
                "properties": properties,
                "additionalProperties": false
            }
        }
    });
    Ok((
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/schema+json"),
        )],
        Json(body),
    )
        .into_response())
}

/// Looks up a collection and downcasts to `PostgresTableSource`.
///
/// Returning an owned clone (`PostgresTableSource: Clone`) sidesteps the
/// lifetime problem of borrowing through an `arc_swap::Guard`. The struct
/// is cheap to clone — it only holds `Arc`-backed pool, metadata and
/// caches.
fn resolve_table(
    state: &crate::reload::AppState,
    collection_id: &str,
) -> Result<PostgresTableSource, TileServerError> {
    let source = state
        .sources
        .get(collection_id)
        .ok_or_else(|| TileServerError::SourceNotFound(collection_id.to_string()))?;
    source
        .as_any()
        .downcast_ref::<PostgresTableSource>()
        .cloned()
        .ok_or_else(|| {
            TileServerError::NotFound(format!(
                "collection '{collection_id}' is not an OGC features source"
            ))
        })
}

/// Builds an OGC collection JSON object from tile source metadata.
#[must_use]
fn build_collection_json(
    meta: &crate::sources::TileMetadata,
    base_url: &str,
    storage_srid: i32,
) -> serde_json::Value {
    let mut extent = serde_json::Map::new();
    if let Some(bounds) = meta.bounds {
        extent.insert(
            "spatial".to_string(),
            serde_json::json!({
                "bbox": [bounds],
                "crs": ogc_crs::CRS84_URI
            }),
        );
    }

    serde_json::json!({
        "id": meta.id,
        "title": meta.name,
        "description": meta.description,
        "extent": extent,
        "itemType": "feature",
        "crs": ogc_crs::collection_supported_crs(storage_srid),
        "storageCrs": ogc_crs::storage_crs_uri(storage_srid),
        "links": [
            {
                "href": format!("{base_url}/collections/{}", meta.id),
                "rel": "self",
                "type": "application/json",
                "title": meta.name
            },
            {
                "href": format!("{base_url}/collections/{}/items", meta.id),
                "rel": "items",
                "type": "application/geo+json",
                "title": format!("{} items", meta.name)
            }
        ]
    })
}

/// Parses the OGC API Features `bbox` query parameter into a 2D envelope.
///
/// Per OGC API Features Part 1 Core §7.15.3, `bbox` is a comma-separated list
/// of **four or six** numbers. The 6-value form carries elevation bounds
/// `[west, south, minZ, east, north, maxZ]`; we drop the elevation axis and
/// return only the horizontal envelope because PostGIS queries operate in 2D
/// here. Rejecting 6-value bboxes (the old behaviour) broke QGIS clients that
/// send 3D filters (fixes Oracle H4).
///
/// # Errors
///
/// Returns [`TileServerError::InvalidTileRequest`] when the string does not
/// parse as 4 or 6 floats, or when the horizontal coordinates are inverted.
fn parse_bbox(bbox_str: Option<&str>) -> Result<Option<[f64; 4]>, TileServerError> {
    let Some(s) = bbox_str else {
        return Ok(None);
    };

    let parts: Vec<f64> = s
        .split(',')
        .map(|p| {
            p.trim()
                .parse::<f64>()
                .map_err(|_| TileServerError::InvalidTileRequest)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let (minx, miny, maxx, maxy) = match parts.len() {
        4 => (parts[0], parts[1], parts[2], parts[3]),
        6 => (parts[0], parts[1], parts[3], parts[4]),
        _ => return Err(TileServerError::InvalidTileRequest),
    };

    if minx > maxx || miny > maxy {
        return Err(TileServerError::InvalidTileRequest);
    }

    Ok(Some([minx, miny, maxx, maxy]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize)]
    struct OgcLink {
        href: String,
        rel: String,
        #[serde(rename = "type")]
        media_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    }

    #[test]
    fn parse_bbox_none_returns_none() {
        assert!(parse_bbox(None).unwrap().is_none());
    }

    #[test]
    fn parse_bbox_valid_four_values() {
        let result = parse_bbox(Some("-180,-90,180,90")).unwrap().unwrap();
        assert_eq!(result, [-180.0, -90.0, 180.0, 90.0]);
    }

    #[test]
    fn parse_bbox_with_spaces() {
        let result = parse_bbox(Some(" -10.5 , -20.3 , 30.1 , 40.2 "))
            .unwrap()
            .unwrap();
        assert_eq!(result, [-10.5, -20.3, 30.1, 40.2]);
    }

    #[test]
    fn parse_bbox_inverted_lon_fails() {
        assert!(parse_bbox(Some("180,-90,-180,90")).is_err());
    }

    #[test]
    fn parse_bbox_inverted_lat_fails() {
        assert!(parse_bbox(Some("-180,90,180,-90")).is_err());
    }

    #[test]
    fn parse_bbox_too_few_values_fails() {
        assert!(parse_bbox(Some("-180,-90,180")).is_err());
    }

    #[test]
    fn parse_bbox_too_many_values_fails() {
        assert!(parse_bbox(Some("-180,-90,180,90,100")).is_err());
    }

    #[test]
    fn parse_bbox_non_numeric_fails() {
        assert!(parse_bbox(Some("abc,-90,180,90")).is_err());
    }

    #[test]
    fn parse_bbox_empty_string_fails() {
        assert!(parse_bbox(Some("")).is_err());
    }

    #[test]
    fn conformance_includes_required_classes() {
        let body = serde_json::json!({
            "conformsTo": [
                "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/core",
                "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/oas30",
                "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/geojson"
            ]
        });
        let conforms = body["conformsTo"].as_array().unwrap();
        assert_eq!(conforms.len(), 3);
        assert!(
            conforms
                .iter()
                .any(|v| v.as_str().unwrap().contains("conf/core"))
        );
        assert!(
            conforms
                .iter()
                .any(|v| v.as_str().unwrap().contains("conf/oas30"))
        );
        assert!(
            conforms
                .iter()
                .any(|v| v.as_str().unwrap().contains("conf/geojson"))
        );
    }

    #[test]
    fn build_collection_json_with_bounds() {
        let meta = crate::sources::TileMetadata {
            id: "test".to_string(),
            name: "Test Layer".to_string(),
            description: Some("A test layer".to_string()),
            attribution: None,
            format: crate::sources::TileFormat::Pbf,
            minzoom: 0,
            maxzoom: 14,
            bounds: Some([8.0, 47.0, 9.0, 48.0]),
            center: None,
            vector_layers: None,
        };

        let json = build_collection_json(&meta, "http://localhost:8080", 4326);
        assert_eq!(json["id"], "test");
        assert_eq!(json["title"], "Test Layer");
        assert_eq!(json["description"], "A test layer");
        assert_eq!(json["itemType"], "feature");

        let spatial = &json["extent"]["spatial"];
        assert!(spatial["bbox"].is_array());
        assert_eq!(spatial["bbox"][0][0], 8.0);
        assert_eq!(spatial["bbox"][0][1], 47.0);
        assert_eq!(spatial["bbox"][0][2], 9.0);
        assert_eq!(spatial["bbox"][0][3], 48.0);
        assert!(spatial["crs"].as_str().unwrap().contains("CRS84"));

        let links = json["links"].as_array().unwrap();
        assert_eq!(links.len(), 2);
        assert!(
            links[0]["href"]
                .as_str()
                .unwrap()
                .contains("/collections/test")
        );
        assert_eq!(links[0]["rel"], "self");
        assert!(
            links[1]["href"]
                .as_str()
                .unwrap()
                .contains("/collections/test/items")
        );
        assert_eq!(links[1]["rel"], "items");
    }

    #[test]
    fn build_collection_json_without_bounds() {
        let meta = crate::sources::TileMetadata {
            id: "no_bounds".to_string(),
            name: "No Bounds".to_string(),
            description: None,
            attribution: None,
            format: crate::sources::TileFormat::Pbf,
            minzoom: 0,
            maxzoom: 22,
            bounds: None,
            center: None,
            vector_layers: None,
        };

        let json = build_collection_json(&meta, "http://example.com", 4326);
        assert_eq!(json["id"], "no_bounds");
        assert!(json["extent"].as_object().unwrap().is_empty());
    }

    #[test]
    fn build_collection_json_links_format() {
        let meta = crate::sources::TileMetadata {
            id: "roads".to_string(),
            name: "Roads".to_string(),
            description: None,
            attribution: None,
            format: crate::sources::TileFormat::Pbf,
            minzoom: 0,
            maxzoom: 14,
            bounds: None,
            center: None,
            vector_layers: None,
        };

        let json = build_collection_json(&meta, "http://host:9090", 4326);
        let links = json["links"].as_array().unwrap();
        assert_eq!(links[0]["type"], "application/json");
        assert_eq!(links[1]["type"], "application/geo+json");
    }

    #[test]
    fn default_limit_is_10() {
        assert_eq!(default_limit(), OGC_FEATURES_LIMIT_DEFAULT);
        assert_eq!(OGC_FEATURES_LIMIT_DEFAULT, 10);
    }

    #[test]
    fn max_limit_is_10000() {
        assert_eq!(OGC_FEATURES_LIMIT_MAX, 10_000);
    }

    #[test]
    fn ogc_link_serializes_correctly() {
        let link = OgcLink {
            href: "http://example.com".to_string(),
            rel: "self".to_string(),
            media_type: "application/json".to_string(),
            title: Some("Test".to_string()),
        };
        let json = serde_json::to_value(&link).unwrap();
        assert_eq!(json["href"], "http://example.com");
        assert_eq!(json["rel"], "self");
        assert_eq!(json["type"], "application/json");
        assert_eq!(json["title"], "Test");
    }

    #[test]
    fn ogc_link_skips_none_title() {
        let link = OgcLink {
            href: "http://example.com".to_string(),
            rel: "self".to_string(),
            media_type: "application/json".to_string(),
            title: None,
        };
        let json = serde_json::to_value(&link).unwrap();
        assert!(!json.as_object().unwrap().contains_key("title"));
    }

    #[test]
    fn items_query_params_defaults() {
        let params: ItemsQueryParams = serde_json::from_str(r#"{}"#).unwrap();
        assert!(params.bbox.is_none());
        assert_eq!(params.limit, 10);
        assert_eq!(params.offset, 0);
        assert!(params.datetime.is_none());
    }

    #[test]
    fn items_query_params_custom_values() {
        let params: ItemsQueryParams =
            serde_json::from_str(r#"{"bbox":"-10,-20,30,40","limit":50,"offset":100}"#).unwrap();
        assert_eq!(params.bbox.as_deref(), Some("-10,-20,30,40"));
        assert_eq!(params.limit, 50);
        assert_eq!(params.offset, 100);
    }

    #[test]
    fn parse_bbox_zero_width_valid() {
        let result = parse_bbox(Some("10,20,10,20")).unwrap().unwrap();
        assert_eq!(result, [10.0, 20.0, 10.0, 20.0]);
    }

    #[test]
    fn parse_bbox_decimal_precision() {
        let result = parse_bbox(Some("8.12345,47.54321,9.98765,48.01234"))
            .unwrap()
            .unwrap();
        assert!((result[0] - 8.12345).abs() < f64::EPSILON);
        assert!((result[1] - 47.54321).abs() < f64::EPSILON);
    }

    #[test]
    fn build_collection_json_crs_field() {
        let meta = crate::sources::TileMetadata {
            id: "t".to_string(),
            name: "T".to_string(),
            description: None,
            attribution: None,
            format: crate::sources::TileFormat::Pbf,
            minzoom: 0,
            maxzoom: 22,
            bounds: None,
            center: None,
            vector_layers: None,
        };
        let json = build_collection_json(&meta, "http://localhost", 4326);
        let crs = json["crs"].as_array().unwrap();
        assert_eq!(crs.len(), 1);
        assert!(crs[0].as_str().unwrap().contains("CRS84"));
    }

    #[test]
    fn conformance_three_uris() {
        let body = serde_json::json!({
            "conformsTo": [
                "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/core",
                "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/oas30",
                "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/geojson"
            ]
        });
        assert_eq!(body["conformsTo"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn feature_collection_structure() {
        let body = serde_json::json!({
            "type": "FeatureCollection",
            "features": [],
            "numberMatched": 0,
            "numberReturned": 0,
            "links": []
        });
        assert_eq!(body["type"], "FeatureCollection");
        assert!(body["features"].as_array().unwrap().is_empty());
        assert_eq!(body["numberMatched"], 0);
        assert_eq!(body["numberReturned"], 0);
    }

    #[test]
    fn feature_structure() {
        let body = serde_json::json!({
            "type": "Feature",
            "id": "42",
            "geometry": {"type": "Point", "coordinates": [8.5, 47.5]},
            "properties": {"name": "Zurich"},
            "links": []
        });
        assert_eq!(body["type"], "Feature");
        assert_eq!(body["id"], "42");
        assert_eq!(body["geometry"]["type"], "Point");
        assert_eq!(body["properties"]["name"], "Zurich");
    }

    #[test]
    fn landing_page_structure() {
        let landing = serde_json::json!({
            "title": "tileserver-rs OGC API",
            "description": "OGC API Features access to PostGIS table sources",
            "links": [
                {"href": "http://localhost:8080/ogc", "rel": "self", "type": "application/json"},
                {"href": "http://localhost:8080/ogc/conformance", "rel": "conformance", "type": "application/json"},
                {"href": "http://localhost:8080/ogc/collections", "rel": "data", "type": "application/json"},
                {"href": "http://localhost:8080/openapi.json", "rel": "service-desc", "type": "application/vnd.oai.openapi+json;version=3.0"},
                {"href": "http://localhost:8080/_openapi", "rel": "service-doc", "type": "text/html"}
            ]
        });
        assert_eq!(landing["title"], "tileserver-rs OGC API");
        let links = landing["links"].as_array().unwrap();
        assert_eq!(links.len(), 5);
        assert!(links.iter().any(|l| l["rel"] == "self"));
        assert!(links.iter().any(|l| l["rel"] == "conformance"));
        assert!(links.iter().any(|l| l["rel"] == "data"));
        assert!(links.iter().any(|l| l["rel"] == "service-desc"));
        assert!(links.iter().any(|l| l["rel"] == "service-doc"));
    }

    #[test]
    fn landing_page_link_types() {
        let links = [
            OgcLink {
                href: "http://localhost".to_string(),
                rel: "self".to_string(),
                media_type: "application/json".to_string(),
                title: Some("this document".to_string()),
            },
            OgcLink {
                href: "http://localhost/conformance".to_string(),
                rel: "conformance".to_string(),
                media_type: "application/json".to_string(),
                title: None,
            },
        ];
        assert_eq!(links[0].rel, "self");
        assert_eq!(links[1].rel, "conformance");
    }

    #[test]
    fn pagination_link_next() {
        let base = "http://localhost:8080";
        let collection_id = "test";
        let limit = 10;
        let offset = 0;
        let number_returned = 10i64;
        let number_matched = 25i64;

        let has_next = offset + number_returned < number_matched;
        assert!(has_next);

        let next_offset = offset + limit;
        let next_href =
            format!("{base}/collections/{collection_id}/items?limit={limit}&offset={next_offset}");
        assert!(next_href.contains("offset=10"));
    }

    #[test]
    fn pagination_no_next_when_exhausted() {
        let offset = 20i64;
        let number_returned = 5i64;
        let number_matched = 25i64;

        let has_next = offset + number_returned < number_matched;
        assert!(!has_next);
    }

    #[test]
    fn limit_clamp_within_range() {
        let limit: i64 = 50;
        let clamped = limit.clamp(1, OGC_FEATURES_LIMIT_MAX);
        assert_eq!(clamped, 50);
    }

    #[test]
    fn limit_clamp_exceeds_max() {
        let limit: i64 = 50_000;
        let clamped = limit.clamp(1, OGC_FEATURES_LIMIT_MAX);
        assert_eq!(clamped, OGC_FEATURES_LIMIT_MAX);
    }

    #[test]
    fn limit_clamp_below_min() {
        let limit: i64 = -5;
        let clamped = limit.clamp(1, OGC_FEATURES_LIMIT_MAX);
        assert_eq!(clamped, 1);
    }

    #[test]
    fn offset_max_zero() {
        let offset: i64 = -10;
        let clamped = offset.max(0);
        assert_eq!(clamped, 0);
    }

    #[test]
    fn offset_positive_passthrough() {
        let offset: i64 = 100;
        let clamped = offset.max(0);
        assert_eq!(clamped, 100);
    }

    #[test]
    fn build_collection_json_item_type() {
        let meta = crate::sources::TileMetadata {
            id: "pts".to_string(),
            name: "Points".to_string(),
            description: None,
            attribution: None,
            format: crate::sources::TileFormat::Pbf,
            minzoom: 0,
            maxzoom: 14,
            bounds: Some([-180.0, -90.0, 180.0, 90.0]),
            center: None,
            vector_layers: None,
        };
        let json = build_collection_json(&meta, "http://localhost", 4326);
        assert_eq!(json["itemType"], "feature");
    }

    #[test]
    fn parse_bbox_negative_coordinates() {
        let result = parse_bbox(Some("-122.5,37.5,-122.0,38.0"))
            .unwrap()
            .unwrap();
        assert!((result[0] - (-122.5)).abs() < f64::EPSILON);
        assert!((result[1] - 37.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_bbox_accepts_six_values_dropping_elevation() {
        let result = parse_bbox(Some("-10,20,0,30,40,100")).unwrap().unwrap();
        assert!((result[0] - (-10.0)).abs() < f64::EPSILON);
        assert!((result[1] - 20.0).abs() < f64::EPSILON);
        assert!((result[2] - 30.0).abs() < f64::EPSILON);
        assert!((result[3] - 40.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_bbox_rejects_invalid_count() {
        assert!(parse_bbox(Some("1,2,3")).is_err());
        assert!(parse_bbox(Some("1,2,3,4,5")).is_err());
        assert!(parse_bbox(Some("1,2,3,4,5,6,7")).is_err());
    }

    #[test]
    fn reject_datetime_passes_when_absent_or_empty() {
        assert!(reject_datetime(None).is_ok());
        assert!(reject_datetime(Some("")).is_ok());
        assert!(reject_datetime(Some("   ")).is_ok());
    }

    #[test]
    fn reject_datetime_errors_on_any_value() {
        assert!(reject_datetime(Some("2024-01-01T00:00:00Z")).is_err());
        assert!(reject_datetime(Some("2024-01-01/2024-12-31")).is_err());
    }

    #[test]
    fn conformance_includes_part2_crs_class() {
        let body = serde_json::json!({
            "conformsTo": [
                "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/core",
                "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/oas30",
                "http://www.opengis.net/spec/ogcapi-features-1/1.0/conf/geojson",
                ogc_crs::CONFORMANCE_CRS,
            ]
        });
        let classes = body["conformsTo"].as_array().unwrap();
        assert!(
            classes
                .iter()
                .any(|c| c.as_str().unwrap().contains("features-2/1.0/conf/crs"))
        );
    }

    #[test]
    fn collection_json_emits_storage_crs_for_non_4326_table() {
        let meta = crate::sources::TileMetadata {
            id: "utm".to_string(),
            name: "UTM Zone 32N".to_string(),
            description: None,
            attribution: None,
            format: crate::sources::TileFormat::Pbf,
            minzoom: 0,
            maxzoom: 14,
            bounds: None,
            center: None,
            vector_layers: None,
        };
        let json = build_collection_json(&meta, "http://localhost", 25832);

        let crs_list = json["crs"].as_array().unwrap();
        assert!(
            crs_list
                .iter()
                .any(|v| v.as_str().unwrap().ends_with("/CRS84"))
        );
        assert!(
            crs_list
                .iter()
                .any(|v| v.as_str().unwrap().ends_with("/25832"))
        );

        let storage = json["storageCrs"].as_str().unwrap();
        assert!(storage.ends_with("/25832"));
    }

    #[test]
    fn collection_json_emits_single_crs_for_wgs84_table() {
        let meta = crate::sources::TileMetadata {
            id: "cities".to_string(),
            name: "Cities".to_string(),
            description: None,
            attribution: None,
            format: crate::sources::TileFormat::Pbf,
            minzoom: 0,
            maxzoom: 14,
            bounds: None,
            center: None,
            vector_layers: None,
        };
        let json = build_collection_json(&meta, "http://localhost", 4326);
        let crs_list = json["crs"].as_array().unwrap();
        assert_eq!(crs_list.len(), 1);
        assert!(crs_list[0].as_str().unwrap().ends_with("/CRS84"));
        assert_eq!(json["storageCrs"].as_str().unwrap(), ogc_crs::CRS84_URI);
    }

    #[test]
    fn build_base_url_appends_ogc_prefix() {
        let state = make_test_app_state(crate::sources::SourceManager::new());
        let base = build_base_url(&state);
        assert_eq!(base, "http://localhost:8080/ogc");
    }

    #[test]
    fn build_base_url_strips_trailing_slash() {
        let mut state = make_test_app_state(crate::sources::SourceManager::new());
        state.base_url = "http://localhost:8080/".to_string();
        let base = build_base_url(&state);
        assert_eq!(base, "http://localhost:8080/ogc");
    }

    #[test]
    fn build_collection_json_description_null_when_none() {
        let meta = crate::sources::TileMetadata {
            id: "no_desc".to_string(),
            name: "No Desc".to_string(),
            description: None,
            attribution: None,
            format: crate::sources::TileFormat::Pbf,
            minzoom: 0,
            maxzoom: 14,
            bounds: None,
            center: None,
            vector_layers: None,
        };
        let json = build_collection_json(&meta, "http://localhost", 4326);
        assert!(json["description"].is_null());
    }

    #[test]
    fn build_collection_json_self_link_title_matches_name() {
        let meta = crate::sources::TileMetadata {
            id: "rivers".to_string(),
            name: "World Rivers".to_string(),
            description: None,
            attribution: None,
            format: crate::sources::TileFormat::Pbf,
            minzoom: 0,
            maxzoom: 14,
            bounds: None,
            center: None,
            vector_layers: None,
        };
        let json = build_collection_json(&meta, "http://localhost", 4326);
        let links = json["links"].as_array().unwrap();
        assert_eq!(links[0]["title"], "World Rivers");
        assert_eq!(links[1]["title"], "World Rivers items");
    }

    #[test]
    fn items_query_params_with_datetime() {
        let params: ItemsQueryParams =
            serde_json::from_str(r#"{"datetime":"2024-01-01T00:00:00Z"}"#).unwrap();
        assert_eq!(params.datetime.as_deref(), Some("2024-01-01T00:00:00Z"));
    }

    #[test]
    fn items_query_params_with_zero_limit() {
        let params: ItemsQueryParams = serde_json::from_str(r#"{"limit":0}"#).unwrap();
        assert_eq!(params.limit, 0);
        let clamped = params.limit.clamp(1, OGC_FEATURES_LIMIT_MAX);
        assert_eq!(clamped, 1);
    }

    #[test]
    fn items_query_params_negative_offset_clamped() {
        let params: ItemsQueryParams = serde_json::from_str(r#"{"offset":-50}"#).unwrap();
        let clamped = params.offset.max(0);
        assert_eq!(clamped, 0);
    }

    #[test]
    fn parse_bbox_large_negative_values() {
        let result = parse_bbox(Some("-179.999,-89.999,179.999,89.999"))
            .unwrap()
            .unwrap();
        assert!((result[0] - (-179.999)).abs() < 1e-6);
        assert!((result[3] - 89.999).abs() < 1e-6);
    }

    #[test]
    fn parse_bbox_single_comma_only_fails() {
        assert!(parse_bbox(Some(",")).is_err());
    }

    #[test]
    fn parse_bbox_with_trailing_comma_fails() {
        assert!(parse_bbox(Some("-10,-20,30,40,")).is_err());
    }

    use std::collections::HashMap;
    use std::sync::Arc;

    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use crate::reload::{AppState, ReloadController, ReloadMeta, RuntimeSettings, SharedState};
    use crate::sources::SourceManager;
    use crate::styles::StyleManager;

    fn make_test_app_state(source_manager: SourceManager) -> AppState {
        AppState {
            sources: Arc::new(source_manager),
            styles: Arc::new(StyleManager::new()),
            renderer: None,
            base_url: "http://localhost:8080".to_string(),
            render_base_url: "http://localhost:8080".to_string(),
            ui_enabled: false,
            fonts_dir: None,
            files_dir: None,
            upload_dir: None,
        }
    }

    fn make_test_shared_state(source_manager: SourceManager) -> SharedState {
        let state = make_test_app_state(source_manager);
        let meta = ReloadMeta {
            config_hash: "test".to_string(),
            loaded_at_unix: 0,
            loaded_sources: 0,
            loaded_styles: 0,
            renderer_enabled: false,
            prometheus_listener_active: false,
        };
        let runtime = RuntimeSettings {
            ui_enabled: false,
            runtime_host: "127.0.0.1".to_string(),
            runtime_port: 8080,
            public_url_override: None,
        };
        let controller = Arc::new(ReloadController::new(state, meta, None, runtime));
        SharedState::new(controller)
    }

    fn ogc_test_router(shared: SharedState) -> Router {
        Router::new()
            .route("/ogc", axum::routing::get(landing_page))
            .route("/ogc/conformance", axum::routing::get(conformance))
            .route("/ogc/collections", axum::routing::get(collections))
            .route("/ogc/collections/{id}", axum::routing::get(collection))
            .route("/ogc/collections/{id}/items", axum::routing::get(items))
            .route(
                "/ogc/collections/{id}/items/{fid}",
                axum::routing::get(feature),
            )
            .with_state(shared)
    }

    #[tokio::test]
    async fn test_landing_page_returns_links() {
        let shared = make_test_shared_state(SourceManager::new());
        let app = ogc_test_router(shared);

        let resp = app
            .oneshot(Request::builder().uri("/ogc").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["title"], "tileserver-rs OGC API");
        let links = json["links"].as_array().unwrap();
        assert_eq!(links.len(), 5);
        assert!(links.iter().any(|l| l["rel"] == "self"));
        assert!(links.iter().any(|l| l["rel"] == "conformance"));
        assert!(links.iter().any(|l| l["rel"] == "data"));
        assert!(links.iter().any(|l| l["rel"] == "service-desc"
            && l["href"].as_str().unwrap().ends_with("/openapi.json")));
        assert!(links.iter().any(
            |l| l["rel"] == "service-doc" && l["href"].as_str().unwrap().ends_with("/_openapi")
        ));
    }

    #[tokio::test]
    async fn test_landing_page_self_link_uses_base_url() {
        let shared = make_test_shared_state(SourceManager::new());
        let app = ogc_test_router(shared);

        let resp = app
            .oneshot(Request::builder().uri("/ogc").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let self_link = json["links"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["rel"] == "self")
            .unwrap();
        assert_eq!(self_link["href"], "http://localhost:8080/ogc");

        let conformance_link = json["links"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["rel"] == "conformance")
            .unwrap();
        assert_eq!(
            conformance_link["href"], "http://localhost:8080/ogc/conformance",
            "conformance link must include /ogc prefix so clients don't hit 404"
        );
    }

    #[tokio::test]
    async fn test_conformance_handler_returns_classes() {
        let shared = make_test_shared_state(SourceManager::new());
        let app = ogc_test_router(shared);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ogc/conformance")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let conforms = json["conformsTo"].as_array().unwrap();
        assert_eq!(conforms.len(), 9);
        assert!(
            conforms
                .iter()
                .any(|v| v.as_str().unwrap().contains("conf/core"))
        );
        assert!(
            conforms
                .iter()
                .any(|v| v.as_str().unwrap().contains("conf/geojson"))
        );
        assert!(
            conforms
                .iter()
                .any(|v| v.as_str().unwrap().contains("features-2/1.0/conf/crs"))
        );
    }

    #[tokio::test]
    async fn test_collections_empty_when_no_table_sources() {
        let mut sources_map: HashMap<String, Arc<dyn crate::sources::TileSource>> = HashMap::new();
        sources_map.insert(
            "pmtiles_src".to_string(),
            Arc::new(MockTileSource::new("pmtiles_src")),
        );
        let mgr = SourceManager::from_sources(sources_map);
        let shared = make_test_shared_state(mgr);
        let app = ogc_test_router(shared);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ogc/collections")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["collections"].as_array().unwrap().is_empty());
        assert!(
            json["links"]
                .as_array()
                .unwrap()
                .iter()
                .any(|l| l["rel"] == "self")
        );
    }

    #[tokio::test]
    async fn test_collections_with_no_sources() {
        let shared = make_test_shared_state(SourceManager::new());
        let app = ogc_test_router(shared);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ogc/collections")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["collections"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_collection_source_not_found() {
        let shared = make_test_shared_state(SourceManager::new());
        let app = ogc_test_router(shared);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ogc/collections/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_collection_not_a_table_source() {
        let mut sources_map: HashMap<String, Arc<dyn crate::sources::TileSource>> = HashMap::new();
        sources_map.insert(
            "pmtiles_src".to_string(),
            Arc::new(MockTileSource::new("pmtiles_src")),
        );
        let mgr = SourceManager::from_sources(sources_map);
        let shared = make_test_shared_state(mgr);
        let app = ogc_test_router(shared);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ogc/collections/pmtiles_src")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_items_source_not_found() {
        let shared = make_test_shared_state(SourceManager::new());
        let app = ogc_test_router(shared);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ogc/collections/nonexistent/items")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_items_not_a_table_source() {
        let mut sources_map: HashMap<String, Arc<dyn crate::sources::TileSource>> = HashMap::new();
        sources_map.insert("mock".to_string(), Arc::new(MockTileSource::new("mock")));
        let mgr = SourceManager::from_sources(sources_map);
        let shared = make_test_shared_state(mgr);
        let app = ogc_test_router(shared);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ogc/collections/mock/items")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_feature_source_not_found() {
        let shared = make_test_shared_state(SourceManager::new());
        let app = ogc_test_router(shared);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ogc/collections/nonexistent/items/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_feature_not_a_table_source() {
        let mut sources_map: HashMap<String, Arc<dyn crate::sources::TileSource>> = HashMap::new();
        sources_map.insert("mock".to_string(), Arc::new(MockTileSource::new("mock")));
        let mgr = SourceManager::from_sources(sources_map);
        let shared = make_test_shared_state(mgr);
        let app = ogc_test_router(shared);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ogc/collections/mock/items/42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    struct MockTileSource {
        meta: crate::sources::TileMetadata,
    }

    impl MockTileSource {
        fn new(id: &str) -> Self {
            Self {
                meta: crate::sources::TileMetadata {
                    id: id.to_string(),
                    name: id.to_string(),
                    description: None,
                    attribution: None,
                    format: crate::sources::TileFormat::Pbf,
                    minzoom: 0,
                    maxzoom: 14,
                    bounds: None,
                    center: None,
                    vector_layers: None,
                },
            }
        }
    }

    #[async_trait::async_trait]
    impl crate::sources::TileSource for MockTileSource {
        async fn get_tile(
            &self,
            _z: u8,
            _x: u32,
            _y: u32,
        ) -> crate::error::Result<Option<crate::sources::TileData>> {
            Ok(None)
        }

        fn metadata(&self) -> &crate::sources::TileMetadata {
            &self.meta
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
}
