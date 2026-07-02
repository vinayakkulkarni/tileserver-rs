//! Integration tests for the multi-source composite endpoint (#601).
//!
//! Covers `GET /data/{a+b}.json` (composite TileJSON) and
//! `GET /data/{a+b}/{z}/{x}/{y}.pbf` (merged MVT tile).

mod common;

use std::sync::Arc;

use common::{MockSource, server_with_sources, server_with_sources_and_config};
use serde_json::Value;
use tileserver_rs::TileSource;
use tileserver_rs::composite::{decode_mvt_layers, encode_test_tile};
use tileserver_rs::config::{CompositeConfig, Config};

fn config_with_composite(id: &str, sources: &[&str]) -> Config {
    Config {
        composites: vec![CompositeConfig::new(
            id,
            sources.iter().map(|s| s.to_string()).collect(),
        )],
        ..Default::default()
    }
}

fn source_with_layer(id: &str, layer: &str, features: usize) -> Arc<dyn TileSource> {
    Arc::new(
        MockSource::pbf(id)
            .with_vector_layers(serde_json::json!([{ "id": layer }]))
            .with_tile_bytes(encode_test_tile(layer, features)),
    ) as Arc<dyn TileSource>
}

// ---------------------------------------------------------------------------
// Composite TileJSON
// ---------------------------------------------------------------------------

#[tokio::test]
async fn composite_tilejson_known_returns_200() {
    let server = server_with_sources(vec![
        source_with_layer("a", "roads", 1),
        source_with_layer("b", "water", 1),
    ]);
    server.get("/data/a+b.json").await.assert_status_ok();
}

#[tokio::test]
async fn composite_tilejson_id_uses_composite_id() {
    let server = server_with_sources(vec![
        source_with_layer("a", "roads", 1),
        source_with_layer("b", "water", 1),
    ]);
    let body: Value = server.get("/data/a+b.json").await.json();
    assert_eq!(body["id"], "a+b");
}

#[tokio::test]
async fn composite_tilejson_tiles_url_path_includes_composite_id() {
    let server = server_with_sources(vec![
        source_with_layer("a", "roads", 1),
        source_with_layer("b", "water", 1),
    ]);
    let body: Value = server.get("/data/a+b.json").await.json();
    let tiles = body["tiles"].as_array().unwrap();
    assert!(tiles[0].as_str().unwrap().contains("/data/a+b/"));
}

#[tokio::test]
async fn composite_tilejson_vector_layers_merged() {
    let server = server_with_sources(vec![
        source_with_layer("a", "roads", 1),
        source_with_layer("b", "water", 1),
    ]);
    let body: Value = server.get("/data/a+b.json").await.json();
    let vl = body["vector_layers"].as_array().unwrap();
    assert_eq!(vl.len(), 2);
}

#[tokio::test]
async fn composite_tilejson_unknown_source_returns_404() {
    let server = server_with_sources(vec![source_with_layer("a", "roads", 1)]);
    server
        .get("/data/a+ghost.json")
        .await
        .assert_status_not_found();
}

#[tokio::test]
async fn composite_tilejson_single_source_returns_200() {
    let server = server_with_sources(vec![source_with_layer("a", "roads", 1)]);
    // "a+" trims to single member "a"
    server.get("/data/a+.json").await.assert_status_ok();
}

#[tokio::test]
async fn composite_tilejson_includes_key_query_when_provided() {
    let server = server_with_sources(vec![
        source_with_layer("a", "roads", 1),
        source_with_layer("b", "water", 1),
    ]);
    let body: Value = server.get("/data/a+b.json?key=secret").await.json();
    let tiles = body["tiles"].as_array().unwrap();
    assert!(tiles[0].as_str().unwrap().contains("?key=secret"));
}

// ---------------------------------------------------------------------------
// Composite tile
// ---------------------------------------------------------------------------

#[tokio::test]
async fn composite_tile_known_returns_200_pbf() {
    let server = server_with_sources(vec![
        source_with_layer("a", "roads", 1),
        source_with_layer("b", "water", 1),
    ]);
    server.get("/data/a+b/0/0/0.pbf").await.assert_status_ok();
}

#[tokio::test]
async fn composite_tile_response_content_type_is_protobuf() {
    let server = server_with_sources(vec![
        source_with_layer("a", "roads", 1),
        source_with_layer("b", "water", 1),
    ]);
    let res = server.get("/data/a+b/0/0/0.pbf").await;
    let ct = res.header("content-type");
    assert_eq!(ct, "application/x-protobuf");
}

#[tokio::test]
async fn composite_tile_two_members_distinct_layers_preserved() {
    let server = server_with_sources(vec![
        source_with_layer("a", "roads", 2),
        source_with_layer("b", "water", 3),
    ]);
    let res = server.get("/data/a+b/0/0/0.pbf").await;
    let layers = decode_mvt_layers(&res.into_bytes()).unwrap();
    let names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
    assert!(names.contains(&"roads"));
    assert!(names.contains(&"water"));
}

#[tokio::test]
async fn composite_tile_colliding_layer_appends_features() {
    let server = server_with_sources(vec![
        source_with_layer("a", "roads", 2),
        source_with_layer("b", "roads", 3),
    ]);
    let res = server.get("/data/a+b/0/0/0.pbf").await;
    let layers = decode_mvt_layers(&res.into_bytes()).unwrap();
    let roads = layers.iter().find(|l| l.name == "roads").unwrap();
    assert_eq!(roads.features.len(), 5);
}

