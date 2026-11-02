//! Font route handlers.
//!
//! Endpoints for listing available fonts and serving font glyph PBF files.

use axum::{
    Json,
    extract::{Path, State},
    http::{
        HeaderMap, HeaderValue,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};

use crate::cache_control;
use crate::error::TileServerError;
use crate::reload::SharedState;

/// Get list of available fonts
/// Route: GET /fonts.json
pub(crate) async fn get_fonts_list(
    State(shared): State<SharedState>,
) -> Result<Json<Vec<String>>, TileServerError> {
    let state = shared.load();

    let fonts_dir = match &state.fonts_dir {
        Some(dir) => dir,
        None => return Ok(Json(Vec::new())),
    };

    let mut fonts = Vec::with_capacity(16);

    // Read the fonts directory to find font families
    // Each subdirectory is a font family (e.g., "Noto Sans Regular")
    if let Ok(mut entries) = tokio::fs::read_dir(fonts_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(file_type) = entry.file_type().await
                && file_type.is_dir()
                && let Some(name) = entry.file_name().to_str()
            {
                // Only include directories that have at least one .pbf file
                let font_dir = entry.path();
                if has_pbf_files(&font_dir).await {
                    fonts.push(name.to_string());
                }
            }
        }
    }

    // Sort alphabetically for consistent output
    fonts.sort();

    Ok(Json(fonts))
}

/// Check if a directory contains at least one .pbf file
async fn has_pbf_files(dir: &std::path::Path) -> bool {
    if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(name) = entry.file_name().to_str()
                && name.ends_with(".pbf")
            {
                return true;
            }
        }
    }
    false
}

/// Font glyph request parameters
#[derive(serde::Deserialize)]
pub(super) struct FontParams {
    fontstack: String, // e.g., "Noto Sans Regular" or "Open Sans Bold,Arial Unicode MS Regular"
    range: String,     // e.g., "0-255.pbf"
}

/// Get font glyphs (PBF format)
/// Route: GET /fonts/{fontstack}/{start}-{end}.pbf
pub(crate) async fn get_font_glyphs(
    State(shared): State<SharedState>,
    Path(params): Path<FontParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let fonts_dir = state.fonts_dir.as_ref().ok_or_else(|| {
        TileServerError::FontNotFound("fonts directory not configured".to_string())
    })?;

    // Parse the range to ensure it's valid (e.g., "0-255.pbf")
    // Must match pattern like "0-255.pbf", "256-511.pbf", etc.
    if !params.range.ends_with(".pbf") {
        return Err(TileServerError::InvalidTileRequest);
    }

    // Security: Validate range format to prevent path traversal
    let range_name = params.range.trim_end_matches(".pbf");
    if range_name.contains("..") || range_name.contains('/') || range_name.contains('\\') {
        return Err(TileServerError::InvalidTileRequest);
    }

    // Font stacks are comma-separated, try each font in order
    let fonts: Vec<&str> = params.fontstack.split(',').map(|s| s.trim()).collect();

    // Security: Canonicalize fonts directory for path validation
    let canonical_fonts_dir = fonts_dir
        .canonicalize()
        .map_err(|_| TileServerError::FontNotFound("fonts directory not accessible".to_string()))?;

    for font_name in &fonts {
        // Security: Reject font names with path traversal sequences
        if font_name.contains("..") || font_name.contains('/') || font_name.contains('\\') {
            continue;
        }

        let font_path = fonts_dir.join(font_name).join(&params.range);

        // Security: Verify the resolved path is within fonts directory
        if let Ok(canonical_path) = font_path.canonicalize()
            && !canonical_path.starts_with(&canonical_fonts_dir)
        {
            continue; // Path escapes fonts directory
        }

        if let Ok(data) = tokio::fs::read(&font_path).await {
            let mut headers = HeaderMap::new();
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-protobuf"),
            );
            headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());

            tracing::debug!("Serving font: {}/{}", font_name, params.range);
            return Ok((headers, data).into_response());
        }
    }

    // Return empty PBF for missing glyph ranges — MapLibre Native requests all
    // 256 possible ranges and fails hard on 404, even for unpopulated Unicode blocks
    tracing::debug!(
        "Font range not found, returning empty PBF: {} (tried: {:?})",
        params.range,
        fonts
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-protobuf"),
    );
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());
    Ok((headers, Vec::<u8>::new()).into_response())
}
