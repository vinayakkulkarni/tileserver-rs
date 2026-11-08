//! Data/tile route handlers.
//!
//! Endpoints for listing tile sources, fetching TileJSON metadata,
//! and serving individual vector tiles.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{
        HeaderMap, HeaderValue,
        header::{ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE, VARY},
    },
    response::{IntoResponse, Response},
};

use crate::cache_control;
use crate::error::TileServerError;
use crate::reload::SharedState;
use crate::sources::{self, TileData, TileJson};

#[cfg(feature = "raster")]
use crate::config;

/// Query parameters for data source endpoints
#[derive(Debug, serde::Deserialize, Default)]
pub(crate) struct DataSourceQueryParams {
    /// API key to append to tile URLs
    key: Option<String>,
}

/// Tile request parameters (raw from URL)
#[derive(serde::Deserialize)]
pub(crate) struct TileParams {
    source: String,
    z: u8,
    x: u32,
    y_fmt: String, // e.g., "123.pbf" or "123.mvt"
}

impl TileParams {
    fn parse_y_and_format(&self) -> Option<(u32, &str)> {
        let (y_str, format) = self.y_fmt.rsplit_once('.')?;
        let y = y_str.parse().ok()?;
        Some((y, format))
    }
}

/// Get all available tile sources
/// Route: GET /data.json
/// Query parameters:
/// - `key`: Optional API key to append to tile URLs
pub(crate) async fn get_all_sources(
    State(shared): State<SharedState>,
    Query(query): Query<DataSourceQueryParams>,
) -> Json<Vec<TileJson>> {
    let state = shared.load();
    let sources: Vec<TileJson> = state
        .sources
        .all_metadata()
        .iter()
        .map(|m| m.to_tilejson_with_key(&state.base_url, query.key.as_deref()))
        .collect();

    Json(sources)
}

/// Get TileJSON for a specific source
/// Route: GET /data/{source}
/// Query parameters:
/// - `key`: Optional API key to append to tile URLs
pub(crate) async fn get_source_tilejson(
    State(shared): State<SharedState>,
    Path(source): Path<String>,
    Query(query): Query<DataSourceQueryParams>,
) -> Result<Json<TileJson>, TileServerError> {
    let state = shared.load();

    // Strip .json extension if present
    let source_id = source.strip_suffix(".json").unwrap_or(&source);

    if let Some(members) = resolve_composite_members(&shared, source_id) {
        return composite_tilejson_response(&state, source_id, &members, query.key.as_deref());
    }

    let source_ref = state
        .sources
        .get(source_id)
        .ok_or_else(|| TileServerError::SourceNotFound(source_id.to_string()))?;

    let tilejson = source_ref
        .metadata()
        .to_tilejson_with_key(&state.base_url, query.key.as_deref());
    Ok(Json(tilejson))
}

/// Resolve a source id into composite member ids, or `None` for a plain source.
///
/// A configured `[[composites]]` entry wins first; otherwise a `+`-joined id is
/// parsed ad-hoc. Plain single-source ids return `None` so the existing flow
/// runs untouched.
fn resolve_composite_members(shared: &SharedState, id: &str) -> Option<Vec<String>> {
    if let Some(cfg) = shared.config().composites.iter().find(|c| c.id == id) {
        return Some(cfg.sources.clone());
    }
    if crate::composite::is_composite_id(id) {
        return crate::composite::parse_composite_id(id);
    }
    None
}

/// Build a composite TileJSON from already-resolved member ids.
fn composite_tilejson_response(
    state: &crate::reload::AppState,
    composite_id: &str,
    members: &[String],
    key: Option<&str>,
) -> Result<Json<TileJson>, TileServerError> {
    crate::composite::validate_composite_source_ids(members, |id| state.sources.exists(id))?;

    let metas: Vec<&sources::TileMetadata> = members
        .iter()
        .filter_map(|id| state.sources.get(id).map(|s| s.metadata()))
        .collect();

    Ok(Json(crate::composite::composite_tilejson(
        composite_id,
        &metas,
        &state.base_url,
        key,
    )))
}

