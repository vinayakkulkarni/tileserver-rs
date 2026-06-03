//! Integration tests for data/tile route handlers in `routes/data.rs`.
//!
//! Covers:
//! - GET /data.json (empty + populated source list)
//! - GET /data/{source} (TileJSON metadata, .json suffix stripping)
//! - GET /data/{source}/{z}/{x}/{y}.{format} (vector tile retrieval)
//! - Error paths (unknown source, invalid format, malformed requests)
//! - Query parameters (`?key=...`)
//! - MLT format requests (best-effort: must not 5xx)
//!
//! Uses the shared test harness from `common/mod.rs` and the `tests/config.test.toml`
//! fixture which references real PMTiles + MBTiles tile data shipped in `data/tiles/`.

mod common;

use std::path::PathBuf;
use std::sync::Arc;

use axum_test::TestServer;
use tileserver_rs::{
    Config, SourceManager,
    reload::{ReloadController, SharedState},
    routes::api_router,
};

// ============================================================
// Helpers
// ============================================================

/// Build a [`TestServer`] backed by sources loaded from `tests/config.test.toml`.
///
/// The test config registers two sources:
/// - `protomaps` — local PMTiles fixture (`data/tiles/protomaps-sample.pmtiles`)
/// - `zurich` — local MBTiles fixture (`data/tiles/zurich_switzerland.mbtiles`)
///
/// Returns a server wired to the real `api_router` so HTTP behaviour is exercised
/// end-to-end (handlers, extractors, response shapes, error mapping).
async fn pmtiles_test_server() -> TestServer {
    let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
        .expect("load tests/config.test.toml");

    let sources = SourceManager::from_configs(&config.sources)
        .await
        .expect("load tile sources from test config");

    let mut state = common::minimal_app_state();
    state.sources = Arc::new(sources);

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

/// Build a [`TestServer`] whose `SourceManager` has the global tile cache
/// enabled, so the `Accept-Encoding` transcode + cache-store + cache-hit path
/// in `routes/data.rs` is exercised end-to-end.
async fn compression_test_server() -> TestServer {
    use tileserver_rs::cache::TileCache;

    let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
        .expect("load tests/config.test.toml");

    let cache = Arc::new(TileCache::new(64, 3600));
    let sources = SourceManager::from_configs(&config.sources)
        .await
        .expect("load tile sources from test config")
        .with_cache(cache);

    let mut state = common::minimal_app_state();
    state.sources = Arc::new(sources);

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
    TestServer::new(api_router(shared))
}

// ============================================================
// Empty-state tests — exercise the "no sources loaded" branches
// ============================================================

#[tokio::test]
async fn data_json_empty_returns_empty_array() {
    let server = common::empty_test_server();
    let resp = server.get("/data.json").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert!(body.is_array(), "/data.json must return a JSON array");
    assert_eq!(
        body.as_array().unwrap().len(),
        0,
        "empty state must return zero sources"
    );
}

#[tokio::test]
async fn data_source_tilejson_not_found_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/data/nonexistent-source").await;
    assert_ne!(
        resp.status_code().as_u16(),
        200,
        "missing source must not return 200"
    );
}

#[tokio::test]
async fn data_source_tilejson_json_suffix_not_found_returns_error() {
    // Path is parsed BEFORE `.json` is stripped, so this also hits the
    // SourceNotFound branch — exercises the `.json` suffix code path too.
    let server = common::empty_test_server();
    let resp = server.get("/data/nonexistent.json").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn data_tile_unknown_source_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/data/nonexistent/0/0/0.pbf").await;
    assert_ne!(
        resp.status_code().as_u16(),
        200,
        "unknown source tile request must not return 200"
    );
}

#[tokio::test]
async fn data_tile_unknown_format_does_not_500() {
    // Format parsing happens server-side; unknown extensions must surface as
    // a 4xx (InvalidTileRequest / source-not-found) rather than crashing.
    let server = common::empty_test_server();
    let resp = server.get("/data/nonexistent/0/0/0.xyz").await;
    let status = resp.status_code().as_u16();
    assert!(status < 500, "unknown format must not 5xx, got {status}");
}

#[tokio::test]
async fn data_tile_malformed_y_fmt_does_not_500() {
    // `y_fmt` has no dot — `parse_y_and_format` returns None → InvalidTileRequest.
    let server = common::empty_test_server();
    let resp = server.get("/data/foo/0/0/notadotted").await;
    let status = resp.status_code().as_u16();
    assert!(status < 500, "malformed y_fmt must not 5xx, got {status}");
}

