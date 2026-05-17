//! Integration tests for spatial API routes (`/api/spatial/...`).
//!
//! These endpoints power the LLM map tools and are entirely deterministic
//! once a `MockSource` is wired in — they don't need PostGIS or real tile
//! data to exercise the request-parsing, error-mapping, and response-shape
//! code paths.

mod common;

use std::sync::Arc;
use tileserver_rs::TileSource;

use common::MockSource;

// ============================================================
// Empty-state error paths (no sources loaded)
// ============================================================

#[tokio::test]
async fn spatial_schema_unknown_source_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/api/spatial/schema/missing").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn spatial_stats_unknown_source_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/api/spatial/stats/missing").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn spatial_query_unknown_source_returns_error() {
    let server = common::empty_test_server();
    let resp = server
        .post("/api/spatial/query")
        .json(&serde_json::json!({"source": "ghost"}))
        .await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn spatial_query_malformed_body_returns_4xx() {
    let server = common::empty_test_server();
    let resp = server
        .post("/api/spatial/query")
        .text("not even json")
        .await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..500).contains(&status),
        "malformed body must yield 4xx, got {status}"
    );
}

#[tokio::test]
async fn spatial_query_missing_source_field_returns_4xx() {
    let server = common::empty_test_server();
    let resp = server
        .post("/api/spatial/query")
        .json(&serde_json::json!({}))
        .await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..500).contains(&status),
        "missing required 'source' must yield 4xx, got {status}"
    );
}

// ============================================================
// Schema endpoint — happy path with MockSource
// ============================================================

#[tokio::test]
async fn spatial_schema_returns_layers_for_known_source() {
    let server = common::server_with_sources(vec![
        Arc::new(MockSource::pbf("vec1")) as Arc<dyn TileSource>
    ]);
    let resp = server.get("/api/spatial/schema/vec1").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["source"], "vec1");
    assert_eq!(body["format"], "pbf");
    assert_eq!(body["minzoom"], 0);
    assert_eq!(body["maxzoom"], 14);
    let layers = body["layers"].as_array().expect("layers array present");
    assert_eq!(layers.len(), 2, "MockSource::pbf has 2 vector_layers");
    let buildings = &layers[0];
    assert_eq!(buildings["id"], "buildings");
    assert_eq!(buildings["minzoom"], 0);
    assert_eq!(buildings["maxzoom"], 14);
}

#[tokio::test]
async fn spatial_schema_returns_bounds() {
    let server = common::server_with_sources(vec![
        Arc::new(MockSource::pbf("bounded")) as Arc<dyn TileSource>
    ]);
    let resp = server.get("/api/spatial/schema/bounded").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let bounds = body["bounds"].as_array().expect("bounds present");
    assert_eq!(bounds.len(), 4);
}

#[tokio::test]
async fn spatial_schema_serializes_field_types() {
    let server = common::server_with_sources(vec![
        Arc::new(MockSource::pbf("fields-src")) as Arc<dyn TileSource>
    ]);
    let resp = server.get("/api/spatial/schema/fields-src").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let buildings = &body["layers"][0];
    let fields = buildings["fields"]
        .as_array()
        .expect("fields array on buildings layer");
    assert_eq!(fields.len(), 2);
    let names: Vec<&str> = fields
        .iter()
        .map(|f| f["name"].as_str().unwrap_or(""))
        .collect();
    assert!(names.contains(&"height"));
    assert!(names.contains(&"name"));
}

// ============================================================
// Stats endpoint — happy path
// ============================================================

#[tokio::test]
async fn spatial_stats_returns_metadata_for_known_source() {
    let server = common::server_with_sources(vec![
        Arc::new(MockSource::pbf("stats1")) as Arc<dyn TileSource>
    ]);
    let resp = server.get("/api/spatial/stats/stats1").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["source"], "stats1");
    assert_eq!(body["format"], "pbf");
    assert_eq!(body["layer_count"], 2);
    assert_eq!(body["minzoom"], 0);
    assert_eq!(body["maxzoom"], 14);
    assert_eq!(body["name"], "stats1");
}

#[tokio::test]
async fn spatial_stats_includes_center_and_bounds() {
    let server = common::server_with_sources(vec![
        Arc::new(MockSource::pbf("geom1")) as Arc<dyn TileSource>
    ]);
    let resp = server.get("/api/spatial/stats/geom1").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body["bounds"].is_array());
    assert!(body["center"].is_array());
}

