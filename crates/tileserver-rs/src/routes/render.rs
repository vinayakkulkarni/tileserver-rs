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

#[cfg(test)]
mod tests {
    use super::*;

    fn raster(y_fmt: &str) -> RasterTileParams {
        RasterTileParams {
            style: "s".into(),
            z: 0,
            x: 0,
            y_fmt: y_fmt.into(),
        }
    }

    fn raster_sz(tile_size: u16, y_fmt: &str) -> RasterTileWithSizeParams {
        RasterTileWithSizeParams {
            style: "s".into(),
            tile_size,
            z: 0,
            x: 0,
            y_fmt: y_fmt.into(),
        }
    }

    fn stat_img(size_fmt: &str) -> StaticImageParams {
        StaticImageParams {
            style: "s".into(),
            static_type: "0,0,2".into(),
            size_fmt: size_fmt.into(),
        }
    }

    #[test]
    fn raster_parse_plain_png() {
        let (y, scale, fmt) = raster("3.png").parse().expect("3.png parses");
        assert_eq!(y, 3);
        assert_eq!(scale, 1);
        assert_eq!(fmt, ImageFormat::Png);
    }

    #[test]
    fn raster_parse_retina_2x() {
        let (y, scale, fmt) = raster("5@2x.webp").parse().expect("@2x parses");
        assert_eq!(y, 5);
        assert_eq!(scale, 2);
        assert_eq!(fmt, ImageFormat::Webp);
    }

    #[test]
    fn raster_parse_retina_max_9x() {
        let parsed = raster("0@9x.jpg").parse().expect("9x boundary parses");
        assert_eq!(parsed.1, 9);
    }

    #[test]
    fn raster_parse_rejects_scale_zero() {
        assert!(raster("0@0x.png").parse().is_none());
    }

    #[test]
    fn raster_parse_rejects_scale_above_9() {
        assert!(raster("0@10x.png").parse().is_none());
    }

    #[test]
    fn raster_parse_rejects_scale_without_x_suffix() {
        assert!(raster("0@2.png").parse().is_none());
    }

    #[test]
    fn raster_parse_rejects_non_numeric_y() {
        assert!(raster("abc.png").parse().is_none());
    }

    #[test]
    fn raster_parse_rejects_missing_extension() {
        assert!(raster("123").parse().is_none());
    }

    #[test]
    fn raster_parse_rejects_unknown_format() {
        assert!(raster("123.gif").parse().is_none());
    }

    #[test]
    fn raster_parse_rejects_non_numeric_scale() {
        assert!(raster("0@xx.png").parse().is_none());
    }

    #[test]
    fn raster_size_parse_basic_png_256() {
        let (y, scale, fmt) = raster_sz(256, "1.png").parse().expect("256/1.png parses");
        assert_eq!(y, 1);
        assert_eq!(scale, 1);
        assert_eq!(fmt, ImageFormat::Png);
    }

    #[test]
    fn raster_size_parse_retina_512_at2x_clamped() {
        let parsed = raster_sz(512, "2@2x.webp")
            .parse()
            .expect("512/2@2x parses");
        assert_eq!(parsed.0, 2);
        assert_eq!(parsed.1, 2);
        assert_eq!(parsed.2, ImageFormat::Webp);
    }

    #[test]
    fn raster_size_parse_rejects_bad_format() {
        assert!(raster_sz(256, "0.bmp").parse().is_none());
    }

    #[test]
    fn raster_size_parse_rejects_zero_scale() {
        assert!(raster_sz(256, "0@0x.png").parse().is_none());
    }

    #[test]
    fn raster_size_parse_rejects_huge_scale() {
        assert!(raster_sz(512, "0@99x.png").parse().is_none());
    }

    #[test]
    fn static_image_parse_basic_size() {
        let (w, h, scale, fmt) = stat_img("800x600.png").parse().expect("800x600.png");
        assert_eq!((w, h, scale), (800, 600, 1));
        assert_eq!(fmt, ImageFormat::Png);
    }

    #[test]
    fn static_image_parse_retina_2x_webp() {
        let (w, h, scale, fmt) = stat_img("400x300@2x.webp").parse().expect("retina webp");
        assert_eq!((w, h, scale), (400, 300, 2));
        assert_eq!(fmt, ImageFormat::Webp);
    }

    #[test]
    fn static_image_parse_rejects_missing_dot() {
        assert!(stat_img("800x600").parse().is_none());
    }

    #[test]
    fn static_image_parse_rejects_missing_x_in_size() {
        assert!(stat_img("800600.png").parse().is_none());
    }

    #[test]
    fn static_image_parse_rejects_non_numeric_width() {
        assert!(stat_img("axb.png").parse().is_none());
    }

    #[test]
    fn static_image_parse_rejects_unknown_format() {
        assert!(stat_img("800x600.gif").parse().is_none());
    }

    #[test]
    fn static_image_parse_rejects_scale_zero() {
        assert!(stat_img("800x600@0x.png").parse().is_none());
    }

    #[test]
    fn static_image_parse_rejects_scale_above_9() {
        assert!(stat_img("800x600@10x.png").parse().is_none());
    }

    #[test]
    fn static_image_parse_rejects_scale_without_x_suffix() {
        assert!(stat_img("800x600@2.png").parse().is_none());
    }

    #[test]
    fn record_render_metric_ok_branch() {
        let started = std::time::Instant::now();
        let result: Result<Vec<u8>, TileServerError> = Ok(vec![1, 2, 3]);
        record_render_metric("style-a", ImageFormat::Png, started, &result);
    }

    #[test]
    fn record_render_metric_err_branch_png() {
        let started = std::time::Instant::now();
        let result: Result<Vec<u8>, TileServerError> =
            Err(TileServerError::RenderError("boom".into()));
        record_render_metric("style-a", ImageFormat::Png, started, &result);
    }

    #[test]
    fn record_render_metric_err_branch_jpeg() {
        let started = std::time::Instant::now();
        let result: Result<Vec<u8>, TileServerError> =
            Err(TileServerError::RenderError("boom".into()));
        record_render_metric("style-b", ImageFormat::Jpeg, started, &result);
    }

    #[test]
    fn record_render_metric_err_branch_webp() {
        let started = std::time::Instant::now();
        let result: Result<Vec<u8>, TileServerError> =
            Err(TileServerError::RenderError("boom".into()));
        record_render_metric("style-c", ImageFormat::Webp, started, &result);
    }
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
