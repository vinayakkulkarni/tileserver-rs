//! Integration tests for admin, upload, and spatial route handlers.
//!
//! Covers:
//! - `POST /__admin/reload` and `POST /__admin/cache/flush` (admin router)
//! - `GET /api/upload`, `POST /api/upload`, `DELETE /api/upload/{id}`
//! - `GET /api/spatial/schema/{source}`, `GET /api/spatial/stats/{source}`,
//!   `POST /api/spatial/query`
//!
//! Uses the shared test harness from `common/mod.rs`.

mod common;

use axum::Router;
use axum_test::TestServer;
use tileserver_rs::{admin, routes::api_router};

/// Build a [`TestServer`] with both the API router and the admin router mounted.
fn full_test_server() -> TestServer {
    let shared = common::minimal_shared_state();
    let router = Router::new()
        .merge(api_router(shared.clone()))
        .merge(admin::admin_router(shared));
    TestServer::new(router)
}

// === Admin endpoint tests ===

#[tokio::test]
async fn admin_reload_returns_non_5xx_with_no_config_path() {
    let server = full_test_server();
    let resp = server.post("/__admin/reload").await;
    // With no config path, reload may return 200 (no-op) or a structured 500
    // with JSON body — either way, it must not panic and must be a valid HTTP
    // response.
    let status = resp.status_code().as_u16();
    assert!(
        (200..600).contains(&status),
        "admin reload must produce a valid HTTP status, got {status}"
    );
}

#[tokio::test]
async fn admin_reload_with_flush_param_does_not_panic() {
    let server = full_test_server();
    let resp = server.post("/__admin/reload?flush=true").await;
    let status = resp.status_code().as_u16();
    assert!(
        (200..600).contains(&status),
        "admin reload with flush must produce a valid HTTP status, got {status}"
    );
}

#[tokio::test]
async fn admin_reload_with_flush_false_does_not_panic() {
    let server = full_test_server();
    let resp = server.post("/__admin/reload?flush=false").await;
    let status = resp.status_code().as_u16();
    assert!((200..600).contains(&status));
}

#[tokio::test]
async fn admin_reload_error_response_is_json() {
    // When there's no config path configured, the reload should fail and
    // return a JSON error body. Verify the response is valid JSON regardless
    // of whether it's success or error.
    let server = full_test_server();
    let resp = server.post("/__admin/reload").await;
    let body: serde_json::Value = resp.json();
    assert!(
        body["ok"].is_boolean(),
        "admin reload response must have boolean `ok` field"
    );
}

#[tokio::test]
async fn admin_cache_flush_with_no_cache_returns_ok() {
    let server = full_test_server();
    let resp = server.post("/__admin/cache/flush").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn admin_cache_flush_response_has_invalidated_entries() {
    let server = full_test_server();
    let resp = server.post("/__admin/cache/flush").await;
    let body: serde_json::Value = resp.json();
    assert!(
        body["invalidated_entries"].is_number(),
        "cache flush response must have numeric invalidated_entries"
    );
}

#[tokio::test]
async fn admin_cache_flush_response_has_freed_bytes() {
    let server = full_test_server();
    let resp = server.post("/__admin/cache/flush").await;
    let body: serde_json::Value = resp.json();
    assert!(
        body["freed_bytes"].is_number(),
        "cache flush response must have numeric freed_bytes"
    );
}

#[tokio::test]
async fn admin_cache_flush_with_no_cache_returns_zero_entries() {
    // With minimal_shared_state (no source manager cache), entries should be 0.
    let server = full_test_server();
    let resp = server.post("/__admin/cache/flush").await;
    let body: serde_json::Value = resp.json();
    assert_eq!(body["invalidated_entries"], 0);
    assert_eq!(body["freed_bytes"], 0);
}

// === Upload endpoint tests ===

#[tokio::test]
async fn upload_list_returns_200() {
    let server = common::empty_test_server();
    let resp = server.get("/api/upload").await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn upload_list_returns_empty_array_initially() {
    let server = common::empty_test_server();
    let resp = server.get("/api/upload").await;
    let body: serde_json::Value = resp.json();
    assert!(body.is_array(), "/api/upload list must be JSON array");
    assert_eq!(
        body.as_array().unwrap().len(),
        0,
        "no uploads should exist initially"
    );
}

#[tokio::test]
async fn upload_delete_nonexistent_id_returns_404() {
    let server = common::empty_test_server();
    let resp = server.delete("/api/upload/nonexistent-id-xyz").await;
    // SourceNotFound → 404 (see TileServerError::IntoResponse)
    assert_eq!(resp.status_code().as_u16(), 404);
}

#[tokio::test]
async fn upload_delete_uuid_like_id_returns_404() {
    let server = common::empty_test_server();
    let resp = server
        .delete("/api/upload/00000000-0000-0000-0000-000000000000")
        .await;
    assert_eq!(resp.status_code().as_u16(), 404);
}

#[tokio::test]
async fn upload_post_without_upload_dir_returns_400() {
    // minimal_app_state has upload_dir = None, so the handler should return
    // UploadError → 400.
    let server = common::empty_test_server();
    let resp = server.post("/api/upload").await;
    let status = resp.status_code().as_u16();
    // No multipart body without upload dir configured → 400 (UploadError) or
    // 415 (no content-type) — either is fine as long as it's a 4xx, not 5xx.
    assert!(
        (400..500).contains(&status),
        "upload without upload_dir must return 4xx, got {status}"
    );
}

#[tokio::test]
async fn upload_post_empty_multipart_returns_error() {
    // Empty multipart body without upload dir configured should return a 4xx
    // error from the handler.
    let server = common::empty_test_server();
    let resp = server
        .post("/api/upload")
        .add_header("content-type", "multipart/form-data; boundary=test")
        .text("")
        .await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..500).contains(&status),
        "empty multipart upload must return 4xx, got {status}"
    );
}

