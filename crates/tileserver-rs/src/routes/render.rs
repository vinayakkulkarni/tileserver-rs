//! Raster rendering route handlers.
//!
//! Endpoints for rendering raster tiles (PNG/JPEG/WebP) and static map images
//! from vector styles using the native MapLibre renderer.

use axum::{
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
use crate::render::{ImageFormat, RenderOptions, StaticQueryParams, StaticType};
use crate::styles;

/// Raster tile request parameters
#[derive(serde::Deserialize)]
pub(super) struct RasterTileParams {
    style: String,
    z: u8,
    x: u32,
    y_fmt: String, // e.g., "123.png" or "123@2x.webp"
}

impl RasterTileParams {
    /// Parse y, scale, and format from "123@2x.png" style string
    fn parse(&self) -> Option<(u32, u8, ImageFormat)> {
        // Split extension first: "123@2x" and "png"
        let (y_and_scale, format_str) = self.y_fmt.rsplit_once('.')?;

        let format = format_str.parse::<ImageFormat>().ok()?;

        // Check for scale: "123@2x" or just "123"
        if let Some((y_str, scale_str)) = y_and_scale.split_once('@') {
            let y = y_str.parse().ok()?;
            // Parse scale like "2x" -> 2
            let scale = scale_str.strip_suffix('x')?.parse().ok()?;
            // Validate scale range (1-9)
            if (1..=9).contains(&scale) {
                Some((y, scale, format))
            } else {
                None
            }
        } else {
            // No scale, default to 1
            let y = y_and_scale.parse().ok()?;
            Some((y, 1, format))
        }
    }
}

/// Get a raster tile (rendered from style)
/// Route: GET /styles/{style}/{z}/{x}/{y}[@{scale}x].{format}
pub(crate) async fn get_raster_tile(
    State(shared): State<SharedState>,
    Path(params): Path<RasterTileParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();

    let renderer = state
        .renderer
        .as_ref()
        .ok_or_else(|| TileServerError::RenderError("Rendering not available".to_string()))?;

    // Parse parameters
    let (y, scale, format) = params.parse().ok_or(TileServerError::InvalidTileRequest)?;

    // Get style
    let style = match state.styles.get(&params.style) {
        Some(s) => s,
        None => return Err(TileServerError::StyleNotFound(params.style)),
    };

    let rewritten_style =
        styles::rewrite_style_for_native(&style.style_json, &state.render_base_url, &state.sources);

    let render_started = std::time::Instant::now();
    let render_result = renderer
        .render_tile(
            &rewritten_style.to_string(),
            params.z,
            params.x,
            y,
            scale,
            format,
        )
        .await;
    record_render_metric(&params.style, format, render_started, &render_result);
    let image_data = render_result?;

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(format.content_type()),
    );
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());

    Ok((headers, image_data).into_response())
}

fn record_render_metric(
    style_id: &str,
    format: ImageFormat,
    started: std::time::Instant,
    result: &Result<Vec<u8>, TileServerError>,
) {
    let metric_format = match format {
        ImageFormat::Png => crate::sources::TileFormat::Png,
        ImageFormat::Jpeg => crate::sources::TileFormat::Jpeg,
        ImageFormat::Webp => crate::sources::TileFormat::Webp,
    };
    let (outcome, error_reason) = match result {
        Ok(_) => (crate::metrics::RenderOutcome::Success, None),
        Err(_) => (crate::metrics::RenderOutcome::Error, Some("render_failed")),
    };
    crate::metrics::render_recorded(crate::metrics::RenderEvent {
        style: style_id,
        format: metric_format,
        duration: started.elapsed(),
        outcome,
        error_reason,
    });
}

/// Raster tile request parameters with variable tile size
#[derive(serde::Deserialize)]
pub(super) struct RasterTileWithSizeParams {
    style: String,
    tile_size: u16, // e.g., 256 or 512
    z: u8,
    x: u32,
    y_fmt: String, // e.g., "123.png" or "123@2x.webp"
}

impl RasterTileWithSizeParams {
    /// Parse y, scale, and format from "123@2x.png" style string
    fn parse(&self) -> Option<(u32, u8, ImageFormat)> {
        // Split extension first: "123@2x" and "png"
        let (y_and_scale, format_str) = self.y_fmt.rsplit_once('.')?;

        let format = format_str.parse::<ImageFormat>().ok()?;

        // Check for scale: "123@2x" or just "123"
        if let Some((y_str, scale_str)) = y_and_scale.split_once('@') {
            let y = y_str.parse().ok()?;
            // Parse scale like "2x" -> 2
            let scale = scale_str.strip_suffix('x')?.parse().ok()?;
            // Validate scale range (1-9)
            if (1..=9).contains(&scale) {
                Some((y, scale, format))
            } else {
                None
            }
        } else {
            // No scale, default to 1
            let y = y_and_scale.parse().ok()?;
            Some((y, 1, format))
        }
    }
}

