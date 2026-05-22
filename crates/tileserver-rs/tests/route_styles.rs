//! Integration tests for style routes.
//!
//! Covers:
//! - GET /styles.json (list)
//! - GET /styles/{style_json} (TileJSON for raster tiles)
//! - GET /styles/{style}/style.json (full MapLibre style spec)
//! - GET /styles/{style}/wmts.xml (WMTS capabilities)
//! - GET /styles/{style}/{sprite_file} (sprite validation + 404 paths)

mod common;

use axum_test::TestServer;
use std::path::PathBuf;
use std::sync::Arc;
use tileserver_rs::{
    config::{Config, StyleConfig},
    reload::{ReloadController, SharedState},
    routes::api_router,
    styles::StyleManager,
};

/// Build a [`TestServer`] backed by a real style fixture (`protomaps-light`).
///
/// The style is resolved relative to `CARGO_MANIFEST_DIR` so the test passes
/// regardless of the current working directory.
fn style_test_server() -> TestServer {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let style_path = manifest_dir.join("../../data/styles/protomaps-light/style.json");
    assert!(
        style_path.exists(),
        "missing style fixture at {}",
        style_path.display()
    );

    let style_config = StyleConfig {
        id: "protomaps-light".to_string(),
        path: style_path,
        name: Some("Protomaps Light".to_string()),
    };

    let styles = StyleManager::from_configs(&[style_config]).expect("load styles");

    let mut state = common::minimal_app_state();
    state.styles = Arc::new(styles);

    let meta = common::minimal_meta();
    let runtime = common::minimal_runtime();
    let controller = Arc::new(ReloadController::new(
        state,
        meta,
        Config::default(),
        None,
        runtime,
    ));
    let shared = SharedState::new(controller);
    let router = api_router(shared);
    TestServer::new(router)
}

#[tokio::test]
async fn styles_json_empty_returns_200() {
    let server = common::empty_test_server();
    let response = server.get("/styles.json").await;
    response.assert_status_ok();
}

#[tokio::test]
async fn styles_json_empty_returns_empty_array() {
    let server = common::empty_test_server();
    let response = server.get("/styles.json").await;
    let body: serde_json::Value = response.json();
    assert!(body.is_array(), "/styles.json must return a JSON array");
    assert_eq!(
        body.as_array().unwrap().len(),
        0,
        "empty state must yield empty array"
    );
}

#[tokio::test]
async fn style_json_unknown_style_returns_not_found() {
    let server = common::empty_test_server();
    let response = server.get("/styles/nonexistent/style.json").await;
    assert_eq!(
        response.status_code().as_u16(),
        404,
        "unknown style must return 404"
    );
}

#[tokio::test]
async fn style_tilejson_unknown_style_returns_not_found() {
    let server = common::empty_test_server();
    let response = server.get("/styles/nonexistent.json").await;
    assert_eq!(
        response.status_code().as_u16(),
        404,
        "unknown style TileJSON must return 404"
    );
}

#[tokio::test]
async fn style_tilejson_without_json_suffix_returns_not_found() {
    let server = common::empty_test_server();
    let response = server.get("/styles/no-suffix").await;
    assert_eq!(
        response.status_code().as_u16(),
        404,
        "path without .json suffix must return 404"
    );
}

#[tokio::test]
async fn wmts_unknown_style_returns_not_found() {
    let server = common::empty_test_server();
    let response = server.get("/styles/nonexistent/wmts.xml").await;
    assert_eq!(
        response.status_code().as_u16(),
        404,
        "unknown style WMTS must return 404"
    );
}

#[tokio::test]
async fn sprite_unknown_style_returns_not_found() {
    let server = common::empty_test_server();
    let response = server.get("/styles/nonexistent/sprite.png").await;
    assert_eq!(
        response.status_code().as_u16(),
        404,
        "unknown-style sprite must return 404"
    );
}

#[tokio::test]
async fn styles_json_with_fixture_returns_one_entry() {
    let server = style_test_server();
    let response = server.get("/styles.json").await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body.is_array());
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn styles_json_entry_has_id() {
    let server = style_test_server();
    let response = server.get("/styles.json").await;
    let body: serde_json::Value = response.json();
    let entry = &body[0];
    assert_eq!(entry["id"], "protomaps-light");
}

#[tokio::test]
async fn styles_json_entry_has_name() {
    let server = style_test_server();
    let response = server.get("/styles.json").await;
    let body: serde_json::Value = response.json();
    let entry = &body[0];
    assert!(entry["name"].is_string(), "name must be a string");
    assert!(
        !entry["name"].as_str().unwrap().is_empty(),
        "name must not be empty"
    );
}

#[tokio::test]
async fn styles_json_entry_has_absolute_style_url() {
    let server = style_test_server();
    let response = server.get("/styles.json").await;
    let body: serde_json::Value = response.json();
    let url = body[0]["url"].as_str().expect("url must be present");
    assert!(
        url.starts_with("http://"),
        "url must be absolute (was {url})"
    );
    assert!(
        url.contains("/styles/protomaps-light/style.json"),
        "url must point at style.json (was {url})"
    );
}

#[tokio::test]
async fn styles_json_with_key_param_forwards_key_to_url() {
    let server = style_test_server();
    let response = server.get("/styles.json?key=myapikey").await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let url = body[0]["url"].as_str().expect("url must be present");
    assert!(
        url.contains("key=myapikey"),
        "url must contain api key query param (was {url})"
    );
}

