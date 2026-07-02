//! Style route handlers.
//!
//! Endpoints for listing styles, serving style JSON, sprites,
//! style TileJSON, and WMTS capabilities.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{
        HeaderMap, HeaderValue,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};

use crate::cache_control;
use crate::error::TileServerError;
use crate::reload::SharedState;
use crate::styles::{self, StyleInfo, UrlQueryParams};
use crate::wmts;

use super::RasterTileJson;

/// Query parameters for styles list endpoint
#[derive(Debug, serde::Deserialize, Default)]
pub(super) struct StylesQueryParams {
    /// API key to append to style URLs
    key: Option<String>,
}

/// Get all available styles
/// Route: GET /styles.json
/// Query parameters:
/// - `key`: Optional API key to append to style URLs
pub(crate) async fn get_all_styles(
    State(shared): State<SharedState>,
    Query(query): Query<StylesQueryParams>,
) -> Json<Vec<StyleInfo>> {
    let state = shared.load();
    Json(
        state
            .styles
            .all_infos_with_key(&state.base_url, query.key.as_deref()),
    )
}

/// Query parameters for style.json endpoint
#[derive(Debug, serde::Deserialize, Default)]
pub(super) struct StyleQueryParams {
    /// API key to forward to all URLs in the style
    key: Option<String>,
}

/// Get style.json for a specific style
/// Returns the style with all relative URLs rewritten to absolute URLs
/// Query parameters (like `?key=API_KEY`) are forwarded to all rewritten URLs
pub(crate) async fn get_style_json(
    State(shared): State<SharedState>,
    Path(style_id): Path<String>,
    Query(query): Query<StyleQueryParams>,
) -> Result<Json<serde_json::Value>, TileServerError> {
    let state = shared.load();

    let Some(style) = state.styles.get(&style_id) else {
        return auto_generate_style(&state, &style_id, query.key.as_deref());
    };

    // Build query params to forward to rewritten URLs
    let url_params = UrlQueryParams::with_key(query.key);

    // Rewrite relative URLs to absolute URLs for external clients
    let rewritten_style = styles::rewrite_style_for_api(
        &style.style_json,
        &state.base_url,
        &url_params,
        &state.sources,
    );

    Ok(Json(rewritten_style))
}

/// Fall back to an auto-generated MapLibre style when no `[[styles]]` entry
/// matches but a vector source with the same id exists (#710). Raster and
/// unknown-format sources still yield `StyleNotFound`.
fn auto_generate_style(
    state: &crate::reload::AppState,
    style_id: &str,
    key: Option<&str>,
) -> Result<Json<serde_json::Value>, TileServerError> {
    match state.sources.get(style_id) {
        Some(source) if source.metadata().format.is_vector() => {
            Ok(Json(crate::stylegen::generate_style_for_vector_source(
                source.metadata(),
                &state.base_url,
                key,
            )))
        }
        _ => Err(TileServerError::StyleNotFound(style_id.to_string())),
    }
}

/// Get TileJSON for raster tiles of a style
/// Query parameters for style TileJSON endpoint
#[derive(Debug, serde::Deserialize, Default)]
pub(super) struct StyleTileJsonQueryParams {
    /// API key to append to tile URLs
    key: Option<String>,
}

/// Get TileJSON for raster tiles of a style
/// Route: GET /styles/{style}.json
/// Query parameters:
/// - `key`: Optional API key to append to tile URLs
pub(crate) async fn get_style_tilejson(
    State(shared): State<SharedState>,
    Path(style_json): Path<String>,
    Query(query): Query<StyleTileJsonQueryParams>,
) -> Result<Json<RasterTileJson>, TileServerError> {
    let state = shared.load();

    // Only handle requests ending with .json
    let style_id = match style_json.strip_suffix(".json") {
        Some(id) => id,
        None => return Err(TileServerError::StyleNotFound(style_json)),
    };

    let style = match state.styles.get(style_id) {
        Some(s) => s,
        None => return Err(TileServerError::StyleNotFound(style_id.to_string())),
    };

    // Build raster tile URL template with optional key
    let key_query = query
        .key
        .as_ref()
        .map(|k| format!("?key={}", urlencoding::encode(k)))
        .unwrap_or_default();

    let tile_url = format!(
        "{}/styles/{}/{{z}}/{{x}}/{{y}}.png{}",
        state.base_url, style_id, key_query
    );

    Ok(Json(RasterTileJson {
        tilejson: "3.0.0",
        name: style.name.clone(),
        tiles: vec![tile_url],
        minzoom: 0,
        maxzoom: 22,
        attribution: None,
    }))
}

