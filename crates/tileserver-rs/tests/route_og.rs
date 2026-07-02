//! Integration tests for the `GET /og/{style}` route.
//!
//! The test harness has no native renderer, so a successfully-parsed request
//! reaches the renderer branch and returns `RenderError` (500). That 500 is the
//! correct observable here: it proves the route is registered, the style lookup
//! passed, and query validation accepted the input.

mod common;

use axum::http::StatusCode;
use common::{empty_test_server, shared_state_populated, test_server_with_config};
use tileserver_rs::config::Config;
use tileserver_rs::routes::api_router;

fn populated_server() -> axum_test::TestServer {
    axum_test::TestServer::new(api_router(shared_state_populated()))
}

#[tokio::test]
async fn og_unknown_style_returns_404() {
    let server = empty_test_server();
    let res = server.get("/og/no-such-style?center=0,0&zoom=2").await;
    res.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn og_no_renderer_returns_500() {
    let server = populated_server();
    let res = server.get("/og/protomaps-light?center=0,0&zoom=2").await;
    res.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn og_with_bounds_no_renderer_returns_500() {
    let server = populated_server();
    let res = server.get("/og/protomaps-light?bounds=-10,-10,10,10").await;
    res.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn og_oversized_width_no_renderer_returns_error_status() {
    let server = populated_server();
    let res = server
        .get("/og/protomaps-light?center=0,0&width=99999")
        .await;
    let status = res.status_code().as_u16();
    assert!(
        (400..600).contains(&status),
        "oversized width must be a 4xx/5xx, got {status}"
    );
}

#[tokio::test]
async fn og_with_disable_render_route_unregistered() {
    let mut config = Config::default();
    config.server.disable_render = true;
    let server = test_server_with_config(config);
    let res = server.get("/og/anything?center=0,0").await;
    res.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn og_route_registered_when_render_enabled() {
    let server = populated_server();
    let res = server.get("/og/protomaps-light?center=0,0").await;
    // 500 (renderer absent) proves the route is registered, not 404.
    res.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn og_query_param_format_passes_validation() {
    let server = populated_server();
    let res = server
        .get("/og/protomaps-light?center=0,0&format=jpeg")
        .await;
    res.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn og_missing_center_and_bounds_returns_400() {
    let server = populated_server();
    let res = server.get("/og/protomaps-light").await;
    res.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn og_invalid_center_returns_400() {
    let server = populated_server();
    let res = server.get("/og/protomaps-light?center=bad").await;
    res.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn og_invalid_bounds_returns_400() {
    let server = populated_server();
    let res = server.get("/og/protomaps-light?bounds=10,10,5,5").await;
    res.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn og_invalid_format_returns_400() {
    let server = populated_server();
    let res = server
        .get("/og/protomaps-light?center=0,0&format=bmp")
        .await;
    res.assert_status(StatusCode::BAD_REQUEST);
}