#[tokio::test]
async fn composite_tile_single_member_returns_underlying_layer() {
    let server = server_with_sources(vec![source_with_layer("a", "roads", 4)]);
    let res = server.get("/data/a+/0/0/0.pbf").await;
    let layers = decode_mvt_layers(&res.into_bytes()).unwrap();
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].features.len(), 4);
}

#[tokio::test]
async fn composite_tile_member_miss_returns_other_members() {
    let server = server_with_sources(vec![
        source_with_layer("a", "roads", 2),
        Arc::new(MockSource::empty("b")) as Arc<dyn TileSource>,
    ]);
    let res = server.get("/data/a+b/0/0/0.pbf").await;
    res.assert_status_ok();
    let layers = decode_mvt_layers(&res.into_bytes()).unwrap();
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].name, "roads");
}

#[tokio::test]
async fn composite_tile_all_members_miss_returns_empty_tile_200() {
    let server = server_with_sources(vec![
        Arc::new(MockSource::empty("a")) as Arc<dyn TileSource>,
        Arc::new(MockSource::empty("b")) as Arc<dyn TileSource>,
    ]);
    let res = server.get("/data/a+b/0/0/0.pbf").await;
    res.assert_status_ok();
    let layers = decode_mvt_layers(&res.into_bytes()).unwrap();
    assert!(layers.is_empty());
}

#[tokio::test]
async fn composite_tile_gzipped_member_is_decompressed_before_merge() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let raw = encode_test_tile("roads", 2);
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&raw).unwrap();
    let gz = enc.finish().unwrap();

    let a = Arc::new(
        MockSource::pbf("a")
            .with_vector_layers(serde_json::json!([{ "id": "roads" }]))
            .with_gzip_tile_bytes(gz),
    ) as Arc<dyn TileSource>;
    let server = server_with_sources(vec![a, source_with_layer("b", "water", 1)]);
    let res = server.get("/data/a+b/0/0/0.pbf").await;
    res.assert_status_ok();
    let layers = decode_mvt_layers(&res.into_bytes()).unwrap();
    let roads = layers.iter().find(|l| l.name == "roads").unwrap();
    assert_eq!(roads.features.len(), 2);
}

#[tokio::test]
async fn composite_tile_unknown_source_id_returns_404() {
    let server = server_with_sources(vec![source_with_layer("a", "roads", 1)]);
    server
        .get("/data/a+ghost/0/0/0.pbf")
        .await
        .assert_status_not_found();
}

#[tokio::test]
async fn composite_tile_format_alias_mvt_works() {
    let server = server_with_sources(vec![
        source_with_layer("a", "roads", 1),
        source_with_layer("b", "water", 1),
    ]);
    server.get("/data/a+b/0/0/0.mvt").await.assert_status_ok();
}

#[tokio::test]
async fn composite_tile_raster_member_returns_400() {
    let server = server_with_sources(vec![
        source_with_layer("a", "roads", 1),
        Arc::new(MockSource::png("b")) as Arc<dyn TileSource>,
    ]);
    let res = server.get("/data/a+b/0/0/0.pbf").await;
    res.assert_status_bad_request();
}

#[tokio::test]
async fn composite_tile_invalid_y_returns_400() {
    let server = server_with_sources(vec![source_with_layer("a", "roads", 1)]);
    server
        .get("/data/a+b/0/0/notadot")
        .await
        .assert_status_bad_request();
}

// ---------------------------------------------------------------------------
// Named composites via [[composites]]
// ---------------------------------------------------------------------------

#[tokio::test]
async fn composite_tilejson_named_returns_200() {
    let server = server_with_sources_and_config(
        vec![
            source_with_layer("a", "roads", 1),
            source_with_layer("b", "water", 1),
        ],
        config_with_composite("world", &["a", "b"]),
    );
    let res = server.get("/data/world.json").await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["id"], "world");
    assert!(body["tiles"][0].as_str().unwrap().contains("/data/world/"));
}

#[tokio::test]
async fn composite_tilejson_named_merges_member_vector_layers() {
    let server = server_with_sources_and_config(
        vec![
            source_with_layer("a", "roads", 1),
            source_with_layer("b", "water", 1),
        ],
        config_with_composite("world", &["a", "b"]),
    );
    let body: Value = server.get("/data/world.json").await.json();
    assert_eq!(body["vector_layers"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn composite_tilejson_named_unknown_member_returns_404() {
    let server = server_with_sources_and_config(
        vec![source_with_layer("a", "roads", 1)],
        config_with_composite("world", &["a", "ghost"]),
    );
    server
        .get("/data/world.json")
        .await
        .assert_status_not_found();
}

#[tokio::test]
async fn composite_tile_named_returns_200_merged() {
    let server = server_with_sources_and_config(
        vec![
            source_with_layer("a", "roads", 2),
            source_with_layer("b", "water", 3),
        ],
        config_with_composite("world", &["a", "b"]),
    );
    let res = server.get("/data/world/0/0/0.pbf").await;
    res.assert_status_ok();
    let layers = decode_mvt_layers(&res.into_bytes()).unwrap();
    let names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
    assert!(names.contains(&"roads"));
    assert!(names.contains(&"water"));
}