pub(crate) async fn get_tile(
    State(shared): State<SharedState>,
    Path(params): Path<TileParams>,
    Query(query): Query<std::collections::HashMap<String, String>>,
    req_headers: HeaderMap,
) -> Result<Response, TileServerError> {
    let state = shared.load();

    let (y, format) = params
        .parse_y_and_format()
        .ok_or(TileServerError::InvalidTileRequest)?;

    // CEL request filter (`?filter=`). Compiled before any fetch so an invalid
    // expression is a 400 without touching the source.
    #[cfg(feature = "cel")]
    let request_filter = query
        .get("filter")
        .map(|expr| crate::cel_filter::CelFilter::compile(expr).map(std::sync::Arc::new))
        .transpose()?;

    if let Some(members) = resolve_composite_members(&shared, &params.source) {
        let response = get_composite_tile(&state, &members, params.z, params.x, y).await?;
        #[cfg(feature = "cel")]
        if let Some(filter) = &request_filter {
            return filter_composite_response(response, filter).await;
        }
        return Ok(response);
    }

    if format == "geojson" {
        return get_tile_as_geojson(&state, &params.source, params.z, params.x, y).await;
    }

    let tile = fetch_source_tile(
        &state.sources,
        &params.source,
        (params.z, params.x, y),
        &query,
    )
    .await?;

    // Band math: when the client passes `?expression=...` and the
    // requested tile is a raster format, parse the expression and
    // apply it in-place on the decoded pixel data before re-encoding
    // as PNG.  This intentionally runs AFTER the source has produced
    // the raw tile bytes and BEFORE MLT transcoding — the two do not
    // interact because band math only applies to raster formats and
    // MLT only to vector formats.
    #[cfg(feature = "raster")]
    let tile = if query.contains_key("expression") {
        // Decode, per-pixel eval and PNG encode are CPU work: keep them off
        // the async worker threads.
        let band_query = query.clone();
        tokio::task::spawn_blocking(move || apply_band_math_if_requested(tile, &band_query))
            .await
            .map_err(|e| TileServerError::RenderError(format!("band math task failed: {e}")))?
    } else {
        tile
    };

    // Every active CEL filter (source preset + request) folds into one hash:
    // filtered variants get their own cache entries.
    #[cfg(feature = "cel")]
    let filter_hash = active_filter_hash(&state.sources, &params.source, request_filter.as_deref());
    #[cfg(not(feature = "cel"))]
    let filter_hash = 0;

    #[cfg(feature = "cel")]
    let tile = match &request_filter {
        Some(filter) => {
            filter_request_tile(
                &state.sources,
                TileId {
                    source: &params.source,
                    z: params.z,
                    x: params.x,
                    y,
                    filter: filter_hash,
                },
                tile,
                filter,
            )
            .await?
        }
        None => tile,
    };

    // A filter that removed every feature yields an empty tile: 204.
    if filter_hash != 0 && tile.format.is_vector() && tile.data.is_empty() {
        return Ok(axum::http::StatusCode::NO_CONTENT.into_response());
    }

    #[cfg(feature = "mlt")]
    let tile = transcode_if_requested(tile, format, (&params.source, params.z, params.x, y)).await;

    let accept_encoding = req_headers
        .get(ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok());
    let compression_cfg = shared.config().compression.clone();
    let tile = negotiate_tile_encoding(
        &state.sources,
        &compression_cfg,
        TileId {
            source: &params.source,
            z: params.z,
            x: params.x,
            y,
            filter: filter_hash,
        },
        tile,
        accept_encoding,
    )
    .await?;

    Ok(tile_response(tile))
}

