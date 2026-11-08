//! Integration tests for render routes.
//!
//! Every test asserts a non-200 response because `state.renderer = None` in the
//! shared empty test harness. This still drives route matching, path-parameter
//! extraction, the `parse()` helpers for raster tiles and static images, and
//! the `"Rendering not available"` branch in each handler — without requiring
//! MapLibre Native to be linked into the test binary.

mod common;

#[tokio::test]
async fn raster_tile_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/0/0/0.png").await;
    assert_ne!(
        resp.status_code().as_u16(),
        200,
        "raster tile without renderer must not return 200"
    );
}

#[tokio::test]
async fn raster_tile_with_size_512_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/512/0/0/0.png").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn static_image_center_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server
        .get("/styles/any-style/static/0,0,2/800x600.png")
        .await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

// ---------------------------------------------------------------------------
// server.disable_render route gating (#1004)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn disable_render_unregisters_raster_tile_route() {
    let mut config = tileserver_rs::config::Config::default();
    config.server.disable_render = true;
    let server = common::test_server_with_config(config);
    let resp = server.get("/styles/any-style/0/0/0.png").await;
    assert_eq!(
        resp.status_code().as_u16(),
        404,
        "raster tile route must be unregistered when disable_render = true"
    );
}

#[tokio::test]
async fn disable_render_unregisters_static_image_route() {
    let mut config = tileserver_rs::config::Config::default();
    config.server.disable_render = true;
    let server = common::test_server_with_config(config);
    let resp = server
        .get("/styles/any-style/static/0,0,2/400x300.png")
        .await;
    assert_eq!(resp.status_code().as_u16(), 404);
}

#[tokio::test]
async fn disable_render_keeps_styles_list_route() {
    let mut config = tileserver_rs::config::Config::default();
    config.server.disable_render = true;
    let server = common::test_server_with_config(config);
    let resp = server.get("/styles.json").await;
    assert_ne!(
        resp.status_code().as_u16(),
        404,
        "/styles.json must remain registered when only rendering is disabled"
    );
}

#[tokio::test]
async fn render_enabled_by_default_keeps_raster_route_registered() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/0/0/0.png").await;
    assert_ne!(
        resp.status_code().as_u16(),
        404,
        "raster route must be registered by default (non-404 even without a renderer)"
    );
}
