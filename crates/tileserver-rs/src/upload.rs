//! File upload endpoints for server-side geospatial format processing.
//!
//! Supports MBTiles, SQLite, and COG files that require server-side processing.
//! Uploaded files become temporary tile sources available until removed.
//! Files are streamed to disk chunk-by-chunk to avoid OOM on large uploads.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use serde::Serialize;
use tokio::io::AsyncWriteExt;

use crate::config::{SourceConfig, SourceType};
use crate::error::TileServerError;
use crate::reload::{AppState, SharedState, UploadInfo};
use crate::sources::SourceManager;

fn state_with_sources(state: &AppState, new_manager: SourceManager) -> AppState {
    AppState {
        sources: Arc::new(new_manager),
        styles: state.styles.clone(),
        renderer: state.renderer.clone(),
        base_url: state.base_url.clone(),
        render_base_url: state.render_base_url.clone(),
        ui_enabled: state.ui_enabled,
        fonts_dir: state.fonts_dir.clone(),
        files_dir: state.files_dir.clone(),
        upload_dir: state.upload_dir.clone(),
    }
}

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
        SourceType::Dir => "dir",
        SourceType::Tar => "tar",
        #[cfg(feature = "postgres")]
        SourceType::Postgres => "postgres",
        #[cfg(feature = "raster")]
        SourceType::Cog => "cog",
        #[cfg(feature = "raster")]
        SourceType::Vrt => "vrt",
        #[cfg(feature = "geoparquet")]
        SourceType::GeoParquet => "geoparquet",
        #[cfg(feature = "duckdb")]
        SourceType::DuckDB => "duckdb",
        #[cfg(feature = "stac")]
        SourceType::Stac => "stac",
        #[cfg(feature = "dem")]
        SourceType::Dem => "dem",
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
        query: None,
        serve_as: None,
        #[cfg(feature = "raster")]
        colormap: None,
        options: None,
        collection: None,
        asset_role: "visual".to_string(),
        dynamic: false,
        max_items: 100,
        stac_bbox: None,
        pixel_selection: crate::config::PixelSelectionMethod::First,
        tile_path_template: None,
        tms: false,
        #[cfg(feature = "dem")]
        input_source: None,
        #[cfg(feature = "dem")]
        dem_encoding: crate::config::DemEncoding::Terrarium,
        #[cfg(feature = "dem")]
        dem_scale: None,
        #[cfg(feature = "dem")]
        dem_offset: None,
        #[cfg(feature = "dem")]
        dem_band: 1,
        #[cfg(feature = "dem")]
        dem_nodata_color: None,
    };

    let mut temp_manager = SourceManager::new();
    if let Err(e) = temp_manager.load_source(&source_config).await {
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

    let new_state = state_with_sources(&state, new_manager);

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

    let new_state = state_with_sources(&state, new_manager);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_source_type_mbtiles() {
        let st = detect_source_type("world.mbtiles").unwrap();
        assert!(matches!(st, SourceType::MBTiles));
    }

    #[test]
    fn test_detect_source_type_sqlite() {
        let st = detect_source_type("data.sqlite").unwrap();
        assert!(matches!(st, SourceType::MBTiles));
    }

    #[test]
    fn test_detect_source_type_db() {
        let st = detect_source_type("data.db").unwrap();
        assert!(matches!(st, SourceType::MBTiles));
    }

    #[test]
    fn test_detect_source_type_unsupported() {
        let result = detect_source_type("data.csv");
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_source_type_no_extension() {
        let result = detect_source_type("noextension");
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_source_type_case_insensitive() {
        let st = detect_source_type("tiles.MBTILES").unwrap();
        assert!(matches!(st, SourceType::MBTiles));
    }

    #[test]
    fn test_source_type_label_mbtiles() {
        assert_eq!(source_type_label(&SourceType::MBTiles), "mbtiles");
    }

    #[test]
    fn test_source_type_label_pmtiles() {
        assert_eq!(source_type_label(&SourceType::PMTiles), "pmtiles");
    }

    #[test]
    fn test_source_type_label_all_variants() {
        assert_eq!(source_type_label(&SourceType::MBTiles), "mbtiles");
        assert_eq!(source_type_label(&SourceType::PMTiles), "pmtiles");
        assert_eq!(source_type_label(&SourceType::Dir), "dir");
        assert_eq!(source_type_label(&SourceType::Tar), "tar");
        #[cfg(feature = "postgres")]
        assert_eq!(source_type_label(&SourceType::Postgres), "postgres");
        #[cfg(feature = "raster")]
        assert_eq!(source_type_label(&SourceType::Cog), "cog");
        #[cfg(feature = "raster")]
        assert_eq!(source_type_label(&SourceType::Vrt), "vrt");
        #[cfg(feature = "geoparquet")]
        assert_eq!(source_type_label(&SourceType::GeoParquet), "geoparquet");
        #[cfg(feature = "duckdb")]
        assert_eq!(source_type_label(&SourceType::DuckDB), "duckdb");
        #[cfg(feature = "stac")]
        assert_eq!(source_type_label(&SourceType::Stac), "stac");
        #[cfg(feature = "dem")]
        assert_eq!(source_type_label(&SourceType::Dem), "dem");
    }

    #[cfg(feature = "raster")]
    #[test]
    fn test_detect_source_type_tiff() {
        let st = detect_source_type("satellite.tif").unwrap();
        assert!(matches!(st, SourceType::Cog));
    }

    #[cfg(feature = "geoparquet")]
    #[test]
    fn test_detect_source_type_parquet() {
        let st = detect_source_type("buildings.parquet").unwrap();
        assert!(matches!(st, SourceType::GeoParquet));
    }

    #[test]
    #[cfg(feature = "stac")]
    fn test_source_type_label_stac() {
        assert_eq!(source_type_label(&SourceType::Stac), "stac");
    }

    use crate::reload::{
        AppState, ReloadController, ReloadMeta, RuntimeSettings, SharedState, UploadInfo,
        now_unix_seconds,
    };
    use crate::sources::SourceManager;
    use crate::styles::StyleManager;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn build_shared(upload_dir: Option<PathBuf>) -> SharedState {
        let state = AppState {
            sources: Arc::new(SourceManager::new()),
            styles: Arc::new(StyleManager::new()),
            renderer: None,
            base_url: "http://h".to_string(),
            render_base_url: "http://127.0.0.1:0".to_string(),
            ui_enabled: false,
            fonts_dir: None,
            files_dir: None,
            upload_dir,
        };
        let meta = ReloadMeta {
            config_hash: "h".to_string(),
            loaded_at_unix: now_unix_seconds(),
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
        let controller = Arc::new(ReloadController::new(
            state,
            meta,
            crate::config::Config::default(),
            None,
            runtime,
        ));
        SharedState::new(controller)
    }

    #[test]
    fn state_with_sources_swaps_sources_only() {
        let shared = build_shared(Some(PathBuf::from("/tmp")));
        let original = shared.load();

        let _ = HashMap::<String, Arc<dyn crate::sources::TileSource>>::new();
        let new_manager = SourceManager::new();
        let new_state = state_with_sources(&original, new_manager);

        assert!(new_state.sources.is_empty());
        assert_eq!(new_state.base_url, original.base_url);
        assert_eq!(new_state.render_base_url, original.render_base_url);
        assert_eq!(new_state.ui_enabled, original.ui_enabled);
        assert_eq!(new_state.upload_dir, original.upload_dir);
    }

    #[test]
    fn state_with_sources_preserves_renderer_and_dirs() {
        let shared = build_shared(None);
        let original = shared.load();
        let new_state = state_with_sources(&original, SourceManager::new());
        assert!(new_state.renderer.is_none());
        assert!(new_state.fonts_dir.is_none());
        assert!(new_state.files_dir.is_none());
    }

    #[tokio::test]
    async fn list_uploads_returns_empty_when_no_uploads() {
        let shared = build_shared(None);
        let Json(infos) = list_uploads(axum::extract::State(shared)).await;
        assert!(infos.is_empty());
    }

    #[tokio::test]
    async fn list_uploads_returns_registered_uploads() {
        let shared = build_shared(None);
        {
            let mut uploads = shared.uploads().write().await;
            uploads.insert(
                "upload-abc".to_string(),
                UploadInfo {
                    id: "abc".to_string(),
                    file_name: "world.mbtiles".to_string(),
                    format: "mbtiles".to_string(),
                    file_path: PathBuf::from("/tmp/abc.mbtiles"),
                },
            );
        }
        let Json(infos) = list_uploads(axum::extract::State(shared)).await;
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].file_name, "world.mbtiles");
        assert_eq!(infos[0].format, "mbtiles");
    }

    #[tokio::test]
    async fn delete_upload_unknown_id_returns_source_not_found() {
        let shared = build_shared(None);
        let result = delete_upload(
            axum::extract::State(shared),
            axum::extract::Path("does-not-exist".to_string()),
        )
        .await;
        assert!(matches!(result, Err(TileServerError::SourceNotFound(_))));
    }

    #[tokio::test]
    async fn delete_upload_by_full_source_id_succeeds_and_removes_registry_entry() {
        let tempdir = tempfile::tempdir().unwrap();
        let file_path = tempdir.path().join("abc.mbtiles");
        tokio::fs::write(&file_path, b"fake").await.unwrap();

        let shared = build_shared(Some(tempdir.path().to_path_buf()));
        {
            let mut uploads = shared.uploads().write().await;
            uploads.insert(
                "upload-abc".to_string(),
                UploadInfo {
                    id: "abc".to_string(),
                    file_name: "abc.mbtiles".to_string(),
                    format: "mbtiles".to_string(),
                    file_path: file_path.clone(),
                },
            );
        }

        let status = delete_upload(
            axum::extract::State(shared.clone()),
            axum::extract::Path("upload-abc".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(shared.uploads().read().await.is_empty());
        assert!(!file_path.exists(), "uploaded file must be deleted");
    }

    #[tokio::test]
    async fn delete_upload_by_uuid_alias_succeeds() {
        let tempdir = tempfile::tempdir().unwrap();
        let file_path = tempdir.path().join("u-uuid.mbtiles");
        tokio::fs::write(&file_path, b"fake").await.unwrap();

        let shared = build_shared(Some(tempdir.path().to_path_buf()));
        {
            let mut uploads = shared.uploads().write().await;
            uploads.insert(
                "upload-uuid".to_string(),
                UploadInfo {
                    id: "uuid".to_string(),
                    file_name: "u.mbtiles".to_string(),
                    format: "mbtiles".to_string(),
                    file_path: file_path.clone(),
                },
            );
        }

        let status = delete_upload(
            axum::extract::State(shared.clone()),
            axum::extract::Path("uuid".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(shared.uploads().read().await.is_empty());
    }

    #[tokio::test]
    async fn delete_upload_missing_file_on_disk_still_succeeds() {
        let tempdir = tempfile::tempdir().unwrap();
        let phantom = tempdir.path().join("gone.mbtiles");

        let shared = build_shared(Some(tempdir.path().to_path_buf()));
        {
            let mut uploads = shared.uploads().write().await;
            uploads.insert(
                "upload-gone".to_string(),
                UploadInfo {
                    id: "gone".to_string(),
                    file_name: "gone.mbtiles".to_string(),
                    format: "mbtiles".to_string(),
                    file_path: phantom,
                },
            );
        }

        let status = delete_upload(
            axum::extract::State(shared.clone()),
            axum::extract::Path("upload-gone".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(shared.uploads().read().await.is_empty());
    }

    fn write_minimal_mbtiles(path: &std::path::Path) {
        let conn = rusqlite::Connection::open(path).expect("open sqlite");
        conn.execute_batch(
            "CREATE TABLE metadata (name TEXT, value TEXT);
             CREATE TABLE tiles (zoom_level INTEGER, tile_column INTEGER, tile_row INTEGER, tile_data BLOB);
             INSERT INTO metadata VALUES ('name', 'test');
             INSERT INTO metadata VALUES ('format', 'pbf');
             INSERT INTO metadata VALUES ('minzoom', '0');
             INSERT INTO metadata VALUES ('maxzoom', '5');
             INSERT INTO metadata VALUES ('bounds', '-180,-85,180,85');",
        )
        .expect("create mbtiles schema");
    }

    use axum_test::TestServer;
    use axum_test::multipart::{MultipartForm, Part};

    fn router_with_shared(shared: SharedState) -> axum::Router {
        crate::routes::api_router(shared)
    }

    #[tokio::test]
    async fn upload_post_unsupported_extension_returns_4xx() {
        let tempdir = tempfile::tempdir().unwrap();
        let shared = build_shared(Some(tempdir.path().to_path_buf()));
        let server = TestServer::new(router_with_shared(shared));

        let form = MultipartForm::new().add_part(
            "file",
            Part::bytes(b"hello".to_vec()).file_name("notes.txt"),
        );
        let resp = server.post("/api/upload").multipart(form).await;
        assert!(resp.status_code().is_client_error());
    }

    #[tokio::test]
    async fn upload_post_no_filename_uses_upload_default_and_fails_detect() {
        let tempdir = tempfile::tempdir().unwrap();
        let shared = build_shared(Some(tempdir.path().to_path_buf()));
        let server = TestServer::new(router_with_shared(shared));

        let form = MultipartForm::new().add_part("file", Part::bytes(b"hello".to_vec()));
        let resp = server.post("/api/upload").multipart(form).await;
        assert!(resp.status_code().is_client_error());
    }

    #[tokio::test]
    async fn upload_post_valid_mbtiles_registers_source_and_returns_url() {
        let tempdir = tempfile::tempdir().unwrap();

        let staging = tempdir.path().join("src.mbtiles");
        write_minimal_mbtiles(&staging);
        let bytes = std::fs::read(&staging).unwrap();
        std::fs::remove_file(&staging).unwrap();

        let shared = build_shared(Some(tempdir.path().to_path_buf()));
        let server = TestServer::new(router_with_shared(shared.clone()));

        let form =
            MultipartForm::new().add_part("file", Part::bytes(bytes).file_name("world.mbtiles"));
        let resp = server.post("/api/upload").multipart(form).await;
        assert_eq!(resp.status_code().as_u16(), 200, "body: {}", resp.text());

        let body: serde_json::Value = resp.json();
        assert_eq!(body["format"], "mbtiles");
        assert_eq!(body["file_name"], "world.mbtiles");
        let source_id = body["source_id"].as_str().unwrap().to_string();
        assert!(source_id.starts_with("upload-"));
        assert!(body["tilejson_url"].as_str().unwrap().ends_with(".json"));

        let registered = shared.load();
        assert!(registered.sources.exists(&source_id));

        let uploads = shared.uploads().read().await;
        assert_eq!(uploads.len(), 1);
        assert!(uploads.contains_key(&source_id));
    }

    #[tokio::test]
    async fn upload_post_corrupt_mbtiles_rolls_back_and_removes_temp_file() {
        let tempdir = tempfile::tempdir().unwrap();
        let shared = build_shared(Some(tempdir.path().to_path_buf()));
        let server = TestServer::new(router_with_shared(shared.clone()));

        let form = MultipartForm::new().add_part(
            "file",
            Part::bytes(b"not a sqlite database".to_vec()).file_name("broken.mbtiles"),
        );
        let resp = server.post("/api/upload").multipart(form).await;
        assert!(resp.status_code().is_client_error());

        let entries: Vec<_> = std::fs::read_dir(tempdir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert!(
            entries.is_empty(),
            "tempdir should be empty after failed upload, found: {entries:?}"
        );
        assert!(shared.uploads().read().await.is_empty());
    }
}