/// Fetch the tile from its source. PostGIS function sources get the query
/// string as parameters; raster sources honour `?resampling=` (and out-db
/// rasters get the query string too).
async fn fetch_source_tile(
    sources: &sources::SourceManager,
    source: &str,
    (z, x, y): (u8, u32, u32),
    query: &std::collections::HashMap<String, String>,
) -> Result<TileData, TileServerError> {
    #[cfg(feature = "postgres")]
    if sources.is_postgres_function_source(source) {
        let params = serde_json::to_value(query).unwrap_or_default();
        return sources
            .get_vector_tile_with_query_params(source, z, x, y, &params)
            .await?
            .ok_or(TileServerError::TileNotFound { z, x, y });
    }
    #[cfg(feature = "raster")]
    let tile = {
        let resampling = query
            .get("resampling")
            .and_then(|s| s.parse::<config::ResamplingMethod>().ok());
        #[cfg(feature = "postgres")]
        let params = sources
            .is_outdb_raster_source(source)
            .then(|| serde_json::to_value(query).unwrap_or_default());
        #[cfg(not(feature = "postgres"))]
        let params: Option<serde_json::Value> = None;
        sources
            .get_raster_tile_with_params(source, z, x, y, 256, resampling, params)
            .await?
    };
    #[cfg(not(feature = "raster"))]
    let tile = {
        let _ = query;
        sources.get_tile(source, z, x, y).await?
    };
    tile.ok_or(TileServerError::TileNotFound { z, x, y })
}

/// One hash over the source preset and the request filter; 0 when neither.
#[cfg(feature = "cel")]
fn active_filter_hash(
    sources: &sources::SourceManager,
    source: &str,
    request_filter: Option<&crate::cel_filter::CelFilter>,
) -> u64 {
    let preset = sources.preset_filter(source);
    let active: Vec<&crate::cel_filter::CelFilter> = preset
        .map(AsRef::as_ref)
        .into_iter()
        .chain(request_filter)
        .collect();
    crate::cel_filter::combined_hash(&active)
}

/// Transcode between vector formats (MVT ↔ MLT) when the URL asks for a
/// different one. On a transcode error the original tile is served: it is
/// still valid, only the conversion failed.
#[cfg(feature = "mlt")]
async fn transcode_if_requested(
    tile: TileData,
    format: &str,
    (source, z, x, y): (&str, u8, u32, u32),
) -> TileData {
    let Some(target) = format.parse::<crate::sources::TileFormat>().ok() else {
        return tile;
    };
    if tile.format == target || !tile.format.is_vector() || !target.is_vector() {
        return tile;
    }
    match crate::transcode::transcode_tile_blocking(tile.clone(), target).await {
        Ok(transcoded) => transcoded,
        Err(e) => {
            tracing::warn!(
                "transcoding {:?} -> {:?} failed for {}/{}/{}/{}, serving original tile: {}",
                tile.format,
                target,
                source,
                z,
                x,
                y,
                e
            );
            tile
        }
    }
}

/// Tile body with content type, cache headers, `Vary` and encoding.
fn tile_response(tile: TileData) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(tile.format.content_type()),
    );
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());
    // Vary is mandatory on every tile response: without it CDNs may serve a
    // wrong-encoding cached copy to a client that does not accept it.
    headers.insert(VARY, HeaderValue::from_static("Accept-Encoding"));
    if let Some(encoding) = tile.compression.content_encoding() {
        headers.insert(CONTENT_ENCODING, HeaderValue::from_static(encoding));
    }
    (headers, tile.data).into_response()
}

struct TileId<'a> {
    source: &'a str,
    z: u8,
    x: u32,
    y: u32,
    /// Combined CEL filter hash (`0` = unfiltered); part of the cache key.
    filter: u64,
}

/// Apply a request CEL filter, caching the filtered tile under
/// `(source, z, x, y, identity, filter hash)`.
#[cfg(feature = "cel")]
async fn filter_request_tile(
    sources: &sources::SourceManager,
    id: TileId<'_>,
    tile: sources::TileData,
    filter: &std::sync::Arc<crate::cel_filter::CelFilter>,
) -> Result<sources::TileData, TileServerError> {
    let key = sources.cache().map(|_| crate::cache::TileCacheKey {
        source_id: id.source.into(),
        z: id.z,
        x: id.x,
        y: id.y,
        encoding: sources::TileCompression::None,
        filter: id.filter,
    });
    if let (Some(cache), Some(key)) = (sources.cache(), key.as_ref())
        && let Some(hit) = cache.get(key).await
    {
        return Ok(hit);
    }
    let filtered =
        crate::cel_filter::apply_blocking(tile, vec![std::sync::Arc::clone(filter)]).await?;
    if let (Some(cache), Some(key)) = (sources.cache(), key) {
        cache.insert(key, filtered.clone()).await;
    }
    Ok(filtered)
}

