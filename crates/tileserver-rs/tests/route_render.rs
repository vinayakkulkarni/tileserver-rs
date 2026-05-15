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
async fn raster_tile_jpeg_format_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/0/0/0.jpg").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn raster_tile_jpeg_long_extension_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/0/0/0.jpeg").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn raster_tile_webp_format_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/0/0/0.webp").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn raster_tile_retina_2x_scale_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/0/0/0@2x.png").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn raster_tile_3x_scale_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/0/0/0@3x.png").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn raster_tile_invalid_format_extension_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/0/0/0.xyz").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn raster_tile_invalid_scale_out_of_range_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/0/0/0@99x.png").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn raster_tile_higher_zoom_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/10/512/512.png").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn raster_tile_with_size_512_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/512/0/0/0.png").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn raster_tile_with_size_256_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/256/0/0/0.webp").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn raster_tile_with_size_invalid_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/128/0/0/0.png").await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn raster_tile_with_size_retina_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server.get("/styles/any-style/512/0/0/0@2x.png").await;
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

#[tokio::test]
async fn static_image_center_with_bearing_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server
        .get("/styles/any-style/static/-122.4,37.8,12@45/800x600.png")
        .await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn static_image_bounds_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server
        .get("/styles/any-style/static/-180,-85,180,85/400x400.png")
        .await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn static_image_auto_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server
        .get("/styles/any-style/static/auto/800x600.png")
        .await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn static_image_retina_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server
        .get("/styles/any-style/static/0,0,2/800x600@2x.png")
        .await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn static_image_jpeg_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server
        .get("/styles/any-style/static/0,0,2/800x600.jpg")
        .await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn static_image_webp_no_renderer_returns_error() {
    let server = common::empty_test_server();
    let resp = server
        .get("/styles/any-style/static/0,0,2/800x600.webp")
        .await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn static_image_invalid_size_format_returns_error() {
    let server = common::empty_test_server();
    let resp = server
        .get("/styles/any-style/static/0,0,2/notxsize.png")
        .await;
    assert_ne!(resp.status_code().as_u16(), 200);
}

#[tokio::test]
async fn static_image_invalid_static_type_returns_error() {
    let server = common::empty_test_server();
    let resp = server
        .get("/styles/any-style/static/not,valid,coords/800x600.png")
        .await;
    assert_ne!(resp.status_code().as_u16(), 200);
}
