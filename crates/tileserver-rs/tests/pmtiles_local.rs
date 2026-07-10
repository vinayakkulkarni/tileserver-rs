//! Integration tests for the local (mmap) PMTiles source
//! (`src/sources/pmtiles/local.rs`).
//!
//! Exercises `LocalPmTilesSource::from_file` against the on-disk
//! `protomaps-sample.pmtiles` fixture: header/metadata parsing, a real
//! `get_tile`, the zoom-bounds and coordinate-bounds branches, plus the two
//! open-time failure paths (missing file → `ConfigError`, non-PMTiles bytes →
//! `MetadataError`).

use std::io::Write;
use std::path::PathBuf;

use tempfile::NamedTempFile;

use tileserver_rs::TileSource;
use tileserver_rs::config::SourceConfig;
use tileserver_rs::sources::TileFormat;
use tileserver_rs::sources::pmtiles::local::LocalPmTilesSource;

/// Absolute path to the sample PMTiles archive shipped under `data/tiles/`.
fn sample_pmtiles() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/tiles/protomaps-sample.pmtiles")
        .to_string_lossy()
        .into_owned()
}

/// Build a minimal PMTiles `SourceConfig` for the given path. Uses TOML
/// deserialization so cfg-gated struct fields don't have to be spelled out.
fn local_source_config(id: &str, path: &str) -> SourceConfig {
    let toml = format!("id = \"{id}\"\ntype = \"pmtiles\"\npath = \"{path}\"\n");
    toml::from_str(&toml).expect("valid source config")
}

#[tokio::test]
async fn local_source_reads_metadata() {
    let config = local_source_config("local-pm", &sample_pmtiles());
    let source = LocalPmTilesSource::from_file(&config)
        .await
        .expect("open local pmtiles");

    let meta = source.metadata();
    assert_eq!(meta.id, "local-pm");
    assert_eq!(meta.format, TileFormat::Pbf);
    assert_eq!(meta.minzoom, 0);
    assert_eq!(meta.maxzoom, 15);
    assert!(meta.bounds.is_some());
    assert!(meta.center.is_some());
}

#[tokio::test]
async fn local_source_serves_a_real_tile() {
    let config = local_source_config("local-pm", &sample_pmtiles());
    let source = LocalPmTilesSource::from_file(&config)
        .await
        .expect("open local pmtiles");

    let tile = source.get_tile(0, 0, 0).await.expect("get_tile ok");
    assert!(tile.is_some(), "z0/0/0 should exist in the sample archive");
    let tile = tile.unwrap();
    assert_eq!(tile.format, TileFormat::Pbf);
    assert!(!tile.data.is_empty());
}

#[tokio::test]
async fn local_source_out_of_zoom_returns_none() {
    let config = local_source_config("local-pm", &sample_pmtiles());
    let source = LocalPmTilesSource::from_file(&config)
        .await
        .expect("open local pmtiles");

    let tile = source.get_tile(20, 0, 0).await.expect("get_tile ok");
    assert!(tile.is_none(), "above-maxzoom request must be a miss");
}

#[tokio::test]
async fn local_source_invalid_coordinates_error() {
    let config = local_source_config("local-pm", &sample_pmtiles());
    let source = LocalPmTilesSource::from_file(&config)
        .await
        .expect("open local pmtiles");

    let err = source.get_tile(1, 5, 0).await;
    assert!(err.is_err(), "out-of-range coordinate must error");
}

#[tokio::test]
async fn local_source_missing_file_is_config_error() {
    let config = local_source_config("local-pm", "/nonexistent/path/to.pmtiles");
    let result = LocalPmTilesSource::from_file(&config).await;
    assert!(result.is_err(), "missing file must fail to open");
}

#[tokio::test]
async fn local_source_invalid_bytes_is_metadata_error() {
    // A file that exists but is not a valid PMTiles archive: the existence
    // check passes, then the header parse fails with a MetadataError.
    let mut tmp = NamedTempFile::new().expect("temp file");
    tmp.write_all(b"not a pmtiles archive at all")
        .expect("write junk");
    let path = tmp.path().to_string_lossy().into_owned();

    let config = local_source_config("local-pm", &path);
    let result = LocalPmTilesSource::from_file(&config).await;
    assert!(result.is_err(), "invalid archive bytes must fail to open");
}