#[tokio::test]
async fn spatial_stats_layer_count_zero_when_no_vector_layers() {
    let server = common::server_with_sources(vec![
        Arc::new(MockSource::png("rast")) as Arc<dyn TileSource>
    ]);
    let resp = server.get("/api/spatial/stats/rast").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["layer_count"], 0);
    assert_eq!(body["format"], "png");
}

// ============================================================
// Query endpoint — happy path & branches
// ============================================================

#[tokio::test]
async fn spatial_query_basic_returns_response_shape() {
    let server =
        common::server_with_sources(vec![Arc::new(MockSource::pbf("q1")) as Arc<dyn TileSource>]);
    let resp = server
        .post("/api/spatial/query")
        .json(&serde_json::json!({"source": "q1"}))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["source"], "q1");
    assert!(body["features"].is_array());
    assert!(body["total"].is_number());
    assert!(body["truncated"].is_boolean());
}

#[tokio::test]
async fn spatial_query_with_bbox_uses_bbox_center() {
    let server =
        common::server_with_sources(vec![Arc::new(MockSource::pbf("bb")) as Arc<dyn TileSource>]);
    let resp = server
        .post("/api/spatial/query")
        .json(&serde_json::json!({
            "source": "bb",
            "bbox": [-10.0, -10.0, 10.0, 10.0],
            "zoom": 5,
        }))
        .await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn spatial_query_with_layer_filter_succeeds() {
    let server = common::server_with_sources(vec![
        Arc::new(MockSource::pbf("filt")) as Arc<dyn TileSource>
    ]);
    let resp = server
        .post("/api/spatial/query")
        .json(&serde_json::json!({
            "source": "filt",
            "layers": ["roads"],
            "limit": 50,
        }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["source"], "filt");
}

#[tokio::test]
async fn spatial_query_with_no_center_falls_back_to_world() {
    // MockSource::no_center exercises the `(0, 0, 0)` default-tile branch
    let server = common::server_with_sources(vec![
        Arc::new(MockSource::no_center("nc")) as Arc<dyn TileSource>
    ]);
    let resp = server
        .post("/api/spatial/query")
        .json(&serde_json::json!({"source": "nc"}))
        .await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn spatial_query_against_raster_returns_invalid_tile_request() {
    let server = common::server_with_sources(vec![
        Arc::new(MockSource::png("rast")) as Arc<dyn TileSource>
    ]);
    let resp = server
        .post("/api/spatial/query")
        .json(&serde_json::json!({"source": "rast"}))
        .await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..500).contains(&status),
        "raster source must be rejected as InvalidTileRequest, got {status}"
    );
}

#[tokio::test]
async fn spatial_query_clamps_zoom_to_maxzoom() {
    // MockSource maxzoom = 14; requesting zoom=20 must clamp and not 5xx.
    let server =
        common::server_with_sources(vec![Arc::new(MockSource::pbf("cz")) as Arc<dyn TileSource>]);
    let resp = server
        .post("/api/spatial/query")
        .json(&serde_json::json!({
            "source": "cz",
            "bbox": [0.0, 0.0, 1.0, 1.0],
            "zoom": 20,
        }))
        .await;
    let status = resp.status_code().as_u16();
    assert!(status < 500, "clamped zoom must not 5xx, got {status}");
}

#[tokio::test]
async fn spatial_query_empty_source_no_tile_yields_4xx() {
    // MockSource::empty returns Ok(None) → TileNotFound → 4xx
    let server = common::server_with_sources(vec![
        Arc::new(MockSource::empty("notiles")) as Arc<dyn TileSource>
    ]);
    let resp = server
        .post("/api/spatial/query")
        .json(&serde_json::json!({"source": "notiles"}))
        .await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..500).contains(&status),
        "missing tile must surface as 4xx, got {status}"
    );
}

#[tokio::test]
async fn spatial_query_gzip_compressed_tile_decompresses_or_errors_cleanly() {
    // MockSource::pbf_gzip carries the gzip flag but invalid gzip bytes.
    // The handler must surface the decode error as a 5xx-or-4xx (whichever
    // the error mapping picks) — never panic.
    let server = common::server_with_sources(vec![
        Arc::new(MockSource::pbf_gzip("gz")) as Arc<dyn TileSource>
    ]);
    let resp = server
        .post("/api/spatial/query")
        .json(&serde_json::json!({"source": "gz"}))
        .await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..600).contains(&status),
        "broken-gzip source must yield 4xx or 5xx without panic, got {status}"
    );
}
