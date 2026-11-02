//! Static file serving route handler.
//!
//! Endpoint for serving static files (GeoJSON, icons, etc.) from
//! a configured files directory.

use axum::{
    extract::{Path, State},
    http::{
        HeaderMap, HeaderValue,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};

use crate::error::TileServerError;
use crate::reload::SharedState;

/// Get a static file from the files directory
/// Route: GET /files/{*filepath}
pub(crate) async fn get_static_file(
    State(shared): State<SharedState>,
    Path(filepath): Path<String>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let files_dir = state
        .files_dir
        .as_ref()
        .ok_or_else(|| TileServerError::NotFound("files directory not configured".to_string()))?;

    // Sanitize the filepath to prevent directory traversal attacks
    let filepath = filepath.trim_start_matches('/');
    if filepath.contains("..") || filepath.starts_with('/') {
        return Err(TileServerError::NotFound("invalid file path".to_string()));
    }

    let file_path = files_dir.join(filepath);

    // Ensure the resolved path is still within the files directory
    let canonical_files_dir = files_dir
        .canonicalize()
        .map_err(|_| TileServerError::NotFound("files directory not accessible".to_string()))?;
    let canonical_file_path = file_path
        .canonicalize()
        .map_err(|_| TileServerError::NotFound(format!("file not found: {}", filepath)))?;

    if !canonical_file_path.starts_with(&canonical_files_dir) {
        return Err(TileServerError::NotFound("invalid file path".to_string()));
    }

    // Read the file
    let data = tokio::fs::read(&canonical_file_path)
        .await
        .map_err(|_| TileServerError::NotFound(format!("file not found: {}", filepath)))?;

    // Determine content type from extension
    let content_type = mime_guess::from_path(&canonical_file_path)
        .first_or_octet_stream()
        .to_string();

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&content_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    // Cache static files for 1 hour
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );

    Ok((headers, data).into_response())
}
