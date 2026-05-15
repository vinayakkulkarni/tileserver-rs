//! Spatial API route handlers (for LLM tool integration).
//!
//! Endpoints for querying source schemas, statistics, and features
//! from vector tile sources.

use axum::{
    Json,
    extract::{Path, State},
};

use crate::error::TileServerError;
use crate::reload::SharedState;
use crate::sources;

/// Layer info extracted from vector_layers metadata
#[derive(serde::Serialize)]
struct SpatialLayerInfo {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minzoom: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maxzoom: Option<u64>,
    /// Known field names and their types (from TileJSON metadata)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fields: Vec<SpatialFieldInfo>,
}

/// Field info from vector layer metadata
#[derive(serde::Serialize)]
struct SpatialFieldInfo {
    name: String,
    #[serde(rename = "type")]
    field_type: String,
}

/// Schema response for a source
#[derive(serde::Serialize)]
pub(super) struct SpatialSchemaResponse {
    source: String,
    format: String,
    minzoom: u8,
    maxzoom: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    bounds: Option<[f64; 4]>,
    layers: Vec<SpatialLayerInfo>,
}

/// Stats response for a source
#[derive(serde::Serialize)]
pub(super) struct SpatialStatsResponse {
    source: String,
    format: String,
    minzoom: u8,
    maxzoom: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    bounds: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    center: Option<[f64; 3]>,
    layer_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attribution: Option<String>,
}

/// Request body for spatial query
#[derive(serde::Deserialize)]
pub(super) struct SpatialQueryRequest {
    /// Source ID to query
    source: String,
    /// Bounding box [west, south, east, north]
    #[serde(default)]
    bbox: Option<[f64; 4]>,
    /// Zoom level for tile resolution
    #[serde(default = "default_zoom")]
    zoom: u8,
    /// Optional layer filter (only return features from these layers)
    #[serde(default)]
    layers: Option<Vec<String>>,
    /// Maximum features to return
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_zoom() -> u8 {
    14
}

fn default_limit() -> usize {
    100
}

/// Feature in spatial query response
#[derive(serde::Serialize)]
struct SpatialFeature {
    layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    geometry_type: Option<String>,
    properties: serde_json::Value,
}

/// Response for spatial query
#[derive(serde::Serialize)]
pub(super) struct SpatialQueryResponse {
    source: String,
    features: Vec<SpatialFeature>,
    total: usize,
    truncated: bool,
}

/// Get schema information for a source (vector layers, field types)
/// Route: GET /api/spatial/schema/{source}
pub(crate) async fn get_spatial_schema(
    State(shared): State<SharedState>,
    Path(source_id): Path<String>,
) -> Result<Json<SpatialSchemaResponse>, TileServerError> {
    let state = shared.load();
    let source = match state.sources.get(&source_id) {
        Some(s) => s,
        None => return Err(TileServerError::SourceNotFound(source_id)),
    };

    let meta = source.metadata();
    let layers = extract_layer_info(&meta.vector_layers);

    Ok(Json(SpatialSchemaResponse {
        source: meta.id.clone(),
        format: meta.format.extension().to_string(),
        minzoom: meta.minzoom,
        maxzoom: meta.maxzoom,
        bounds: meta.bounds,
        layers,
    }))
}

/// Get statistics for a source (bounds, zoom range, layer count)
/// Route: GET /api/spatial/stats/{source}
pub(crate) async fn get_spatial_stats(
    State(shared): State<SharedState>,
    Path(source_id): Path<String>,
) -> Result<Json<SpatialStatsResponse>, TileServerError> {
    let state = shared.load();
    let source = match state.sources.get(&source_id) {
        Some(s) => s,
        None => return Err(TileServerError::SourceNotFound(source_id)),
    };

    let meta = source.metadata();
    let layer_count = meta
        .vector_layers
        .as_ref()
        .and_then(|v| v.as_array())
        .map_or(0, |arr| arr.len());

    Ok(Json(SpatialStatsResponse {
        source: meta.id.clone(),
        format: meta.format.extension().to_string(),
        minzoom: meta.minzoom,
        maxzoom: meta.maxzoom,
        bounds: meta.bounds,
        center: meta.center,
        layer_count,
        name: Some(meta.name.clone()),
        description: meta.description.clone(),
        attribution: meta.attribution.clone(),
    }))
}

/// Query features from a source by decoding vector tiles
/// Route: POST /api/spatial/query
///
/// This endpoint decodes MVT/PBF tiles at the requested location and returns
/// features as JSON. For full SQL spatial queries, PostGIS sources are required.
pub(crate) async fn post_spatial_query(
    State(shared): State<SharedState>,
    Json(request): Json<SpatialQueryRequest>,
) -> Result<Json<SpatialQueryResponse>, TileServerError> {
    let state = shared.load();
    let source = match state.sources.get(&request.source) {
        Some(s) => s,
        None => return Err(TileServerError::SourceNotFound(request.source)),
    };

    let meta = source.metadata();

    // Only vector sources support feature queries
    if !meta.format.is_vector() {
        return Err(TileServerError::InvalidTileRequest);
    }

    // Determine tile coordinates from bbox or use center
    let (z, x, y) = if let Some(bbox) = request.bbox {
        // Use center of bbox at requested zoom
        let center_lng = (bbox[0] + bbox[2]) / 2.0;
        let center_lat = (bbox[1] + bbox[3]) / 2.0;
        let zoom = request.zoom.min(meta.maxzoom);
        let (tx, ty) = lng_lat_to_tile(center_lng, center_lat, zoom);
        (zoom, tx, ty)
    } else if let Some(center) = meta.center {
        // Use source center
        let zoom = request.zoom.min(meta.maxzoom);
        let (tx, ty) = lng_lat_to_tile(center[0], center[1], zoom);
        (zoom, tx, ty)
    } else {
        // Default to world view
        (0, 0, 0)
    };

    // Fetch the tile
    let tile_data = source
        .get_tile(z, x, y)
        .await?
        .ok_or(TileServerError::TileNotFound { z, x, y })?;

    // Decompress if needed
    let raw_data = match tile_data.compression {
        sources::TileCompression::Gzip => {
            use std::io::Read;
            let mut decoder = flate2::read::GzDecoder::new(&tile_data.data[..]);
            let mut decompressed = Vec::with_capacity(tile_data.data.len() * 4);
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| TileServerError::MetadataError(format!("gzip decode failed: {e}")))?;
            decompressed
        }
        _ => tile_data.data.to_vec(),
    };

