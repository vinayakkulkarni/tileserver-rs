//! File upload endpoints for server-side geospatial format processing.
//!
//! Supports MBTiles, SQLite, and COG files that require server-side processing.
//! Uploaded files become temporary tile sources available until removed.
//! Files are streamed to disk chunk-by-chunk to avoid OOM on large uploads.

use std::sync::Arc;

use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use tokio::io::AsyncWriteExt;

use crate::config::{SourceConfig, SourceType};
use crate::error::TileServerError;
use crate::reload::{AppState, SharedState, UploadInfo};
use crate::sources::SourceManager;

/// Upload response returned to the client
#[derive(Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub source_id: String,
    pub file_name: String,
    pub format: String,
    pub tilejson_url: String,
}

/// Detect source type from file extension
fn detect_source_type(filename: &str) -> Result<SourceType, TileServerError> {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();

    match ext.as_str() {
        "mbtiles" => Ok(SourceType::MBTiles),
        "sqlite" | "db" => Ok(SourceType::MBTiles),
        #[cfg(feature = "raster")]
        "tif" | "tiff" => Ok(SourceType::Cog),
        #[cfg(feature = "geoparquet")]
        "parquet" | "geoparquet" => Ok(SourceType::GeoParquet),
        _ => Err(TileServerError::UploadError(format!(
            "unsupported file format: .{ext}"
        ))),
    }
}

/// Format string for a source type (used in responses)
fn source_type_label(st: &SourceType) -> &'static str {
    match st {
        SourceType::MBTiles => "mbtiles",
        SourceType::PMTiles => "pmtiles",
        #[cfg(feature = "postgres")]
        SourceType::Postgres => "postgres",
        #[cfg(feature = "raster")]
        SourceType::Cog => "cog",
        #[cfg(feature = "raster")]
        SourceType::Vrt => "vrt",
        #[cfg(feature = "geoparquet")]
        SourceType::GeoParquet => "geoparquet",
    }
}

/// POST /api/upload — Upload a geospatial file (streamed to disk)
pub async fn upload_file(
    State(shared): State<SharedState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, TileServerError> {
    let state = shared.load();
    let upload_dir = state
        .upload_dir
        .as_ref()
        .ok_or_else(|| TileServerError::UploadError("upload directory not configured".into()))?;

    // Extract file field from multipart
    let mut field = multipart
        .next_field()
        .await
        .map_err(|e| TileServerError::UploadError(format!("failed to read multipart field: {e}")))?
        .ok_or_else(|| TileServerError::UploadError("no file field in request".into()))?;

    let file_name = field
        .file_name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "upload".to_string());
    let source_type = detect_source_type(&file_name)?;

    // Generate unique ID and prepare file path
    let upload_id = uuid::Uuid::new_v4().to_string();
    let ext = file_name.rsplit('.').next().unwrap_or("bin");
    let saved_name = format!("{upload_id}.{ext}");
    let file_path = upload_dir.join(&saved_name);

    // Compute max size from config (loaded at startup)
    // Default 500 MB, read from the live config value stored during build_app_state
    let max_upload_bytes: usize = 500 * 1024 * 1024; // fallback; real limit enforced by axum layer

    // Stream chunks to disk — never holds the full file in memory
    let mut file = tokio::fs::File::create(&file_path)
        .await
        .map_err(|e| TileServerError::UploadError(format!("failed to create file: {e}")))?;

    let mut total_size: usize = 0;

    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|e| TileServerError::UploadError(format!("failed to read chunk: {e}")))?
    {
        total_size += chunk.len();
        if total_size > max_upload_bytes {
            // Clean up partial file
            drop(file);
            let _ = tokio::fs::remove_file(&file_path).await;
            return Err(TileServerError::UploadTooLarge);
        }
        file.write_all(&chunk)
            .await
            .map_err(|e| TileServerError::UploadError(format!("failed to write chunk: {e}")))?;
    }

    file.flush()
        .await
        .map_err(|e| TileServerError::UploadError(format!("failed to flush file: {e}")))?;
    drop(file);

    tracing::info!(
        "Uploaded file saved: {} ({} bytes)",
        file_path.display(),
        total_size
    );

    let source_id = format!("upload-{upload_id}");

    // Create source config and load the source to validate the file
    let source_config = SourceConfig {
        id: source_id.clone(),
        source_type: source_type.clone(),
        path: file_path.to_string_lossy().to_string(),
        name: Some(file_name.clone()),
        attribution: None,
        description: Some(format!("Uploaded file: {file_name}")),
        resampling: None,
        layer_name: None,
        geometry_column: None,
        minzoom: None,
        maxzoom: None,
        serve_as: None,
        #[cfg(feature = "raster")]
        colormap: None,
    };

    let mut temp_manager = SourceManager::new();
    if let Err(e) = temp_manager.load_source(&source_config).await {
        // Clean up file on load failure
        let _ = tokio::fs::remove_file(&file_path).await;
        return Err(TileServerError::UploadError(format!(
            "failed to load source from uploaded file: {e}"
        )));
    }

    let new_source = temp_manager.get(&source_id).cloned().ok_or_else(|| {
        let _ = std::fs::remove_file(&file_path);
        TileServerError::UploadError("source failed to register".into())
    })?;

    // Swap into the live state: clone sources, add new one, rebuild AppState
    let mut sources_map = state.sources.clone_sources();
    sources_map.insert(source_id.clone(), new_source);
    let new_manager = SourceManager::from_sources(sources_map);

    let new_state = AppState {
        sources: Arc::new(new_manager),
        styles: state.styles.clone(),
        renderer: state.renderer.clone(),
        base_url: state.base_url.clone(),
        render_base_url: state.render_base_url.clone(),
        ui_enabled: state.ui_enabled,
        fonts_dir: state.fonts_dir.clone(),
        files_dir: state.files_dir.clone(),
        upload_dir: state.upload_dir.clone(),
    };

    shared.store(Arc::new(new_state));

    // Track in upload registry
    let format_label = source_type_label(&source_type);

    {
        let mut uploads = shared.uploads().write().await;
        uploads.insert(
            source_id.clone(),
            UploadInfo {
                id: upload_id.clone(),
                file_name: file_name.clone(),
                format: format_label.to_string(),
                file_path,
            },
        );
    }

    let tilejson_url = format!("{}/data/{source_id}.json", state.base_url);

    tracing::info!(
        "Registered uploaded source: {} ({})",
        source_id,
        format_label
    );

    Ok(Json(UploadResponse {
        id: upload_id,
        source_id,
        file_name,
        format: format_label.to_string(),
        tilejson_url,
    }))
}

