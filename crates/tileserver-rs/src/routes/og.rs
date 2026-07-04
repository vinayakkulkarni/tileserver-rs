//! OpenGraph / social-card image route handler (`GET /og/{style}`).
//!
//! Reuses the native MapLibre renderer to produce a social-share image
//! (default 1200x630 PNG). Gated by `disable_render` because it requires the
//! native renderer, matching `/styles/{style}/static/...`.

use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::{
        HeaderMap, HeaderValue,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};

use super::render::build_static_options;
use crate::error::TileServerError;
use crate::reload::SharedState;
use crate::render::{ImageFormat, StaticQueryParams, StaticType};
use crate::styles;

/// Default social-card width (matches the OpenGraph image spec).
const DEFAULT_WIDTH: u32 = 1200;
/// Default social-card height (matches the OpenGraph image spec).
const DEFAULT_HEIGHT: u32 = 630;
/// Default zoom when a `center` is given without explicit zoom.
const DEFAULT_ZOOM: f64 = 10.0;

/// Typed, validated `/og` query parameters.
struct OgParams {
    static_type: StaticType,
    width: u32,
    height: u32,
    format: ImageFormat,
}

/// Parse a finite `f64`, rejecting NaN and infinities.
fn parse_finite(s: &str) -> Option<f64> {
    let v: f64 = s.trim().parse().ok()?;
    if v.is_finite() { Some(v) } else { None }
}

/// Parse `bounds=min_lng,min_lat,max_lng,max_lat` into a [`StaticType::BoundingBox`].
fn parse_bounds(raw: &str) -> Result<StaticType, TileServerError> {
    let parts: Vec<&str> = raw.split(',').collect();
    if parts.len() != 4 {
        return Err(TileServerError::InvalidTileRequest);
    }
    let mut vals = [0.0_f64; 4];
    for (slot, part) in vals.iter_mut().zip(parts.iter()) {
        *slot = parse_finite(part).ok_or(TileServerError::InvalidTileRequest)?;
    }
    let [min_lon, min_lat, max_lon, max_lat] = vals;
    if min_lon > max_lon || min_lat > max_lat {
        return Err(TileServerError::InvalidTileRequest);
    }
    Ok(StaticType::BoundingBox {
        min_lon,
        min_lat,
        max_lon,
        max_lat,
    })
}

/// Parse `center=lat,lng` (+ optional `zoom`) into a [`StaticType::Center`].
fn parse_center(raw: &str, zoom: f64) -> Result<StaticType, TileServerError> {
    let mut parts = raw.split(',');
    let (Some(lat_s), Some(lng_s), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(TileServerError::InvalidTileRequest);
    };
    let lat = parse_finite(lat_s).ok_or(TileServerError::InvalidTileRequest)?;
    let lon = parse_finite(lng_s).ok_or(TileServerError::InvalidTileRequest)?;
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return Err(TileServerError::InvalidTileRequest);
    }
    Ok(StaticType::Center {
        lon,
        lat,
        zoom,
        bearing: None,
        pitch: None,
    })
}

/// Parse the raw query map into typed, validated [`OgParams`].
///
/// # Errors
///
/// Returns [`TileServerError::InvalidTileRequest`] (400) when neither `center`
/// nor `bounds` is present, when a coordinate fails to parse, or when `format`
/// is not one of `png`/`jpg`/`jpeg`/`webp`.
fn parse_og_query(raw: &HashMap<String, String>) -> Result<OgParams, TileServerError> {
    let format = match raw.get("format") {
        Some(f) => f
            .parse::<ImageFormat>()
            .map_err(|()| TileServerError::InvalidTileRequest)?,
        None => ImageFormat::Png,
    };

    let width = match raw.get("width") {
        Some(w) => w.parse().map_err(|_| TileServerError::InvalidTileRequest)?,
        None => DEFAULT_WIDTH,
    };
    let height = match raw.get("height") {
        Some(h) => h.parse().map_err(|_| TileServerError::InvalidTileRequest)?,
        None => DEFAULT_HEIGHT,
    };

    let zoom = match raw.get("zoom") {
        Some(z) => parse_finite(z).ok_or(TileServerError::InvalidTileRequest)?,
        None => DEFAULT_ZOOM,
    };

    let static_type = if let Some(bounds) = raw.get("bounds") {
        parse_bounds(bounds)?
    } else if let Some(center) = raw.get("center") {
        parse_center(center, zoom)?
    } else {
        return Err(TileServerError::InvalidTileRequest);
    };

    Ok(OgParams {
        static_type,
        width,
        height,
        format,
    })
}

/// Serve a rendered OpenGraph / social-card image for a style.
///
/// # Errors
///
/// - [`TileServerError::InvalidTileRequest`] (400) on bad query params.
/// - [`TileServerError::StyleNotFound`] (404) for an unknown style.
/// - [`TileServerError::RenderError`] (500) when the native renderer is
///   unavailable or rendering fails.
pub(crate) async fn get_og(
    State(shared): State<SharedState>,
    Path(style): Path<String>,
    Query(raw): Query<HashMap<String, String>>,
) -> Result<Response, TileServerError> {
    let state = shared.load();

    let params = parse_og_query(&raw)?;

    let Some(loaded) = state.styles.get(&style) else {
        return Err(TileServerError::StyleNotFound(style));
    };

    let renderer = state
        .renderer
        .as_ref()
        .ok_or_else(|| TileServerError::RenderError("Rendering not available".to_string()))?;

    let rewritten_style = styles::rewrite_style_for_native(
        &loaded.style_json,
        &state.render_base_url,
        &state.sources,
    );

    let options = build_static_options(
        style,
        rewritten_style.to_string(),
        params.static_type,
        params.width,
        params.height,
        1,
        params.format,
        StaticQueryParams::default(),
    )?;

    let image_data = renderer.render_static(options).await?;

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(params.format.content_type()),
    );
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );

    Ok((headers, image_data).into_response())
}