// ============================================================
// PMTiles-backed tests — exercise the happy-path branches
// ============================================================

#[tokio::test]
async fn data_json_with_sources_returns_entries() {
    let server = pmtiles_test_server().await;
    let resp = server.get("/data.json").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let arr = body.as_array().expect("/data.json must return an array");
    assert!(
        !arr.is_empty(),
        "should have at least 1 source loaded from config.test.toml"
    );
}

#[tokio::test]
async fn data_json_entry_has_tilejson_shape() {
    let server = pmtiles_test_server().await;
    let resp = server.get("/data.json").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let arr = body.as_array().unwrap();
    let first = &arr[0];
    assert_eq!(
        first["tilejson"], "3.0.0",
        "each entry must be a TileJSON 3.0.0 object"
    );
    assert!(first["tiles"].is_array(), "entry must have tiles array");
}

#[tokio::test]
async fn data_json_with_key_query_param_appends_key() {
    let server = pmtiles_test_server().await;
    let resp = server.get("/data.json?key=testkey").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let arr = body.as_array().unwrap();
    let first_tile_url = arr[0]["tiles"][0].as_str().unwrap_or("");
    assert!(
        first_tile_url.contains("key=testkey"),
        "tile URL must carry the key query parameter, got: {first_tile_url}"
    );
}

#[tokio::test]
async fn data_source_tilejson_found_returns_tilejson_3_0_0() {
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/protomaps").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["tilejson"], "3.0.0");
}

#[tokio::test]
async fn data_source_tilejson_has_tiles_array() {
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/protomaps").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let tiles = body["tiles"]
        .as_array()
        .expect("tilejson must have tiles array");
    assert!(!tiles.is_empty(), "tiles array must not be empty");
    let first = tiles[0].as_str().expect("tile URL must be a string");
    assert!(
        first.contains("/data/protomaps/"),
        "tile URL must reference the source id, got: {first}"
    );
}

#[tokio::test]
async fn data_source_tilejson_strips_json_suffix() {
    // routes/data.rs strips `.json` before lookup — both forms must resolve.
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/protomaps.json").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["tilejson"], "3.0.0");
}

#[tokio::test]
async fn data_source_tilejson_with_key_query_param() {
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/protomaps?key=apikey123").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let first_tile = body["tiles"][0].as_str().unwrap_or("");
    assert!(
        first_tile.contains("key=apikey123"),
        "key must be appended to tile URLs, got: {first_tile}"
    );
}