/// GET /api/upload — List all uploaded sources
pub async fn list_uploads(State(shared): State<SharedState>) -> Json<Vec<UploadInfo>> {
    let uploads = shared.uploads().read().await;
    Json(uploads.values().cloned().collect())
}

/// DELETE /api/upload/{id} — Remove an uploaded source and delete the file
pub async fn delete_upload(
    State(shared): State<SharedState>,
    Path(upload_id): Path<String>,
) -> Result<StatusCode, TileServerError> {
    // Find the source_id from upload registry
    let source_id = {
        let uploads = shared.uploads().read().await;
        // Accept either the UUID or the full source ID (upload-{uuid})
        let entry = uploads
            .iter()
            .find(|(sid, info)| info.id == upload_id || sid.as_str() == upload_id);

        match entry {
            Some((sid, _)) => sid.clone(),
            None => return Err(TileServerError::SourceNotFound(upload_id)),
        }
    };

    // Remove from upload registry and get file path for cleanup
    let file_path = {
        let mut uploads = shared.uploads().write().await;
        uploads.remove(&source_id).map(|info| info.file_path)
    };

    // Remove source from live state
    let state = shared.load();
    let mut sources_map = state.sources.clone_sources();
    sources_map.remove(&source_id);
    let new_manager = SourceManager::from_sources(sources_map);

    let new_state = AppState {
        sources: Arc::new(new_manager),
        styles: state.styles.clone(),
        renderer: state.renderer.clone(),
        base_url: state.base_url.clone(),
        render_base_url: state.render_base_url.clone(),
        ui_enabled: state.ui_enabled,
        fonts_dir: state.fonts_dir.clone(),
        files_dir: state.files_dir.clone(),
        upload_dir: state.upload_dir.clone(),
    };

    shared.store(Arc::new(new_state));

    // Delete the uploaded file from disk
    if let Some(path) = file_path {
        if let Err(e) = tokio::fs::remove_file(&path).await {
            tracing::warn!("Failed to delete uploaded file {}: {}", path.display(), e);
        } else {
            tracing::info!("Deleted uploaded file: {}", path.display());
        }
    }

    Ok(StatusCode::NO_CONTENT)
}