/// Apply a request CEL filter to a merged composite tile response.
#[cfg(feature = "cel")]
async fn filter_composite_response(
    response: Response,
    filter: &std::sync::Arc<crate::cel_filter::CelFilter>,
) -> Result<Response, TileServerError> {
    let (parts, body) = response.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|e| TileServerError::RenderError(format!("composite body: {e}")))?;
    let tile = sources::TileData {
        data: bytes,
        format: sources::TileFormat::Pbf,
        compression: sources::TileCompression::None,
    };
    let filtered =
        crate::cel_filter::apply_blocking(tile, vec![std::sync::Arc::clone(filter)]).await?;
    if filtered.data.is_empty() {
        return Ok(axum::http::StatusCode::NO_CONTENT.into_response());
    }
    Ok((parts.headers, filtered.data).into_response())
}

/// Negotiate the response `Content-Encoding` for `tile` and re-encode if needed.
///
/// Passthrough (target == source encoding) returns the tile untouched. Otherwise
/// the re-encoded variant is looked up in / stored to the global tile cache keyed
/// by `(source, z, x, y, target)`, so the re-encode cost is paid once per
/// `(tile, encoding)` pair and repeat requests hit the cache.
async fn negotiate_tile_encoding(
    sources: &sources::SourceManager,
    cfg: &crate::config::CompressionConfig,
    id: TileId<'_>,
    tile: sources::TileData,
    accept_encoding: Option<&str>,
) -> Result<sources::TileData, TileServerError> {
    let target =
        crate::compression::negotiate(accept_encoding, tile.compression, tile.data.len(), cfg);
    let encoding_label = |c: sources::TileCompression| c.content_encoding().unwrap_or("identity");

    if target == tile.compression {
        crate::metrics::compression_recorded(
            id.source,
            encoding_label(target),
            crate::metrics::CompressionAction::Passthrough,
        );
        return Ok(tile);
    }

    let cache = sources.cache();
    let key = cache.map(|_| crate::cache::TileCacheKey {
        source_id: id.source.into(),
        z: id.z,
        x: id.x,
        y: id.y,
        encoding: target,
        filter: id.filter,
    });

    if let (Some(cache), Some(key)) = (cache, key.as_ref())
        && let Some(hit) = cache.get(key).await
    {
        crate::metrics::compression_recorded(
            id.source,
            encoding_label(target),
            crate::metrics::CompressionAction::TranscodeHit,
        );
        return Ok(hit);
    }

    let recoded = crate::compression::recode(&tile, target, cfg)?;
    crate::metrics::compression_recorded(
        id.source,
        encoding_label(target),
        crate::metrics::CompressionAction::TranscodeMiss,
    );

    if let (Some(cache), Some(key)) = (cache, key) {
        cache.insert(key, recoded.clone()).await;
    }
    Ok(recoded)
}