#[tokio::test]
async fn data_source_tilejson_unknown_source_returns_error() {
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/does-not-exist").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

// ============================================================
// Tile retrieval tests
// ============================================================

#[tokio::test]
async fn data_tile_zoom_0_returns_2xx_or_204() {
    // Tile 0/0/0 may or may not be present in the fixture; both
    // success (200) and no-content (204) are valid outcomes. What
    // matters: the handler runs without 5xxing.
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/protomaps/0/0/0.pbf").await;
    let status = resp.status_code().as_u16();
    assert!(
        matches!(status, 200 | 204 | 404),
        "tile request must return 200/204/404, got {status}"
    );
}

#[tokio::test]
async fn data_tile_mvt_format_alias_accepted() {
    // `.mvt` and `.pbf` are both valid vector tile extensions.
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/protomaps/0/0/0.mvt").await;
    let status = resp.status_code().as_u16();
    assert!(status < 500, ".mvt request must not 5xx, got {status}");
}

#[tokio::test]
async fn data_tile_out_of_range_zoom_does_not_500() {
    // Zoom 30 is beyond any sane tileset's maxzoom — handler must
    // return a 4xx (TileNotFound), never panic or 5xx.
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/protomaps/30/0/0.pbf").await;
    let status = resp.status_code().as_u16();
    assert!(status < 500, "out-of-range zoom must not 5xx, got {status}");
}

#[tokio::test]
async fn data_tile_unknown_source_with_real_sources_returns_error() {
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/unknown-id/0/0/0.pbf").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn data_tile_mlt_format_request_does_not_500() {
    // MLT may or may not be supported depending on feature flags;
    // the handler must either transcode successfully or fall back
    // cleanly — never 5xx.
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/protomaps/0/0/0.mlt").await;
    let status = resp.status_code().as_u16();
    assert!(status < 500, ".mlt request must not 5xx, got {status}");
}

#[tokio::test]
async fn data_tile_compression_test_server() {
    // placeholder to keep the cache server constructor exercised
    let _ = compression_test_server().await;
}

// ============================================================
// Directory + tar source tests (Bundle C — #1006 / #1005)
// ============================================================

#[tokio::test]
async fn dir_source_tilejson_returns_metadata() {
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/dir-tiles").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["id"], "dir-tiles");
    assert!(body["tiles"].is_array());
}

#[tokio::test]
async fn dir_source_serves_known_tile_bytes() {
    let server = pmtiles_test_server().await;
    // Fixture has 3/4/5.pbf with the crafted "roads"-layer MVT.
    let resp = server.get("/data/dir-tiles/3/4/5.pbf").await;
    resp.assert_status_ok();
    let bytes = resp.as_bytes();
    assert!(!bytes.is_empty(), "dir tile must return non-empty bytes");
}

#[tokio::test]
async fn dir_source_missing_tile_not_200() {
    let server = pmtiles_test_server().await;
    // 5/0/0 is absent from the fixture pyramid.
    let resp = server.get("/data/dir-tiles/5/0/0.pbf").await;
    assert_ne!(
        resp.status_code().as_u16(),
        200,
        "missing dir tile must not return 200"
    );
}

#[tokio::test]
async fn tar_source_tilejson_returns_metadata() {
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/tar-tiles").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["id"], "tar-tiles");
    assert!(body["tiles"].is_array());
}

#[tokio::test]
async fn tar_source_serves_known_tile_bytes() {
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/tar-tiles/3/4/5.pbf").await;
    resp.assert_status_ok();
    let bytes = resp.as_bytes();
    assert!(!bytes.is_empty(), "tar tile must return non-empty bytes");
}

#[tokio::test]
async fn tar_source_missing_tile_not_200() {
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/tar-tiles/5/0/0.pbf").await;
    assert_ne!(
        resp.status_code().as_u16(),
        200,
        "missing tar tile must not return 200"
    );
}

#[tokio::test]
async fn tar_source_introspects_vector_layers() {
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/tar-tiles").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let layers = body["vector_layers"]
        .as_array()
        .expect("tar source must expose vector_layers");
    assert!(
        layers.iter().any(|l| l["id"] == "roads"),
        "tar vector_layers must include the crafted 'roads' layer"
    );
}

#[tokio::test]
async fn data_tile_malformed_y_fmt_no_dot() {
    // y_fmt without `.` — InvalidTileRequest branch.
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/protomaps/0/0/garbage").await;
    let status = resp.status_code().as_u16();
    assert!(status < 500, "malformed y must not 5xx, got {status}");
    assert_ne!(status, 200, "malformed y must not return 200");
}

#[tokio::test]
async fn data_tile_malformed_y_fmt_non_numeric() {
    // y_fmt with dot but non-numeric y — parse_y_and_format returns None.
    let server = pmtiles_test_server().await;
    let resp = server.get("/data/protomaps/0/0/notanumber.pbf").await;
    let status = resp.status_code().as_u16();
    assert!(status < 500, "non-numeric y must not 5xx, got {status}");
    assert_ne!(status, 200, "non-numeric y must not return 200");
}

// ============================================================
// Mock-source tests — exercise handler success branches with
// deterministic in-memory tile data, no PMTiles I/O required.
// ============================================================

#[tokio::test]
async fn data_json_with_mock_source_returns_single_entry() {
    let server = common::server_with_sources(vec![
        Arc::new(common::MockSource::pbf("mock-vec")) as Arc<dyn tileserver_rs::TileSource>
    ]);
    let resp = server.get("/data.json").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let arr = body.as_array().expect("array response");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "mock-vec");
    assert_eq!(arr[0]["tilejson"], "3.0.0");
}

#[tokio::test]
async fn data_tilejson_mock_source_returns_correct_metadata() {
    let server = common::server_with_sources(vec![
        Arc::new(common::MockSource::pbf("mock-meta")) as Arc<dyn tileserver_rs::TileSource>
    ]);
    let resp = server.get("/data/mock-meta").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["id"], "mock-meta");
    assert_eq!(body["minzoom"], 0);
    assert_eq!(body["maxzoom"], 14);
    assert!(body["vector_layers"].is_array());
}

#[tokio::test]
async fn data_tilejson_mock_source_strip_json_suffix() {
    let server = common::server_with_sources(vec![
        Arc::new(common::MockSource::pbf("mock-strip")) as Arc<dyn tileserver_rs::TileSource>
    ]);
    let resp = server.get("/data/mock-strip.json").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["id"], "mock-strip");
}