/// Sprite request parameters
#[derive(serde::Deserialize)]
pub(super) struct SpriteParams {
    style: String,
    sprite_file: String, // e.g., "sprite.png", "sprite@2x.json", "sprite.json"
}

/// Get sprite image or metadata for a style
/// Route: GET /styles/{style}/sprite[@{scale}x].{format}
pub(crate) async fn get_sprite(
    State(shared): State<SharedState>,
    Path(params): Path<SpriteParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    // Security: Strict validation of sprite file name
    // Only allow: sprite.png, sprite.json, sprite@2x.png, sprite@2x.json, sprite@3x.png, etc.
    if !params.sprite_file.starts_with("sprite") {
        return Err(TileServerError::InvalidTileRequest);
    }

    // Security: Reject any path traversal attempts
    if params.sprite_file.contains("..")
        || params.sprite_file.contains('/')
        || params.sprite_file.contains('\\')
    {
        return Err(TileServerError::InvalidTileRequest);
    }

    // Security: Validate sprite file matches expected pattern
    // Valid patterns: sprite.png, sprite.json, sprite@2x.png, sprite@2x.json, sprite@3x.png, etc.
    let valid_extensions = [".png", ".json"];
    let has_valid_extension = valid_extensions
        .iter()
        .any(|ext| params.sprite_file.ends_with(ext));
    if !has_valid_extension {
        return Err(TileServerError::InvalidTileRequest);
    }

    // Get style to find its directory
    let style = match state.styles.get(&params.style) {
        Some(s) => s,
        None => return Err(TileServerError::StyleNotFound(params.style)),
    };

    // Get the style directory (parent of style.json)
    let style_dir = style
        .path
        .parent()
        .ok_or_else(|| TileServerError::StyleNotFound(style.id.clone()))?;

    // Build path to sprite file
    let sprite_path = style_dir.join(&params.sprite_file);

    // Read the sprite file
    let data = tokio::fs::read(&sprite_path).await.map_err(|e| {
        tracing::debug!("Sprite file not found: {} ({})", sprite_path.display(), e);
        TileServerError::SpriteNotFound(params.sprite_file.clone())
    })?;

    // Determine content type
    let content_type = if params.sprite_file.ends_with(".json") {
        "application/json"
    } else if params.sprite_file.ends_with(".png") {
        "image/png"
    } else {
        "application/octet-stream"
    };

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());

    Ok((headers, data).into_response())
}

/// Query parameters for WMTS endpoint
#[derive(Debug, serde::Deserialize, Default)]
pub(super) struct WmtsQueryParams {
    /// API key to include in all URLs
    key: Option<String>,
}

/// Get WMTS GetCapabilities document for a style
/// Route: GET /styles/{style}/wmts.xml
/// Query parameters:
/// - `key`: Optional API key to append to all tile URLs (e.g., `?key=my_api_key`)
pub(crate) async fn get_wmts_capabilities(
    State(shared): State<SharedState>,
    Path(style_id): Path<String>,
    Query(query): Query<WmtsQueryParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let style = match state.styles.get(&style_id) {
        Some(s) => s,
        None => return Err(TileServerError::StyleNotFound(style_id)),
    };

    // Generate WMTS capabilities XML with optional key
    let xml = wmts::generate_wmts_capabilities(
        &state.base_url,
        &style.id,
        &style.name,
        0,  // minzoom
        22, // maxzoom
        query.key.as_deref(),
    );

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/xml"));
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );

    Ok((headers, xml).into_response())
}
