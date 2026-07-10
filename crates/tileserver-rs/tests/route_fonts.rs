//! Integration tests for the font routes (`routes/fonts.rs`).
//!
//! Covers `GET /fonts.json` (family listing) and
//! `GET /fonts/{fontstack}/{range}` (glyph PBF serving) across both the
//! fonts-dir-configured and fonts-dir-absent states.
//!
//! Behavioural note: when a fonts directory *is* configured, a request for a
//! missing glyph range returns an **empty 200 PBF** (not a 404) — MapLibre
//! Native probes all 256 Unicode ranges and fails hard on 404s, so the handler
//! returns an empty protobuf for unpopulated ranges. A 404 only happens when no
//! fonts directory is configured at all.

mod common;

use axum::http::StatusCode;
use common::{empty_test_server, fixture_path, server_with_fonts_dir};

/// Directory name of the fixture font family, URL-encoded (spaces → `%20`)
/// for use in request paths. The on-disk family is `Test Sans Regular`.
const FONT_STACK: &str = "Test%20Sans%20Regular";

#[tokio::test]
async fn fonts_list_empty_when_no_fonts_dir() {
    let server = empty_test_server();
    let res = server.get("/fonts.json").await;
    res.assert_status(StatusCode::OK);
    let body: Vec<String> = res.json();
    assert!(body.is_empty(), "expected empty list, got {body:?}");
}

#[tokio::test]
async fn fonts_list_reports_family_with_glyphs() {
    let server = server_with_fonts_dir(fixture_path("fonts"));
    let res = server.get("/fonts.json").await;
    res.assert_status(StatusCode::OK);
    let body: Vec<String> = res.json();
    assert!(
        body.iter().any(|f| f == "Test Sans Regular"),
        "expected the fixture family in {body:?}"
    );
}

#[tokio::test]
async fn font_glyphs_missing_dir_returns_404() {
    let server = empty_test_server();
    let res = server.get(&format!("/fonts/{FONT_STACK}/0-255.pbf")).await;
    res.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn font_glyphs_valid_range_returns_bytes() {
    let server = server_with_fonts_dir(fixture_path("fonts"));
    let res = server.get(&format!("/fonts/{FONT_STACK}/0-255.pbf")).await;
    res.assert_status(StatusCode::OK);
    assert_eq!(
        res.header("content-type"),
        "application/x-protobuf",
        "font glyphs must be served as protobuf"
    );
    assert_eq!(res.as_bytes().as_ref(), b"GLYPHBYTES-0-255");
}

#[tokio::test]
async fn font_glyphs_missing_range_returns_empty_pbf() {
    let server = server_with_fonts_dir(fixture_path("fonts"));
    let res = server
        .get(&format!("/fonts/{FONT_STACK}/1024-1279.pbf"))
        .await;
    res.assert_status(StatusCode::OK);
    assert_eq!(res.header("content-type"), "application/x-protobuf");
    assert!(
        res.as_bytes().is_empty(),
        "unpopulated range must be an empty PBF"
    );
}

#[tokio::test]
async fn font_glyphs_non_pbf_range_returns_400() {
    let server = server_with_fonts_dir(fixture_path("fonts"));
    let res = server.get(&format!("/fonts/{FONT_STACK}/0-255.txt")).await;
    res.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn font_glyphs_traversal_in_range_returns_400() {
    let server = server_with_fonts_dir(fixture_path("fonts"));
    let res = server
        .get(&format!("/fonts/{FONT_STACK}/..%2f..%2fsecret.pbf"))
        .await;
    res.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn font_glyphs_multi_fontstack_falls_back_to_second() {
    let server = server_with_fonts_dir(fixture_path("fonts"));
    let res = server
        .get(&format!("/fonts/Missing%20Family,{FONT_STACK}/0-255.pbf"))
        .await;
    res.assert_status(StatusCode::OK);
    assert_eq!(res.as_bytes().as_ref(), b"GLYPHBYTES-0-255");
}

#[tokio::test]
async fn font_glyphs_traversal_in_fontstack_falls_through_to_empty_pbf() {
    let server = server_with_fonts_dir(fixture_path("fonts"));
    let res = server.get("/fonts/..%2f..%2fetc/0-255.pbf").await;
    res.assert_status(StatusCode::OK);
    assert!(res.as_bytes().is_empty());
}