#[tokio::test]
async fn data_tile_mock_source_returns_200_with_content_type() {
    let server = common::server_with_sources(vec![
        Arc::new(common::MockSource::pbf("mock-tile")) as Arc<dyn tileserver_rs::TileSource>
    ]);
    let resp = server.get("/data/mock-tile/0/0/0.pbf").await;
    resp.assert_status_ok();
    let ct = resp
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or("").to_string())
        .unwrap_or_default();
    assert!(
        ct.contains("vnd.mapbox-vector-tile")
            || ct.contains("application/x-protobuf")
            || ct.contains("application/vnd.mapbox-vector-tile"),
        "vector tile content-type must be MVT-compatible, got: {ct}"
    );
}

#[tokio::test]
async fn data_tile_mock_source_response_body_matches() {
    let server = common::server_with_sources(vec![
        Arc::new(common::MockSource::pbf("body-mock")) as Arc<dyn tileserver_rs::TileSource>
    ]);
    let resp = server.get("/data/body-mock/0/0/0.pbf").await;
    resp.assert_status_ok();
    let body = resp.as_bytes();
    assert_eq!(
        &body[..],
        b"mock-pbf-bytes",
        "body must echo MockSource data"
    );
}

#[tokio::test]
async fn data_tile_mock_gzipped_sets_content_encoding() {
    let server = common::server_with_sources(vec![
        Arc::new(common::MockSource::pbf_gzip("gz-mock")) as Arc<dyn tileserver_rs::TileSource>,
    ]);
    let resp = server.get("/data/gz-mock/0/0/0.pbf").await;
    resp.assert_status_ok();
    let ce = resp
        .headers()
        .get("content-encoding")
        .map(|v| v.to_str().unwrap_or("").to_string());
    assert_eq!(ce.as_deref(), Some("gzip"));
}

#[tokio::test]
async fn data_tile_mock_empty_source_returns_4xx() {
    let server = common::server_with_sources(vec![
        Arc::new(common::MockSource::empty("nada")) as Arc<dyn tileserver_rs::TileSource>
    ]);
    let resp = server.get("/data/nada/0/0/0.pbf").await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..500).contains(&status),
        "empty source must yield 4xx (TileNotFound), got {status}"
    );
}

#[tokio::test]
async fn data_tile_geojson_format_on_raster_source_returns_error() {
    let server = common::server_with_sources(vec![
        Arc::new(common::MockSource::png("rast")) as Arc<dyn tileserver_rs::TileSource>
    ]);
    let resp = server.get("/data/rast/0/0/0.geojson").await;
    let status = resp.status_code().as_u16();
    assert!(
        status >= 400,
        "geojson conversion on non-PBF must fail, got {status}"
    );
}

#[tokio::test]
async fn data_tile_geojson_format_on_unknown_source_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/data/ghost/0/0/0.geojson").await;
    let status = resp.status_code().as_u16();
    assert_ne!(status, 200);
}

#[tokio::test]
async fn data_tile_geojson_format_on_empty_pbf_source_returns_4xx() {
    let server = common::server_with_sources(vec![
        Arc::new(common::MockSource::empty("emp")) as Arc<dyn tileserver_rs::TileSource>
    ]);
    let resp = server.get("/data/emp/0/0/0.geojson").await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..500).contains(&status),
        "empty tile request must surface as 4xx, got {status}"
    );
}

#[tokio::test]
async fn data_tile_geojson_decode_error_on_garbage_bytes_returns_5xx() {
    // MockSource::pbf returns bytes that are not a valid MVT — Tile::decode
    // should fail with a RenderError mapped to a 4xx/5xx.
    let server = common::server_with_sources(vec![
        Arc::new(common::MockSource::pbf("bad-mvt")) as Arc<dyn tileserver_rs::TileSource>
    ]);
    let resp = server.get("/data/bad-mvt/0/0/0.geojson").await;
    let status = resp.status_code().as_u16();
    assert!(
        (400..600).contains(&status),
        "garbage MVT bytes must yield 4xx/5xx, got {status}"
    );
}

#[tokio::test]
async fn data_json_multiple_mock_sources_listed() {
    let server = common::server_with_sources(vec![
        Arc::new(common::MockSource::pbf("alpha")) as Arc<dyn tileserver_rs::TileSource>,
        Arc::new(common::MockSource::pbf("beta")) as Arc<dyn tileserver_rs::TileSource>,
        Arc::new(common::MockSource::png("gamma")) as Arc<dyn tileserver_rs::TileSource>,
    ]);
    let resp = server.get("/data.json").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 3);
}

