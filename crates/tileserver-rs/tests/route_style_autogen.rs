//! Integration tests for auto-generated style JSON (#710).
//!
//! `GET /styles/{id}/style.json` returns a generated MapLibre v8 style when no
//! `[[styles]]` entry matches but a vector source with that id exists.

mod common;

use std::sync::Arc;

use common::{MockSource, server_with_sources};
use serde_json::Value;
use tileserver_rs::TileSource;

fn vector_source(id: &str) -> Arc<dyn TileSource> {
    Arc::new(MockSource::pbf(id).with_vector_layers(serde_json::json!([
        { "id": "buildings" },
        { "id": "roads" },
        { "id": "water" }
    ]))) as Arc<dyn TileSource>
}

#[tokio::test]
async fn style_json_unknown_style_no_source_returns_404() {
    let server = server_with_sources(vec![]);
    let res = server.get("/styles/ghost/style.json").await;
    res.assert_status_not_found();
}

#[tokio::test]
async fn style_json_unknown_style_with_vector_source_returns_200() {
    let server = server_with_sources(vec![vector_source("basemap")]);
    let res = server.get("/styles/basemap/style.json").await;
    res.assert_status_ok();
}

#[tokio::test]
async fn style_json_unknown_style_with_raster_source_returns_404() {
    let server = server_with_sources(vec![
        Arc::new(MockSource::png("imagery")) as Arc<dyn TileSource>
    ]);
    let res = server.get("/styles/imagery/style.json").await;
    res.assert_status_not_found();
}

#[tokio::test]
async fn style_json_auto_gen_response_is_valid_maplibre_spec() {
    let server = server_with_sources(vec![vector_source("basemap")]);
    let res = server.get("/styles/basemap/style.json").await;
    let body: Value = res.json();
    assert_eq!(body["version"], 8);
    assert!(body["sources"].is_object());
    assert!(body["layers"].is_array());
    assert!(body["glyphs"].is_string());
}

#[tokio::test]
async fn style_json_auto_gen_layers_match_vector_layers_geometry() {
    let server = server_with_sources(vec![vector_source("basemap")]);
    let res = server.get("/styles/basemap/style.json").await;
    let body: Value = res.json();
    let layers = body["layers"].as_array().unwrap();
    // background + 3 source-layers * 3 kinds
    assert_eq!(layers.len(), 1 + 3 * 3);
}

#[tokio::test]
async fn style_json_auto_gen_url_points_at_data_tilejson() {
    let server = server_with_sources(vec![vector_source("basemap")]);
    let res = server.get("/styles/basemap/style.json").await;
    let body: Value = res.json();
    let url = body["sources"]["basemap"]["url"].as_str().unwrap();
    assert!(url.contains("/data/basemap.json"), "got {url}");
}

#[tokio::test]
async fn style_json_auto_gen_forwards_key_query() {
    let server = server_with_sources(vec![vector_source("basemap")]);
    let res = server.get("/styles/basemap/style.json?key=secret").await;
    let body: Value = res.json();
    let url = body["sources"]["basemap"]["url"].as_str().unwrap();
    assert!(url.contains("?key=secret"), "got {url}");
}

#[tokio::test]
async fn style_json_auto_gen_naming_convention() {
    let server = server_with_sources(vec![vector_source("basemap")]);
    let res = server.get("/styles/basemap/style.json").await;
    let body: Value = res.json();
    assert_eq!(body["name"], "basemap (auto)");
}

#[tokio::test]
async fn style_json_auto_gen_with_mlt_source_uses_pbf_tile_url() {
    let src = Arc::new(
        MockSource::mlt("vectormlt").with_vector_layers(serde_json::json!([{ "id": "roads" }])),
    ) as Arc<dyn TileSource>;
    let server = server_with_sources(vec![src]);
    let res = server.get("/styles/vectormlt/style.json").await;
    res.assert_status_ok();
    let body: Value = res.json();
    let tiles = body["sources"]["vectormlt"]["tiles"].as_array().unwrap();
    assert!(tiles[0].as_str().unwrap().ends_with("/{z}/{x}/{y}.pbf"));
}