/// Serve a merged MVT tile from resolved composite member ids (#601).
///
/// Members are validated up front, fetched concurrently, decompressed
/// (gzip only), transcoded MLT->MVT when needed, then merged into one PBF.
/// A member that returns no tile is skipped; if every member misses the
/// response is a valid empty MVT with `200 OK`. Raster members are rejected
/// with `400` — composites are vector-only.
async fn get_composite_tile(
    state: &crate::reload::AppState,
    members: &[String],
    z: u8,
    x: u32,
    y: u32,
) -> Result<Response, TileServerError> {
    use crate::composite;

    composite::validate_composite_source_ids(members, |id| state.sources.exists(id))?;

    let fetches = members.iter().map(|id| state.sources.get_tile(id, z, x, y));
    let results = futures::future::join_all(fetches).await;

    let mut tiles = Vec::with_capacity(results.len());
    for tile in results {
        let Some(tile) = tile? else {
            continue;
        };
        if !tile.format.is_vector() {
            return Err(TileServerError::InvalidTileRequest);
        }
        tiles.push(tile);
    }

    // Decode, transcode, merge and encode are CPU work: keep them off the runtime.
    let bytes = tokio::task::spawn_blocking(move || merge_composite_tiles(tiles))
        .await
        .map_err(|e| TileServerError::Internal(anyhow::anyhow!("composite task failed: {e}")))??;

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(sources::TileFormat::Pbf.content_type()),
    );
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());
    Ok((headers, bytes).into_response())
}

/// Decode every member tile and merge them into one MVT PBF.
fn merge_composite_tiles(tiles: Vec<sources::TileData>) -> Result<bytes::Bytes, TileServerError> {
    use crate::composite;

    let mut all_layers = Vec::new();
    for tile in tiles {
        let (data, compression) = to_mvt_bytes(tile)?;
        let raw = match compression {
            sources::TileCompression::Gzip => composite::decompress_gzip(&data)?,
            _ => data,
        };
        all_layers.extend(composite::decode_mvt_layers(&raw)?);
    }
    Ok(composite::encode_mvt_pbf(&composite::merge_mvt_layers(
        all_layers,
    )))
}

/// Reduce a composite member tile to raw MVT bytes + its compression marker,
/// transcoding MLT sources to MVT first when the `mlt` feature is present.
fn to_mvt_bytes(
    tile: sources::TileData,
) -> Result<(Vec<u8>, sources::TileCompression), TileServerError> {
    #[cfg(feature = "mlt")]
    if tile.format == sources::TileFormat::Mlt {
        let transcoded = crate::transcode::transcode_tile(&tile, sources::TileFormat::Pbf)?;
        return Ok((transcoded.data.to_vec(), transcoded.compression));
    }
    Ok((tile.data.to_vec(), tile.compression))
}

/// Get a tile as GeoJSON (helper function)
async fn get_tile_as_geojson(
    state: &crate::reload::AppState,
    source_id: &str,
    z: u8,
    x: u32,
    y: u32,
) -> Result<Response, TileServerError> {
    use geozero::ProcessToJson;
    use geozero::mvt::{Message, Tile};
    use sources::TileCompression;

    let source = state
        .sources
        .get(source_id)
        .ok_or_else(|| TileServerError::SourceNotFound(source_id.to_string()))?;

    // Check if source is vector format
    if source.metadata().format != sources::TileFormat::Pbf {
        return Err(TileServerError::RenderError(
            "GeoJSON conversion only supported for vector tiles (PBF)".to_string(),
        ));
    }

    let tile = state
        .sources
        .get_tile(source_id, z, x, y)
        .await?
        .ok_or(TileServerError::TileNotFound { z, x, y })?;

    // Decompress if needed
    let raw_data = match tile.compression {
        TileCompression::Gzip => crate::compression::decode_gzip(&tile.data)
            .map_err(|e| TileServerError::RenderError(format!("failed to decompress tile: {e}")))?,
        TileCompression::None => tile.data.to_vec(),
        _ => {
            return Err(TileServerError::RenderError(format!(
                "unsupported compression: {:?}",
                tile.compression
            )));
        }
    };

    // Parse MVT tile using prost
    let mvt_tile = Tile::decode(raw_data.as_slice())
        .map_err(|e| TileServerError::RenderError(format!("failed to decode MVT tile: {}", e)))?;

    // Convert each layer to GeoJSON and combine into a FeatureCollection
    let mut all_features: Vec<serde_json::Value> = Vec::with_capacity(mvt_tile.layers.len() * 64);

    for mut layer in mvt_tile.layers {
        // Each layer implements GeozeroDatasource which can convert to JSON
        if let Ok(layer_json) = layer.to_json()
            && let Ok(fc) = serde_json::from_str::<serde_json::Value>(&layer_json)
            && let Some(features) = fc.get("features").and_then(|f| f.as_array())
        {
            // Add layer name to each feature's properties
            for feature in features {
                let mut feature = feature.clone();
                if let Some(props) = feature.get_mut("properties")
                    && let Some(props_obj) = props.as_object_mut()
                {
                    props_obj.insert(
                        "_layer".to_string(),
                        serde_json::Value::String(layer.name.clone()),
                    );
                }
                all_features.push(feature);
            }
        }
    }

    // Build final FeatureCollection
    let geojson = serde_json::json!({
        "type": "FeatureCollection",
        "features": all_features
    });

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/geo+json"),
    );
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());

    Ok((headers, geojson.to_string()).into_response())
}