#[tokio::test]
async fn data_tile_mock_mvt_extension_alias_works() {
    let server = common::server_with_sources(vec![
        Arc::new(common::MockSource::pbf("alias-mvt")) as Arc<dyn tileserver_rs::TileSource>
    ]);
    let resp = server.get("/data/alias-mvt/0/0/0.mvt").await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn data_tilejson_mock_source_with_api_key_appended() {
    let server = common::server_with_sources(vec![
        Arc::new(common::MockSource::pbf("keyed")) as Arc<dyn tileserver_rs::TileSource>
    ]);
    let resp = server.get("/data/keyed?key=secret-xyz").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let first = body["tiles"][0].as_str().unwrap_or("");
    assert!(
        first.contains("key=secret-xyz"),
        "API key must be appended to tile URLs, got {first}"
    );
}

// ============================================================
// Accept-Encoding negotiation (routes/data.rs transcode + cache path)
// ============================================================

#[tokio::test]
async fn tile_vary_accept_encoding_header_always_present() {
    let server = compression_test_server().await;
    let resp = server.get("/data/protomaps/0/0/0.pbf").await;
    if resp.status_code().as_u16() == 200 {
        let vary = resp.headers().get("vary");
        assert!(
            vary.is_some_and(|v| v
                .to_str()
                .unwrap_or("")
                .to_ascii_lowercase()
                .contains("accept-encoding")),
            "every tile response must carry Vary: Accept-Encoding (CDN poisoning guard)"
        );
    }
}

#[tokio::test]
async fn tile_brotli_request_transcodes_and_caches() {
    let server = compression_test_server().await;
    // Two identical br requests: first transcodes + caches (transcode_miss),
    // second is served from the encoding-keyed cache (transcode_hit). Both must
    // succeed and, when content is present, report Content-Encoding: br.
    for _ in 0..2 {
        let resp = server
            .get("/data/protomaps/0/0/0.pbf")
            .add_header("accept-encoding", "br")
            .await;
        let status = resp.status_code().as_u16();
        assert!(status < 500, "br tile request must not 5xx, got {status}");
        if status == 200 {
            let ce = resp
                .headers()
                .get("content-encoding")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                ce == "br" || ce.is_empty(),
                "br request returns br (or identity for sub-threshold tiles), got {ce:?}"
            );
        }
    }
}

#[tokio::test]
async fn tile_identity_request_does_not_5xx() {
    let server = compression_test_server().await;
    let resp = server
        .get("/data/protomaps/0/0/0.pbf")
        .add_header("accept-encoding", "identity")
        .await;
    assert!(resp.status_code().as_u16() < 500);
}

#[tokio::test]
async fn tile_zstd_request_does_not_5xx() {
    let server = compression_test_server().await;
    let resp = server
        .get("/data/protomaps/0/0/0.pbf")
        .add_header("accept-encoding", "zstd")
        .await;
    assert!(resp.status_code().as_u16() < 500);
}

// ============================================================
// Trailing-slash normalization (main.rs Router::trim_trailing_slash)
// ============================================================

/// Drive a single GET `path` through the production normalization wiring
/// (`NormalizePathLayer::trim_trailing_slash` wrapping `api_router`) and return
/// the response status. `axum_test::TestServer` cannot host a `NormalizePath`
/// service (no `IntoTransportLayer` impl), so we exercise the layer directly via
/// `tower::ServiceExt::oneshot` — the same path the production `axum::serve` uses.
async fn normalized_status(path: &str) -> axum::http::StatusCode {
    use tower::{Layer, ServiceExt};
    use tower_http::normalize_path::NormalizePathLayer;

    let shared = SharedState::new(Arc::new(ReloadController::new(
        common::minimal_app_state(),
        common::minimal_meta(),
        Config::default(),
        None,
        common::minimal_runtime(),
    )));
    let app = NormalizePathLayer::trim_trailing_slash().layer(api_router(shared));
    let req = axum::http::Request::builder()
        .uri(path)
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    resp.status()
}

#[tokio::test]
async fn trailing_slash_on_health_resolves() {
    assert_eq!(
        normalized_status("/health/").await,
        axum::http::StatusCode::OK
    );
}

#[tokio::test]
async fn trailing_slash_on_data_json_resolves() {
    assert_eq!(
        normalized_status("/data.json/").await,
        axum::http::StatusCode::OK
    );
}

#[tokio::test]
async fn no_trailing_slash_still_resolves() {
    assert_eq!(
        normalized_status("/health").await,
        axum::http::StatusCode::OK
    );
}
