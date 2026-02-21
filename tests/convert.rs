//! Integration tests for the `convert` feature.
//!
//! Tests the full GeoJSON → PMTiles pipeline end to end using the public API.
//! Run with: `cargo test --features convert -p tileserver-rs convert`

#![cfg(feature = "convert")]

use std::io::Write;
use tempfile::{Builder, TempDir};
use tileserver_rs::convert::pipeline::{run, ConvertOptions};
use tileserver_rs::convert::progress::SilentReporter;

// ── helpers ──────────────────────────────────────────────────────────────────

fn geojson_file(content: &str) -> tempfile::NamedTempFile {
    let mut f = Builder::new()
        .suffix(".geojson")
        .tempfile()
        .expect("create tempfile");
    f.write_all(content.as_bytes()).expect("write geojson");
    f
}

fn opts(layer: &str) -> ConvertOptions {
    ConvertOptions {
        min_zoom: 0,
        max_zoom: Some(4),
        layer_name: layer.to_string(),
        simplification: None,
        id_property: None,
        include_properties: None,
        exclude_properties: vec![],
    }
}

fn output_in(dir: &TempDir) -> std::path::PathBuf {
    dir.path().join("out.pmtiles")
}

// ── pipeline success cases ────────────────────────────────────────────────────

#[test]
fn converts_point_geojson_to_pmtiles() {
    let input = geojson_file(
        r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":{"type":"Point","coordinates":[13.4,52.5]},"properties":{"name":"Berlin"}}
        ]}"#,
    );
    let dir = TempDir::new().unwrap();
    let out = output_in(&dir);

    run(input.path(), &out, &opts("places"), &SilentReporter).unwrap();

    let meta = std::fs::metadata(&out).expect("output file must exist");
    assert!(meta.len() > 0, "PMTiles archive must not be empty");
}

#[test]
fn converts_linestring_geojson_to_pmtiles() {
    let input = geojson_file(
        r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":{"type":"LineString","coordinates":[[0,0],[10,10],[20,5]]},"properties":{}}
        ]}"#,
    );
    let dir = TempDir::new().unwrap();
    let out = output_in(&dir);

    run(input.path(), &out, &opts("lines"), &SilentReporter).unwrap();

    assert!(std::fs::metadata(&out).unwrap().len() > 0);
}

#[test]
fn converts_polygon_geojson_to_pmtiles() {
    let input = geojson_file(
        r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,10],[0,10],[0,0]]]},"properties":{"area":"square"}}
        ]}"#,
    );
    let dir = TempDir::new().unwrap();
    let out = output_in(&dir);

    run(input.path(), &out, &opts("areas"), &SilentReporter).unwrap();

    assert!(std::fs::metadata(&out).unwrap().len() > 0);
}

#[test]
fn converts_multipolygon_geojson_to_pmtiles() {
    let input = geojson_file(
        r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":{"type":"MultiPolygon","coordinates":
                [[[[0,0],[5,0],[5,5],[0,5],[0,0]]],[[[10,10],[20,10],[20,20],[10,20],[10,10]]]]
            },"properties":{}}
        ]}"#,
    );
    let dir = TempDir::new().unwrap();
    let out = output_in(&dir);

    run(input.path(), &out, &opts("regions"), &SilentReporter).unwrap();

    assert!(std::fs::metadata(&out).unwrap().len() > 0);
}

#[test]
fn converts_multiple_features_to_pmtiles() {
    let input = geojson_file(
        r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":{"type":"Point","coordinates":[-74.0,40.7]},"properties":{"city":"New York","pop":8000000}},
            {"type":"Feature","geometry":{"type":"Point","coordinates":[2.35,48.85]},"properties":{"city":"Paris","pop":2000000}},
            {"type":"Feature","geometry":{"type":"Point","coordinates":[139.7,35.7]},"properties":{"city":"Tokyo","pop":14000000}},
            {"type":"Feature","geometry":{"type":"Point","coordinates":[13.4,52.5]},"properties":{"city":"Berlin","pop":3600000}}
        ]}"#,
    );
    let dir = TempDir::new().unwrap();
    let out = output_in(&dir);

    run(input.path(), &out, &opts("cities"), &SilentReporter).unwrap();

    assert!(std::fs::metadata(&out).unwrap().len() > 0);
}