/// Apply a band-math expression (if provided in the query) to a
/// freshly-rendered raster tile.
///
/// Fails open: on any decode/parse/evaluate error the original tile
/// is returned unchanged so a broken expression does not blank the
/// map.  The exact failure is logged at WARN so operators can fix it.
#[cfg(feature = "raster")]
fn apply_band_math_if_requested(
    tile: sources::TileData,
    query: &std::collections::HashMap<String, String>,
) -> sources::TileData {
    use crate::raster::{decode, encode, expression};

    let Some(expr_str) = query.get("expression") else {
        return tile;
    };
    if !matches!(
        tile.format,
        sources::TileFormat::Png | sources::TileFormat::Jpeg | sources::TileFormat::Webp
    ) {
        return tile;
    }

    let raster = match decode::from_bytes(&tile.data) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "band math: failed to decode tile; serving original");
            return tile;
        }
    };

    let parsed = match expression::ParsedExpression::parse(expr_str, raster.band_count()) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(expression = %expression::log_preview(expr_str), error = %e, "band math: parse failed; serving original");
            return tile;
        }
    };

    let result = match expression::apply(&parsed, &raster) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(expression = %expression::log_preview(expr_str), error = %e, "band math: eval failed; serving original");
            return tile;
        }
    };

    let png = match encode::to_png(&result) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "band math: encode failed; serving original");
            return tile;
        }
    };

    sources::TileData {
        data: png.into(),
        format: sources::TileFormat::Png,
        compression: sources::TileCompression::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tp(y_fmt: &str) -> TileParams {
        TileParams {
            source: "src".into(),
            z: 0,
            x: 0,
            y_fmt: y_fmt.into(),
        }
    }

    #[test]
    fn parse_y_and_format_basic_pbf() {
        let p = tp("123.pbf");
        let (y, fmt) = p.parse_y_and_format().expect("parses");
        assert_eq!(y, 123);
        assert_eq!(fmt, "pbf");
    }

    #[test]
    fn parse_y_and_format_rejects_missing_dot() {
        let p = tp("notadot");
        assert!(p.parse_y_and_format().is_none());
    }

    #[test]
    fn parse_y_and_format_rejects_non_numeric_y() {
        let p = tp("abc.pbf");
        assert!(p.parse_y_and_format().is_none());
    }
}

#[cfg(all(test, feature = "raster"))]
mod band_math_tests {
    use super::apply_band_math_if_requested;
    use crate::raster::{RasterImage, encode};
    use crate::sources::{self, TileCompression, TileData, TileFormat};
    use ndarray::array;
    use std::collections::HashMap;

    fn tile(data: Vec<u8>, format: TileFormat) -> TileData {
        TileData {
            data: data.into(),
            format,
            compression: TileCompression::None,
        }
    }

    fn valid_png() -> Vec<u8> {
        let pixels = array![[[10.0_f32, 20.0], [30.0, 40.0]]];
        let raster = RasterImage::from_opaque(pixels, None);
        encode::to_png(&raster).expect("encode tiny png")
    }

    #[test]
    fn band_math_no_expression_returns_unchanged() {
        let original = valid_png();
        let input = tile(original.clone(), TileFormat::Png);
        let query: HashMap<String, String> = HashMap::new();

        let out = apply_band_math_if_requested(input, &query);

        assert_eq!(out.data.as_ref(), original.as_slice());
        assert_eq!(out.format, TileFormat::Png);
    }