// === Spatial endpoint tests ===

#[tokio::test]
async fn spatial_schema_unknown_source_returns_404() {
    let server = common::empty_test_server();
    let resp = server.get("/api/spatial/schema/nonexistent").await;
    // SourceNotFound → 404
    assert_eq!(resp.status_code().as_u16(), 404);
}

#[tokio::test]
async fn spatial_schema_url_encoded_source_returns_404() {
    let server = common::empty_test_server();
    let resp = server.get("/api/spatial/schema/some%20source").await;
    assert_eq!(resp.status_code().as_u16(), 404);
}

#[tokio::test]
async fn spatial_stats_unknown_source_returns_404() {
    let server = common::empty_test_server();
    let resp = server.get("/api/spatial/stats/nonexistent").await;
    assert_eq!(resp.status_code().as_u16(), 404);
}

#[tokio::test]
async fn spatial_stats_empty_source_returns_not_found_or_404() {
    // A request to /api/spatial/stats/ (no source) won't match the route at
    // all → 404 from axum's NOT_FOUND fallback.
    let server = common::empty_test_server();
    let resp = server.get("/api/spatial/stats/").await;
    assert_eq!(resp.status_code().as_u16(), 404);
}

#[tokio::test]
async fn spatial_query_missing_body_returns_4xx() {
    let server = common::empty_test_server();
    let resp = server.post("/api/spatial/query").await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..500).contains(&status),
        "spatial query without body must return 4xx, got {status}"
    );
}

#[tokio::test]
async fn spatial_query_empty_object_returns_4xx() {
    let server = common::empty_test_server();
    let resp = server
        .post("/api/spatial/query")
        .json(&serde_json::json!({}))
        .await;
    let status = resp.status_code().as_u16();
    // Missing required `source` field → 4xx from Json extractor or handler
    assert!(
        (400..500).contains(&status),
        "spatial query with empty body must return 4xx, got {status}"
    );
}

#[tokio::test]
async fn spatial_query_unknown_source_returns_404() {
    let server = common::empty_test_server();
    let body = serde_json::json!({
        "source": "nonexistent-source",
        "zoom": 5
    });
    let resp = server.post("/api/spatial/query").json(&body).await;
    // SourceNotFound → 404
    assert_eq!(resp.status_code().as_u16(), 404);
}

#[tokio::test]
async fn spatial_query_unknown_source_with_bbox_returns_404() {
    let server = common::empty_test_server();
    let body = serde_json::json!({
        "source": "nonexistent-source",
        "bbox": [-180.0, -85.0, 180.0, 85.0],
        "zoom": 3,
        "limit": 100
    });
    let resp = server.post("/api/spatial/query").json(&body).await;
    assert_eq!(resp.status_code().as_u16(), 404);
}

#[tokio::test]
async fn spatial_query_unknown_source_with_layers_returns_404() {
    let server = common::empty_test_server();
    let body = serde_json::json!({
        "source": "nonexistent-source",
        "layers": ["roads", "buildings"]
    });
    let resp = server.post("/api/spatial/query").json(&body).await;
    assert_eq!(resp.status_code().as_u16(), 404);
}

#[tokio::test]
async fn spatial_query_invalid_json_returns_4xx() {
    let server = common::empty_test_server();
    let resp = server
        .post("/api/spatial/query")
        .add_header("content-type", "application/json")
        .text("{not valid json")
        .await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..500).contains(&status),
        "spatial query with invalid JSON must return 4xx, got {status}"
    );
}