#[test]
fn converts_bare_geometry_geojson() {
    // A GeoJSON file that is just a Geometry (no Feature wrapper)
    let input = geojson_file(r#"{"type":"Point","coordinates":[0.0,0.0]}"#);
    let dir = TempDir::new().unwrap();
    let out = output_in(&dir);

    run(input.path(), &out, &opts("point"), &SilentReporter).unwrap();

    assert!(std::fs::metadata(&out).unwrap().len() > 0);
}

#[test]
fn output_is_created_at_specified_path() {
    let input = geojson_file(
        r#"{"type":"Feature","geometry":{"type":"Point","coordinates":[0,0]},"properties":{}}"#,
    );
    let dir = TempDir::new().unwrap();
    let custom_out = dir.path().join("my_tiles.pmtiles");

    run(input.path(), &custom_out, &opts("layer"), &SilentReporter).unwrap();

    assert!(custom_out.exists(), "output file must exist at given path");
}

// ── narrow zoom range ─────────────────────────────────────────────────────────

#[test]
fn respects_min_max_zoom_range() {
    let input = geojson_file(
        r#"{"type":"Feature","geometry":{"type":"Point","coordinates":[0.0,0.0]},"properties":{}}"#,
    );
    let dir = TempDir::new().unwrap();
    let out = output_in(&dir);

    let narrow = ConvertOptions {
        min_zoom: 2,
        max_zoom: Some(3),
        layer_name: "test".to_string(),
        simplification: None,
        id_property: None,
        include_properties: None,
        exclude_properties: vec![],
    };
    run(input.path(), &out, &narrow, &SilentReporter).unwrap();

    assert!(std::fs::metadata(&out).unwrap().len() > 0);
}

#[test]
fn single_zoom_level_works() {
    let input = geojson_file(
        r#"{"type":"Feature","geometry":{"type":"Point","coordinates":[0.0,0.0]},"properties":{}}"#,
    );
    let dir = TempDir::new().unwrap();
    let out = output_in(&dir);

    let single = ConvertOptions {
        min_zoom: 5,
        max_zoom: Some(5),
        layer_name: "test".to_string(),
        simplification: None,
        id_property: None,
        include_properties: None,
        exclude_properties: vec![],
    };
    run(input.path(), &out, &single, &SilentReporter).unwrap();

    assert!(std::fs::metadata(&out).unwrap().len() > 0);
}

// ── error cases ───────────────────────────────────────────────────────────────

#[test]
fn empty_feature_collection_returns_error() {
    let input = geojson_file(r#"{"type":"FeatureCollection","features":[]}"#);
    let dir = TempDir::new().unwrap();
    let out = output_in(&dir);

    let result = run(input.path(), &out, &opts("empty"), &SilentReporter);
    assert!(result.is_err(), "empty feature collection must fail");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("no features") || msg.contains("empty"),
        "error message should mention empty/no features: {msg}"
    );
}

#[test]
fn invalid_json_returns_error() {
    let input = geojson_file("this is not valid json {{{");
    let dir = TempDir::new().unwrap();
    let out = output_in(&dir);

    let result = run(input.path(), &out, &opts("invalid"), &SilentReporter);
    assert!(result.is_err(), "invalid JSON must return an error");
}

#[test]
fn unsupported_extension_returns_error() {
    // A file with .csv extension should be rejected before any parsing
    let mut f = Builder::new()
        .suffix(".csv")
        .tempfile()
        .expect("create tempfile");
    f.write_all(b"lon,lat\n13.4,52.5").expect("write");

    let dir = TempDir::new().unwrap();
    let out = output_in(&dir);

    let result = run(f.path(), &out, &opts("csv"), &SilentReporter);
    assert!(result.is_err(), "unsupported extension must fail");
}

#[test]
fn nonexistent_input_returns_error() {
    let dir = TempDir::new().unwrap();
    let out = output_in(&dir);
    let missing = std::path::Path::new("/tmp/nonexistent_tileserver_test_file_xyz.geojson");

    let result = run(missing, &out, &opts("missing"), &SilentReporter);
    assert!(result.is_err(), "missing input file must fail");
}