    #[test]
    fn band_math_non_raster_format_returns_unchanged() {
        let original = b"\x1a\x00protobuf-ish".to_vec();
        let input = tile(original.clone(), TileFormat::Pbf);
        let mut query = HashMap::new();
        query.insert("expression".to_string(), "b1".to_string());

        let out = apply_band_math_if_requested(input, &query);

        assert_eq!(out.data.as_ref(), original.as_slice());
        assert_eq!(out.format, TileFormat::Pbf);
    }

    #[test]
    fn band_math_decode_error_fails_open() {
        let garbage = vec![0xFF, 0x00, 0x13, 0x37, 0x42];
        let input = tile(garbage.clone(), TileFormat::Png);
        let mut query = HashMap::new();
        query.insert("expression".to_string(), "b1".to_string());

        let out = apply_band_math_if_requested(input, &query);

        assert_eq!(out.data.as_ref(), garbage.as_slice());
        assert_eq!(out.format, TileFormat::Png);
    }

    #[test]
    fn band_math_parse_error_fails_open() {
        let original = valid_png();
        let input = tile(original.clone(), TileFormat::Png);
        let mut query = HashMap::new();
        query.insert("expression".to_string(), "b1 +* )(".to_string());

        let out = apply_band_math_if_requested(input, &query);

        assert_eq!(out.data.as_ref(), original.as_slice());
        assert_eq!(out.format, TileFormat::Png);
    }

    #[test]
    fn band_math_valid_expression_reencodes() {
        let original = valid_png();
        let input = tile(original.clone(), TileFormat::Webp);
        let mut query = HashMap::new();
        query.insert("expression".to_string(), "b1 * 2".to_string());

        let out = apply_band_math_if_requested(input, &query);

        assert_eq!(out.format, sources::TileFormat::Png);
        assert!(!out.data.is_empty());
        assert_ne!(out.data.as_ref(), original.as_slice());
    }
}

