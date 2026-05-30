//! Integration tests for core routes: /health, /ping, /index.json, /data.json
//!
//! Uses the shared test harness from `common/mod.rs`.

mod common;

#[tokio::test]
async fn health_returns_200() {
    let server = common::empty_test_server();
    let response = server.get("/health").await;
    response.assert_status_ok();
}

#[tokio::test]
async fn health_returns_ok_text() {
    let server = common::empty_test_server();
    let response = server.get("/health").await;
    response.assert_text("OK");
}

#[tokio::test]
async fn ping_returns_200() {
    let server = common::empty_test_server();
    let response = server.get("/ping").await;
    response.assert_status_ok();
}

#[tokio::test]
async fn ping_json_has_status_ok() {
    let server = common::empty_test_server();
    let response = server.get("/ping").await;
    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn ping_json_has_version() {
    let server = common::empty_test_server();
    let response = server.get("/ping").await;
    let body: serde_json::Value = response.json();
    assert!(
        body["version"].is_string(),
        "ping response must have version field"
    );
    assert!(
        !body["version"].as_str().unwrap().is_empty(),
        "version must not be empty"
    );
}

#[tokio::test]
async fn ping_json_has_config_hash() {
    let server = common::empty_test_server();
    let response = server.get("/ping").await;
    let body: serde_json::Value = response.json();
    assert!(
        body["config_hash"].is_string(),
        "ping response must have config_hash"
    );
}

#[tokio::test]
async fn ping_json_loaded_sources_is_zero_on_empty_state() {
    let server = common::empty_test_server();
    let response = server.get("/ping").await;
    let body: serde_json::Value = response.json();
    assert_eq!(
        body["loaded_sources"], 0,
        "empty state must report 0 sources"
    );
}

#[tokio::test]
async fn ping_json_renderer_disabled_on_empty_state() {
    let server = common::empty_test_server();
    let response = server.get("/ping").await;
    let body: serde_json::Value = response.json();
    assert_eq!(
        body["renderer_enabled"], false,
        "renderer must be disabled in empty state"
    );
}

#[tokio::test]
async fn ping_json_exposes_capability_flags() {
    let server = common::empty_test_server();
    let body: serde_json::Value = server.get("/ping").await.json();
    // Defaults: render + OGC + compression all enabled, codecs at default levels.
    assert_eq!(body["render_enabled"], true);
    assert_eq!(body["ogc_enabled"], true);
    assert_eq!(body["compression_enabled"], true);
    assert_eq!(body["compression_br_quality"], 5);
    assert_eq!(body["compression_zstd_level"], 3);
    assert!(
        body["cors_origins"].is_array(),
        "cors_origins must be a JSON array"
    );
    assert!(
        body["cache_dir"].as_str().is_some_and(|s| !s.is_empty()),
        "cache_dir must be a non-empty resolved path"
    );
}

#[tokio::test]
async fn data_json_empty_state_returns_empty_array() {
    let server = common::empty_test_server();
    let response = server.get("/data.json").await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body.is_array(), "/data.json must return JSON array");
    assert_eq!(
        body.as_array().unwrap().len(),
        0,
        "no sources in empty state"
    );
}

#[tokio::test]
async fn data_json_content_type_is_json() {
    let server = common::empty_test_server();
    let response = server.get("/data.json").await;
    let ct = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.contains("application/json"),
        "content-type must be application/json, got: {ct}"
    );
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let server = common::empty_test_server();
    let response = server.get("/nonexistent/route/xyz").await;
    response.assert_status_not_found();
}

#[tokio::test]
async fn index_json_empty_state_returns_array() {
    let server = common::empty_test_server();
    let response = server.get("/index.json").await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body.is_array(), "/index.json must return JSON array");
}

#[tokio::test]
async fn index_json_with_key_param_works() {
    let server = common::empty_test_server();
    let response = server.get("/index.json?key=testkey123").await;
    response.assert_status_ok();
}

#[tokio::test]
async fn styles_json_empty_state_returns_empty_array() {
    let server = common::empty_test_server();
    let response = server.get("/styles.json").await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn data_source_not_found_returns_404() {
    let server = common::empty_test_server();
    let response = server.get("/data/nonexistent-source").await;
    response.assert_status_not_found();
}

#[tokio::test]
async fn data_tile_not_found_returns_404() {
    let server = common::empty_test_server();
    let response = server.get("/data/nonexistent-source/0/0/0.pbf").await;
    response.assert_status_not_found();
}
