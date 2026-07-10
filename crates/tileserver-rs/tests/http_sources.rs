//! Integration tests for the HTTP remote PMTiles source
//! (`src/sources/pmtiles/http.rs`).
//!
//! `HttpPmTilesSource` reads a PMTiles archive over HTTP using ranged GETs, so
//! the test spins up an in-process axum server that serves the on-disk
//! `protomaps-sample.pmtiles` fixture through `tower_http`'s `ServeFile`
//! (which honours `Range` requests). The source is then constructed against
//! that ephemeral URL and exercised end-to-end: header/metadata parsing, a real
//! `get_tile`, zoom-bounds and coordinate-bounds branches, and the failure path
//! when the URL does not resolve to a valid archive.

use std::net::SocketAddr;
use std::path::PathBuf;

use axum::Router;
use axum::routing::get_service;
use tokio::net::TcpListener;
use tower_http::services::ServeFile;

use tileserver_rs::TileSource;
use tileserver_rs::config::SourceConfig;
use tileserver_rs::sources::TileFormat;

/// Absolute path to the sample PMTiles archive shipped under `data/tiles/`.
fn sample_pmtiles() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/tiles/protomaps-sample.pmtiles")
}

/// Start an ephemeral HTTP server that serves the sample archive at
/// `/x.pmtiles` with `Range` support. Returns the bound base URL.
async fn serve_sample_pmtiles() -> String {
    let router =
        Router::new().route_service("/x.pmtiles", get_service(ServeFile::new(sample_pmtiles())));

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("resolve local addr");

    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve pmtiles");
    });

    format!("http://{addr}")
}

/// Build a minimal PMTiles `SourceConfig` pointing at the given URL. Uses TOML
/// deserialization so cfg-gated struct fields don't have to be spelled out.
fn http_source_config(id: &str, url: &str) -> SourceConfig {
    let toml = format!("id = \"{id}\"\ntype = \"pmtiles\"\npath = \"{url}\"\n");
    toml::from_str(&toml).expect("valid source config")
}

#[tokio::test]
async fn http_source_reads_metadata() {
    let base = serve_sample_pmtiles().await;
    let config = http_source_config("remote-pm", &format!("{base}/x.pmtiles"));

    let source = tileserver_rs::sources::pmtiles::http::HttpPmTilesSource::from_url(
        &config,
        reqwest::Client::new(),
    )
    .await
    .expect("open remote pmtiles");

    let meta = source.metadata();
    assert_eq!(meta.id, "remote-pm");
    assert_eq!(meta.format, TileFormat::Pbf);
    assert_eq!(meta.minzoom, 0);
    assert_eq!(meta.maxzoom, 15);
    assert!(meta.bounds.is_some(), "header should carry bounds");
}

#[tokio::test]
async fn http_source_serves_a_real_tile() {
    let base = serve_sample_pmtiles().await;
    let config = http_source_config("remote-pm", &format!("{base}/x.pmtiles"));
    let source = tileserver_rs::sources::pmtiles::http::HttpPmTilesSource::from_url(
        &config,
        reqwest::Client::new(),
    )
    .await
    .expect("open remote pmtiles");

    let tile = source.get_tile(0, 0, 0).await.expect("get_tile ok");
    assert!(tile.is_some(), "z0/0/0 should exist in the sample archive");
    let tile = tile.unwrap();
    assert_eq!(tile.format, TileFormat::Pbf);
    assert!(!tile.data.is_empty(), "tile payload must be non-empty");
}

#[tokio::test]
async fn http_source_out_of_zoom_returns_none() {
    let base = serve_sample_pmtiles().await;
    let config = http_source_config("remote-pm", &format!("{base}/x.pmtiles"));
    let source = tileserver_rs::sources::pmtiles::http::HttpPmTilesSource::from_url(
        &config,
        reqwest::Client::new(),
    )
    .await
    .expect("open remote pmtiles");

    // maxzoom is 15; z20 is above the archive's range.
    let tile = source.get_tile(20, 0, 0).await.expect("get_tile ok");
    assert!(tile.is_none(), "above-maxzoom request must be a miss");
}

#[tokio::test]
async fn http_source_invalid_coordinates_error() {
    let base = serve_sample_pmtiles().await;
    let config = http_source_config("remote-pm", &format!("{base}/x.pmtiles"));
    let source = tileserver_rs::sources::pmtiles::http::HttpPmTilesSource::from_url(
        &config,
        reqwest::Client::new(),
    )
    .await
    .expect("open remote pmtiles");

    // At z1 the valid x/y range is 0..=1; x=5 is out of bounds.
    let err = source.get_tile(1, 5, 0).await;
    assert!(err.is_err(), "out-of-range coordinate must error");
}

#[tokio::test]
async fn http_source_bad_url_fails_to_open() {
    let base = serve_sample_pmtiles().await;
    // Point at a path that 404s — no valid PMTiles header can be read.
    let config = http_source_config("remote-pm", &format!("{base}/missing.pmtiles"));

    let result = tileserver_rs::sources::pmtiles::http::HttpPmTilesSource::from_url(
        &config,
        reqwest::Client::new(),
    )
    .await;

    assert!(result.is_err(), "missing archive must fail to open");
}
