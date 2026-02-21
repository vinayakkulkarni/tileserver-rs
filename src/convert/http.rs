use axum::{
    body::Body,
    extract::{Extension, Multipart, Path},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde_json::json;
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

use crate::convert::{
    job::{ConvertJob, ConvertState, JobStatus},
    pipeline::{run, ConvertOptions},
    progress::HttpProgressReporter,
};

/// POST /convert
///
/// Accepts a multipart upload with a GeoJSON file.
/// Starts a background job and returns the job ID immediately.
///
/// Form fields:
/// - `file`: the GeoJSON file bytes (required)
/// - `min_zoom`: minimum zoom level (optional, default 0)
/// - `max_zoom`: maximum zoom level (optional, default 14)
/// - `layer_name`: MVT layer name (optional)
pub async fn start_conversion(
    Extension(state): Extension<ConvertState>,
    mut multipart: Multipart,
) -> Response {
    // Parse multipart fields
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name = String::from("layer");
    let mut min_zoom: u8 = 0;
    let mut max_zoom: Option<u8> = None;
    let mut layer_name: Option<String> = None;
    let mut id_property: Option<String> = None;
    let mut include_properties: Option<Vec<String>> = None;
    let mut exclude_properties: Vec<String> = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_owned();
        match name.as_str() {
            "file" => {
                if let Some(fname) = field.file_name() {
                    // Extract stem for default layer name
                    file_name = std::path::Path::new(fname)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("layer")
                        .to_owned();
                }
                match field.bytes().await {
                    Ok(b) => file_bytes = Some(b.to_vec()),
                    Err(e) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"error": format!("Failed to read file: {e}")})),
                        )
                            .into_response();
                    }
                }
            }
            "min_zoom" => {
                if let Ok(b) = field.bytes().await {
                    if let Ok(s) = std::str::from_utf8(&b) {
                        min_zoom = s.trim().parse().unwrap_or(0);
                    }
                }
            }
            "max_zoom" => {
                if let Ok(b) = field.bytes().await {
                    if let Ok(s) = std::str::from_utf8(&b) {
                        max_zoom = s.trim().parse().ok();
                    }
                }
            }
            "layer_name" => {
                if let Ok(b) = field.bytes().await {
                    if let Ok(s) = std::str::from_utf8(&b) {
                        layer_name = Some(s.trim().to_owned());
                    }
                }
            }
            "id_property" => {
                if let Ok(b) = field.bytes().await {
                    if let Ok(s) = std::str::from_utf8(&b) {
                        let v = s.trim().to_owned();
                        if !v.is_empty() {
                            id_property = Some(v);
                        }
                    }
                }
            }
            "include_properties" => {
                if let Ok(b) = field.bytes().await {
                    if let Ok(s) = std::str::from_utf8(&b) {
                        let props: Vec<String> = s
                            .split(',')
                            .map(str::trim)
                            .filter(|p| !p.is_empty())
                            .map(str::to_owned)
                            .collect();
                        if !props.is_empty() {
                            include_properties = Some(props);
                        }
                    }
                }
            }
            "exclude_properties" => {
                if let Ok(b) = field.bytes().await {
                    if let Ok(s) = std::str::from_utf8(&b) {
                        exclude_properties = s
                            .split(',')
                            .map(str::trim)
                            .filter(|p| !p.is_empty())
                            .map(str::to_owned)
                            .collect();
                    }
                }
            }
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    let Some(bytes) = file_bytes else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Missing 'file' field in multipart form"})),
        )
            .into_response();
    };

    let id = Uuid::new_v4().to_string();
    let job = ConvertJob::new(&id);
    let progress = Arc::clone(&job.progress);
    state.insert(job);

    let resolved_layer = layer_name.unwrap_or(file_name);
    let output_path = state.temp_dir().join(format!("{id}.pmtiles"));
    let input_path = state.temp_dir().join(format!("{id}.geojson"));

    // Write uploaded file to temp location
    if let Err(e) = fs::write(&input_path, &bytes).await {
        state.update(&id, |job| {
            job.status = JobStatus::Failed;
            job.error = Some(format!("Failed to write temp file: {e}"));
        });
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to store uploaded file"})),
        )
            .into_response();
    }

    let opts = ConvertOptions {
        min_zoom,
        max_zoom,
        layer_name: resolved_layer,
        simplification: None,
        id_property,
        include_properties,
        exclude_properties,
    };

    let state_clone = state.clone();
    let id_clone = id.clone();
    let output_clone = output_path.clone();

    // Run conversion in a blocking thread (CPU-bound work)
    tokio::task::spawn_blocking(move || {
        let reporter = HttpProgressReporter::new(progress);
        match run(&input_path, &output_clone, &opts, &reporter) {
            Ok(()) => {
                state_clone.update(&id_clone, |job| {
                    job.status = JobStatus::Done;
                    job.output_path = Some(output_clone);
                });
            }
            Err(e) => {
                state_clone.update(&id_clone, |job| {
                    job.status = JobStatus::Failed;
                    job.error = Some(e.to_string());
                });
            }
        }
        // Clean up the input temp file regardless
        let _ = std::fs::remove_file(&input_path);
    });

    let progress_url = format!("/convert/{id}/status");
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "id": id,
            "status": "processing",
            "progress_url": progress_url,
        })),
    )
        .into_response()
}

/// GET /convert/{id}/status
pub async fn job_status(
    Path(id): Path<String>,
    Extension(state): Extension<ConvertState>,
) -> Response {
    match state.get_status(&id) {
        Some(status) => Json(status).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Job not found"})),
        )
            .into_response(),
    }
}

/// GET /convert/{id}/download
///
/// Returns the PMTiles file when the job is done.
pub async fn download_result(
    Path(id): Path<String>,
    Extension(state): Extension<ConvertState>,
) -> Response {
    let Some(output_path) = state.get_output_path(&id) else {
        let status = state.get_status(&id);
        return match status {
            None => (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Job not found"})),
            )
                .into_response(),
            Some(s) if s.status == JobStatus::Processing => (
                StatusCode::ACCEPTED,
                Json(json!({"error": "Job still processing", "progress": s.progress})),
            )
                .into_response(),
            Some(s) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": s.error.unwrap_or_else(|| "Conversion failed".into())})),
            )
                .into_response(),
        };
    };

    let file_name = output_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("result.pmtiles");

    match fs::read(&output_path).await {
        Ok(bytes) => {
            let content_disposition = format!("attachment; filename=\"{file_name}\"");
            (
                [
                    (header::CONTENT_TYPE, "application/octet-stream"),
                    (header::CONTENT_DISPOSITION, &content_disposition),
                ],
                Body::from(bytes),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Failed to read output file: {e}")})),
        )
            .into_response(),
    }
}
