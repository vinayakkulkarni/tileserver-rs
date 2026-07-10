//! Integration tests for the static-file route (`routes/files.rs`).
//!
//! Covers `GET /files/{*filepath}` across the files-dir-configured and
//! files-dir-absent states, plus the directory-traversal guards.

mod common;

use axum::http::StatusCode;
use common::{empty_test_server, fixture_path, server_with_files_dir};

#[tokio::test]
async fn files_missing_dir_returns_404() {
    let server = empty_test_server();
    let res = server.get("/files/data.geojson").await;
    res.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn files_serves_existing_file_with_mime() {
    let server = server_with_files_dir(fixture_path("files"));
    let res = server.get("/files/data.geojson").await;
    res.assert_status(StatusCode::OK);
    assert!(
        res.header("content-type")
            .to_str()
            .unwrap()
            .contains("json"),
        "geojson should resolve to a json mime type"
    );
    assert_eq!(
        res.as_bytes().as_ref(),
        br#"{"type":"FeatureCollection","features":[]}"#
    );
}

#[tokio::test]
async fn files_serves_nested_file() {
    let server = server_with_files_dir(fixture_path("files"));
    let res = server.get("/files/subdir/nested.txt").await;
    res.assert_status(StatusCode::OK);
    assert_eq!(res.as_bytes().as_ref(), b"nested-file-content");
}

#[tokio::test]
async fn files_caches_static_content() {
    let server = server_with_files_dir(fixture_path("files"));
    let res = server.get("/files/data.geojson").await;
    res.assert_status(StatusCode::OK);
    assert_eq!(res.header("cache-control"), "public, max-age=3600");
}

#[tokio::test]
async fn files_traversal_returns_404() {
    let server = server_with_files_dir(fixture_path("files"));
    let res = server.get("/files/..%2f..%2fCargo.toml").await;
    res.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn files_missing_file_returns_404() {
    let server = server_with_files_dir(fixture_path("files"));
    let res = server.get("/files/does-not-exist.txt").await;
    res.assert_status(StatusCode::NOT_FOUND);
}