/// Get a raster tile with variable tile size
/// Route: GET /styles/{style}/{tile_size}/{z}/{x}/{y}[@{scale}x].{format}
pub(crate) async fn get_raster_tile_with_size(
    State(shared): State<SharedState>,
    Path(params): Path<RasterTileWithSizeParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();

    // Validate tile size (only 256 and 512 are supported)
    if params.tile_size != 256 && params.tile_size != 512 {
        return Err(TileServerError::RenderError(format!(
            "Invalid tile size: {}. Only 256 and 512 are supported.",
            params.tile_size
        )));
    }

    // Check if rendering is available
    let renderer = state
        .renderer
        .as_ref()
        .ok_or_else(|| TileServerError::RenderError("Rendering not available".to_string()))?;

    // Parse parameters
    let (y, additional_scale, format) =
        params.parse().ok_or(TileServerError::InvalidTileRequest)?;

    // Calculate effective scale
    // For 512px tiles, we use scale=2 (renders at 512px)
    // For 256px tiles, we use scale=1 (renders at 256px)
    // Additional scale from URL (e.g., @2x) multiplies on top
    let base_scale: u8 = if params.tile_size == 512 { 2 } else { 1 };
    let effective_scale = base_scale * additional_scale;

    // Clamp to valid range
    let scale = effective_scale.min(9);

    // Get style
    let style = match state.styles.get(&params.style) {
        Some(s) => s,
        None => return Err(TileServerError::StyleNotFound(params.style)),
    };

    let rewritten_style =
        styles::rewrite_style_for_native(&style.style_json, &state.render_base_url, &state.sources);

    let render_started = std::time::Instant::now();
    let render_result = renderer
        .render_tile(
            &rewritten_style.to_string(),
            params.z,
            params.x,
            y,
            scale,
            format,
        )
        .await;
    record_render_metric(&params.style, format, render_started, &render_result);
    let image_data = render_result?;

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(format.content_type()),
    );
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());

    Ok((headers, image_data).into_response())
}

/// Static image request parameters
#[derive(serde::Deserialize)]
pub(super) struct StaticImageParams {
    style: String,
    static_type: String, // e.g., "-122.4,37.8,12" or "auto"
    size_fmt: String,    // e.g., "800x600.png" or "800x600@2x.webp"
}

impl StaticImageParams {
    /// Parse size, scale, and format from "800x600@2x.png" style string
    fn parse(&self) -> Option<(u32, u32, u8, ImageFormat)> {
        // Split extension: "800x600@2x" and "png"
        let (size_and_scale, format_str) = self.size_fmt.rsplit_once('.')?;

        let format = format_str.parse::<ImageFormat>().ok()?;

        // Check for scale: "800x600@2x" or just "800x600"
        let (size_str, scale) = if let Some((size, scale_str)) = size_and_scale.split_once('@') {
            let scale = scale_str.strip_suffix('x')?.parse().ok()?;
            if !(1..=9).contains(&scale) {
                return None;
            }
            (size, scale)
        } else {
            (size_and_scale, 1)
        };

        // Parse width and height: "800x600"
        let (width_str, height_str) = size_str.split_once('x')?;
        let width = width_str.parse().ok()?;
        let height = height_str.parse().ok()?;

        Some((width, height, scale, format))
    }
}

/// Get a static image
/// Route: GET /styles/{style}/static/{static_type}/{width}x{height}[@{scale}x].{format}
pub(crate) async fn get_static_image(
    State(shared): State<SharedState>,
    Path(params): Path<StaticImageParams>,
    Query(query): Query<StaticQueryParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();

    let renderer = state
        .renderer
        .as_ref()
        .ok_or_else(|| TileServerError::RenderError("Rendering not available".to_string()))?;

    // Parse parameters
    let (width, height, scale, format) = params.parse().ok_or_else(|| {
        TileServerError::RenderError(format!("Invalid size format: {}", params.size_fmt))
    })?;

    // Parse static type
    let static_type = params
        .static_type
        .parse::<StaticType>()
        .map_err(TileServerError::RenderError)?;

    // Get style
    let style = match state.styles.get(&params.style) {
        Some(s) => s,
        None => return Err(TileServerError::StyleNotFound(params.style)),
    };

    // Rewrite style to inline tile URLs for native rendering
    let rewritten_style =
        styles::rewrite_style_for_native(&style.style_json, &state.render_base_url, &state.sources);

    // Create render options
    let options = RenderOptions::for_static(
        params.style.clone(),
        rewritten_style.to_string(),
        static_type,
        width,
        height,
        scale,
        format,
        query,
    )
    .map_err(TileServerError::RenderError)?;

    // Render static image
    let image_data = renderer.render_static(options).await?;

    // Build response
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(format.content_type()),
    );
    // Cache static images for 1 hour
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );

    Ok((headers, image_data).into_response())
}