#[tokio::test]
async fn styles_json_url_encodes_key_param() {
    let server = style_test_server();
    let response = server.get("/styles.json?key=needs%20escaping").await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let url = body[0]["url"].as_str().expect("url must be present");
    // urlencoding crate encodes ' ' as %20
    assert!(
        url.contains("key=needs%20escaping"),
        "url must url-encode the key (was {url})"
    );
}

#[tokio::test]
async fn style_json_found_returns_200() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light/style.json").await;
    response.assert_status_ok();
}

#[tokio::test]
async fn style_json_returns_spec_with_version() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light/style.json").await;
    let body: serde_json::Value = response.json();
    assert!(
        body["version"].is_number(),
        "MapLibre style spec must have numeric version"
    );
}

#[tokio::test]
async fn style_json_has_sources_field() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light/style.json").await;
    let body: serde_json::Value = response.json();
    assert!(
        body["sources"].is_object(),
        "MapLibre style spec must have sources object"
    );
}

#[tokio::test]
async fn style_json_has_layers_array() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light/style.json").await;
    let body: serde_json::Value = response.json();
    assert!(
        body["layers"].is_array(),
        "MapLibre style spec must have layers array"
    );
}

#[tokio::test]
async fn style_json_forwards_key_param() {
    let server = style_test_server();
    let response = server
        .get("/styles/protomaps-light/style.json?key=secret")
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["version"].is_number());
}

#[tokio::test]
async fn style_tilejson_found_returns_200() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light.json").await;
    response.assert_status_ok();
}

#[tokio::test]
async fn style_tilejson_has_tilejson_version() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light.json").await;
    let body: serde_json::Value = response.json();
    assert_eq!(
        body["tilejson"], "3.0.0",
        "raster TileJSON must report tilejson 3.0.0"
    );
}

#[tokio::test]
async fn style_tilejson_has_tiles_array() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light.json").await;
    let body: serde_json::Value = response.json();
    let tiles = body["tiles"].as_array().expect("tiles must be array");
    assert_eq!(tiles.len(), 1);
    let tile_url = tiles[0].as_str().expect("tile url must be a string");
    assert!(
        tile_url.contains("/styles/protomaps-light/"),
        "tile url must reference the style (was {tile_url})"
    );
    assert!(
        tile_url.contains("{z}") && tile_url.contains("{x}") && tile_url.contains("{y}"),
        "tile url must contain {{z}}/{{x}}/{{y}} placeholders (was {tile_url})"
    );
}

#[tokio::test]
async fn style_tilejson_has_zoom_range() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light.json").await;
    let body: serde_json::Value = response.json();
    assert_eq!(body["minzoom"], 0);
    assert_eq!(body["maxzoom"], 22);
}

#[tokio::test]
async fn style_tilejson_key_param_appears_in_tile_url() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light.json?key=abc").await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    let tile_url = body["tiles"][0].as_str().expect("tile url");
    assert!(
        tile_url.contains("key=abc"),
        "tile url must carry forwarded key (was {tile_url})"
    );
}

#[tokio::test]
async fn wmts_found_returns_200() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light/wmts.xml").await;
    response.assert_status_ok();
}

#[tokio::test]
async fn wmts_returns_xml_content_type() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light/wmts.xml").await;
    let ct = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(
        ct.contains("xml"),
        "wmts.xml must return an XML content-type (was {ct})"
    );
}

#[tokio::test]
async fn wmts_body_contains_capabilities_root() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light/wmts.xml").await;
    let body = response.text();
    assert!(
        body.contains("Capabilities") || body.contains("capabilities"),
        "WMTS body must mention Capabilities root element"
    );
}

#[tokio::test]
async fn wmts_body_includes_style_id() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light/wmts.xml").await;
    let body = response.text();
    assert!(
        body.contains("protomaps-light"),
        "WMTS body must reference the style id"
    );
}

#[tokio::test]
async fn wmts_with_key_param_is_accepted() {
    let server = style_test_server();
    let response = server
        .get("/styles/protomaps-light/wmts.xml?key=mywmtskey")
        .await;
    response.assert_status_ok();
}

#[tokio::test]
async fn sprite_rejects_non_sprite_filename() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light/other.png").await;
    let status = response.status_code().as_u16();
    assert!(
        status == 400 || status == 404,
        "non-sprite filename must be rejected (got {status})"
    );
}

#[tokio::test]
async fn sprite_rejects_unsupported_extension() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light/sprite.txt").await;
    let status = response.status_code().as_u16();
    assert!(
        status == 400 || status == 404,
        "unsupported sprite extension must be rejected (got {status})"
    );
}

#[tokio::test]
async fn sprite_missing_file_returns_404() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light/sprite.png").await;
    assert_eq!(
        response.status_code().as_u16(),
        404,
        "missing sprite file must return 404"
    );
}

#[tokio::test]
async fn sprite_retina_filename_missing_returns_404() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light/sprite@2x.png").await;
    assert_eq!(
        response.status_code().as_u16(),
        404,
        "missing @2x sprite must return 404"
    );
}

#[tokio::test]
async fn sprite_json_metadata_missing_returns_404() {
    let server = style_test_server();
    let response = server.get("/styles/protomaps-light/sprite.json").await;
    assert_eq!(
        response.status_code().as_u16(),
        404,
        "missing sprite.json must return 404"
    );
}
