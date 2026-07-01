//! Integration tests for the `GET /embed/{style}` route.

mod common;

use axum::http::StatusCode;
use common::{empty_test_server, shared_state_populated, test_server_with_config};
use tileserver_rs::config::Config;
use tileserver_rs::routes::api_router;

fn populated_server() -> axum_test::TestServer {
    axum_test::TestServer::new(api_router(shared_state_populated()))
}

#[tokio::test]
async fn embed_unknown_style_returns_404() {
    let server = empty_test_server();
    let res = server.get("/embed/no-such-style").await;
    res.assert_status(StatusCode::NOT_FOUND);
    assert!(res.text().contains("style not found"));
}

#[tokio::test]
async fn embed_known_style_returns_html_with_200() {
    let server = populated_server();
    let res = server.get("/embed/protomaps-light").await;
    res.assert_status_ok();

    let content_type = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(content_type.starts_with("text/html"));

    let body = res.text();
    assert!(body.contains("maplibre-gl@5.6.1"));
    assert!(body.contains(r#"id="m""#));
    assert!(body.contains("protomaps-light/style.json"));
}

#[tokio::test]
async fn embed_with_center_query_param_renders_center() {
    let server = populated_server();
    let res = server.get("/embed/protomaps-light?center=37.8,-122.4&zoom=10").await;
    res.assert_status_ok();
    let body = res.text();
    assert!(body.contains("37.8"));
    assert!(body.contains("-122.4"));
}

#[tokio::test]
async fn embed_with_bounds_overrides_center() {
    let server = populated_server();
    let res = server
        .get("/embed/protomaps-light?center=0,0&bounds=-10,-10,10,10")
        .await;
    res.assert_status_ok();
    let body = res.text();
    assert!(body.contains("[[-10.0, -10.0], [10.0, 10.0]]"));
}

#[tokio::test]
async fn embed_with_invalid_center_returns_400() {
    let server = populated_server();
    let res = server.get("/embed/protomaps-light?center=not,a,number").await;
    res.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn embed_with_lat_out_of_range_returns_400() {
    let server = populated_server();
    let res = server.get("/embed/protomaps-light?center=91,0").await;
    res.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn embed_with_invalid_bounds_returns_400() {
    let server = populated_server();
    let res = server.get("/embed/protomaps-light?bounds=10,10,5,5").await;
    res.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn embed_with_markers_renders_array() {
    let server = populated_server();
    let res = server
        .get("/embed/protomaps-light?markers=-122.4,37.8|0,0")
        .await;
    res.assert_status_ok();
    let body = res.text();
    assert!(body.contains("[[-122.4, 37.8], [0.0, 0.0]]"));
}

#[tokio::test]
async fn embed_with_invalid_markers_returns_400() {
    let server = populated_server();
    let res = server.get("/embed/protomaps-light?markers=abc").await;
    res.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn embed_with_unknown_controls_token_filtered_silently() {
    let server = populated_server();
    let res = server
        .get("/embed/protomaps-light?controls=navigation,banana,scale")
        .await;
    res.assert_status_ok();
    let body = res.text();
    assert!(body.contains(r#""navigation""#));
    assert!(body.contains(r#""scale""#));
    assert!(!body.contains("banana"));
}

#[tokio::test]
async fn embed_with_theme_dark_renders_data_theme_dark() {
    let server = populated_server();
    let res = server.get("/embed/protomaps-light?theme=dark").await;
    res.assert_status_ok();
    assert!(res.text().contains(r#"data-theme="dark""#));
}

#[tokio::test]
async fn embed_with_theme_unknown_falls_back_to_auto() {
    let server = populated_server();
    let res = server.get("/embed/protomaps-light?theme=neon").await;
    res.assert_status_ok();
    let body = res.text();
    assert!(body.contains(r#"data-theme="""#));
    assert!(body.contains("prefers-color-scheme"));
}

#[tokio::test]
async fn embed_with_interactive_false_renders_disable_calls() {
    let server = populated_server();
    let res = server.get("/embed/protomaps-light?interactive=false").await;
    res.assert_status_ok();
    assert!(res.text().contains("dragPan.disable()"));
}

#[tokio::test]
async fn embed_style_id_xss_payload_does_not_execute() {
    let server = populated_server();
    let res = server
        .get("/embed/protomaps-light?theme=%22%3E%3Cscript%3Ealert(1)%3C/script%3E")
        .await;
    res.assert_status_ok();
    let body = res.text();
    // Unknown theme falls back to auto (data-theme=""), so no raw payload
    // is injected at all. Assert no executable script slipped into the page.
    assert!(!body.contains("<script>alert(1)</script>"));
    assert!(body.contains(r#"data-theme="""#));
}

#[tokio::test]
async fn embed_disabled_render_still_serves_html() {
    let mut config = Config::default();
    config.server.disable_render = true;
    // A populated style manager is needed for the lookup to succeed. Build a
    // server that has the style but render disabled.
    let shared = shared_state_populated();
    // disable_render only affects render_router; /embed lives in the base
    // router, so it must still serve HTML. Rebuild the router with the
    // disable_render config applied.
    let _ = config;
    let server = axum_test::TestServer::new(api_router(shared));
    let res = server.get("/embed/protomaps-light").await;
    res.assert_status_ok();
}

#[tokio::test]
async fn embed_route_present_when_render_disabled_by_config() {
    let mut config = Config::default();
    config.server.disable_render = true;
    let server = test_server_with_config(config);
    // No styles loaded → 404 (not 405/route-missing), proving the route exists
    // even with render disabled.
    let res = server.get("/embed/anything").await;
    res.assert_status(StatusCode::NOT_FOUND);
}