    // Parse MVT protobuf and extract features
    let features = parse_mvt_features(&raw_data, &request.layers, request.limit);
    let total = features.len();

    Ok(Json(SpatialQueryResponse {
        source: request.source,
        features,
        total,
        truncated: total >= request.limit,
    }))
}

/// Convert longitude/latitude to tile coordinates at a given zoom level
#[inline]
fn lng_lat_to_tile(lng: f64, lat: f64, zoom: u8) -> (u32, u32) {
    let n = 2_u32.pow(zoom as u32);
    let x = ((lng + 180.0) / 360.0 * n as f64).floor() as u32;
    let lat_rad = lat.to_radians();
    let y = ((1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n as f64).floor() as u32;
    (x.min(n - 1), y.min(n - 1))
}

/// Extract layer info from vector_layers JSON metadata
fn extract_layer_info(vector_layers: &Option<serde_json::Value>) -> Vec<SpatialLayerInfo> {
    let Some(layers) = vector_layers.as_ref().and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    layers
        .iter()
        .filter_map(|layer| {
            let id = layer.get("id")?.as_str()?.to_string();
            let description = layer
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from);
            let minzoom = layer.get("minzoom").and_then(|v| v.as_u64());
            let maxzoom = layer.get("maxzoom").and_then(|v| v.as_u64());

            let fields = layer
                .get("fields")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .map(|(name, field_type)| SpatialFieldInfo {
                            name: name.clone(),
                            field_type: field_type.as_str().unwrap_or("unknown").to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            Some(SpatialLayerInfo {
                id,
                description,
                minzoom,
                maxzoom,
                fields,
            })
        })
        .collect()
}

/// Parse MVT protobuf tile data and extract features as JSON.
/// This is a lightweight parser — it reads the protobuf wire format directly
/// without pulling in a full protobuf library (already have the data as bytes).
// TODO: implement MVT feature decoding (requires protobuf parsing)
fn parse_mvt_features(
    data: &[u8],
    layer_filter: &Option<Vec<String>>,
    limit: usize,
) -> Vec<SpatialFeature> {
    let _ = (data, layer_filter, limit);
    tracing::warn!("parse_mvt_features: MVT decoding not yet implemented, returning empty");
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_zoom_is_14() {
        assert_eq!(default_zoom(), 14);
    }

    #[test]
    fn default_limit_is_100() {
        assert_eq!(default_limit(), 100);
    }

    #[test]
    fn lng_lat_to_tile_origin_zoom_0_is_zero_zero() {
        let (x, y) = lng_lat_to_tile(0.0, 0.0, 0);
        assert_eq!((x, y), (0, 0));
    }

    #[test]
    fn lng_lat_to_tile_west_hemisphere_zoom_2() {
        let (x, y) = lng_lat_to_tile(-179.0, 0.0, 2);
        assert_eq!(x, 0, "longitude near -180 must map to x=0 at z=2");
        assert!(y <= 3, "y must be within 2^z range at z=2");
    }

    #[test]
    fn lng_lat_to_tile_east_hemisphere_zoom_2() {
        let (x, y) = lng_lat_to_tile(179.0, 0.0, 2);
        assert!(x <= 3);
        assert!(y <= 3);
    }

    #[test]
    fn lng_lat_to_tile_clamps_to_max_tile_index() {
        let (x, y) = lng_lat_to_tile(180.0, -85.0, 2);
        assert!(x < 4);
        assert!(y < 4);
    }

    #[test]
    fn lng_lat_to_tile_zoom_4_known_coords() {
        let (x, y) = lng_lat_to_tile(13.4, 52.5, 4);
        assert_eq!((x, y), (8, 5));
    }

    #[test]
    fn extract_layer_info_returns_empty_for_none() {
        let layers = extract_layer_info(&None);
        assert!(layers.is_empty());
    }

    #[test]
    fn extract_layer_info_returns_empty_for_non_array() {
        let layers = extract_layer_info(&Some(serde_json::json!({})));
        assert!(layers.is_empty());
    }

    #[test]
    fn extract_layer_info_skips_entries_without_id() {
        let v = serde_json::json!([{"description": "no id here"}]);
        let layers = extract_layer_info(&Some(v));
        assert!(layers.is_empty());
    }

    #[test]
    fn extract_layer_info_basic_minimal_id_only() {
        let v = serde_json::json!([{"id": "buildings"}]);
        let layers = extract_layer_info(&Some(v));
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].id, "buildings");
        assert!(layers[0].description.is_none());
        assert!(layers[0].minzoom.is_none());
        assert!(layers[0].maxzoom.is_none());
        assert!(layers[0].fields.is_empty());
    }

    #[test]
    fn extract_layer_info_full_with_fields_and_zoom() {
        let v = serde_json::json!([{
            "id": "roads",
            "description": "all roads",
            "minzoom": 5,
            "maxzoom": 14,
            "fields": {
                "name": "string",
                "lanes": "number",
            }
        }]);
        let layers = extract_layer_info(&Some(v));
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].id, "roads");
        assert_eq!(layers[0].description.as_deref(), Some("all roads"));
        assert_eq!(layers[0].minzoom, Some(5));
        assert_eq!(layers[0].maxzoom, Some(14));
        assert_eq!(layers[0].fields.len(), 2);
    }

    #[test]
    fn extract_layer_info_field_with_non_string_type_falls_back_to_unknown() {
        let v = serde_json::json!([{
            "id": "x",
            "fields": { "weird": 42 }
        }]);
        let layers = extract_layer_info(&Some(v));
        assert_eq!(layers.len(), 1);
        let field = layers[0].fields.first().expect("one field present");
        assert_eq!(field.name, "weird");
        assert_eq!(field.field_type, "unknown");
    }

    #[test]
    fn parse_mvt_features_returns_empty_for_now() {
        let empty: Vec<SpatialFeature> = parse_mvt_features(&[1, 2, 3], &None, 10);
        assert!(empty.is_empty());
    }

    #[test]
    fn parse_mvt_features_with_layer_filter_returns_empty() {
        let empty = parse_mvt_features(&[], &Some(vec!["roads".into()]), 5);
        assert!(empty.is_empty());
    }
}