#[cfg(all(test, feature = "cel"))]
mod cel_route_tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use bytes::Bytes;
    use geozero::mvt::{Message, Tile, tile};
    use tower::ServiceExt;

    use crate::cel_filter::CelFilter;
    use crate::config::CelFilterConfig;
    use crate::reload::{AppState, ReloadController, ReloadMeta, RuntimeSettings, SharedState};
    use crate::sources::{
        SourceManager, TileCompression, TileData, TileFormat, TileMetadata, TileSource,
    };
    use crate::styles::StyleManager;

    struct MvtSource {
        meta: TileMetadata,
        tile: Bytes,
    }

    #[async_trait]
    impl TileSource for MvtSource {
        async fn get_tile(
            &self,
            _z: u8,
            _x: u32,
            _y: u32,
        ) -> crate::error::Result<Option<TileData>> {
            Ok(Some(TileData {
                data: self.tile.clone(),
                format: TileFormat::Pbf,
                compression: TileCompression::None,
            }))
        }

        fn metadata(&self) -> &TileMetadata {
            &self.meta
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    /// One `landuse` layer: ids 0..3 with (type, height).
    fn landuse_tile() -> Bytes {
        let rows = [
            ("residential", 10),
            ("commercial", 120),
            ("park", 0),
            ("residential", 80),
        ];
        let mut values = Vec::with_capacity(rows.len() * 2);
        let mut features = Vec::with_capacity(rows.len());
        for (id, (kind, height)) in rows.iter().enumerate() {
            values.push(tile::Value {
                string_value: Some((*kind).to_string()),
                ..Default::default()
            });
            values.push(tile::Value {
                int_value: Some(*height),
                ..Default::default()
            });
            let base = u32::try_from(id * 2).expect("index");
            features.push(tile::Feature {
                id: Some(id as u64),
                tags: vec![0, base, 1, base + 1],
                r#type: Some(1),
                geometry: vec![9, 50, 34],
            });
        }
        let layer = tile::Layer {
            version: 2,
            name: "landuse".to_string(),
            features,
            keys: vec!["type".to_string(), "height".to_string()],
            values,
            extent: Some(4096),
        };
        Bytes::from(
            Tile {
                layers: vec![layer],
            }
            .encode_to_vec(),
        )
    }

    fn router(preset: Option<&str>) -> Router {
        let source = MvtSource {
            meta: TileMetadata {
                id: "land".to_string(),
                name: "land".to_string(),
                description: None,
                attribution: None,
                format: TileFormat::Pbf,
                minzoom: 0,
                maxzoom: 14,
                bounds: None,
                center: None,
                vector_layers: None,
            },
            tile: landuse_tile(),
        };
        let mut map: HashMap<String, Arc<dyn TileSource>> = HashMap::new();
        map.insert("land".to_string(), Arc::new(source));
        let mut filters = HashMap::new();
        if let Some(expression) = preset {
            let filter = CelFilter::from_config(&CelFilterConfig {
                expression: expression.to_string(),
                applies_to_layers: None,
            })
            .expect("preset compiles");
            filters.insert("land".to_string(), Arc::new(filter));
        }
        let manager = SourceManager::from_sources(map).with_filters(filters);
        let state = AppState {
            sources: Arc::new(manager),
            styles: Arc::new(StyleManager::new()),
            renderer: None,
            base_url: "http://localhost:8080".to_string(),
            render_base_url: "http://localhost:8080".to_string(),
            ui_enabled: false,
            fonts_dir: None,
            files_dir: None,
            upload_dir: None,
        };
        let meta = ReloadMeta {
            config_hash: "test".to_string(),
            loaded_at_unix: 0,
            loaded_sources: 1,
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
        let controller = Arc::new(ReloadController::new(
            state,
            meta,
            crate::config::Config::default(),
            None,
            runtime,
        ));
        Router::new()
            .route(
                "/data/{source}/{z}/{x}/{y_fmt}",
                axum::routing::get(super::get_tile),
            )
            .with_state(SharedState::new(controller))
    }

    async fn get(router: Router, uri: &str) -> (StatusCode, Vec<u64>) {
        let response = router
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        if status != StatusCode::OK {
            return (status, Vec::new());
        }
        let tile = Tile::decode(body.as_ref()).expect("MVT body");
        let ids = tile
            .layers
            .iter()
            .flat_map(|l| l.features.iter().filter_map(|f| f.id))
            .collect();
        (status, ids)
    }

    #[tokio::test]
    async fn no_filter_returns_every_feature() {
        assert_eq!(
            get(router(None), "/data/land/1/0/0.pbf").await,
            (StatusCode::OK, vec![0, 1, 2, 3])
        );
    }

    #[tokio::test]
    async fn request_filter_keeps_matching_features() {
        assert_eq!(
            get(
                router(None),
                "/data/land/1/0/0.pbf?filter=type%3D%3D'residential'"
            )
            .await,
            (StatusCode::OK, vec![0, 3])
        );
    }

    #[tokio::test]
    async fn invalid_filter_is_bad_request() {
        let (status, _) = get(router(None), "/data/land/1/0/0.pbf?filter=type%3D%3D").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let (status, _) = get(router(None), "/data/land/1/0/0.pbf?filter=dyn(height)%3E1").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn filter_matching_nothing_is_no_content() {
        let (status, _) = get(router(None), "/data/land/1/0/0.pbf?filter=height%3E1000").await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn preset_always_applies() {
        assert_eq!(
            get(router(Some("height > 50")), "/data/land/1/0/0.pbf").await,
            (StatusCode::OK, vec![1, 3])
        );
    }

    #[tokio::test]
    async fn preset_and_request_compose_with_and() {
        assert_eq!(
            get(
                router(Some("height > 50")),
                "/data/land/1/0/0.pbf?filter=type%3D%3D'residential'"
            )
            .await,
            (StatusCode::OK, vec![3])
        );
    }
}
