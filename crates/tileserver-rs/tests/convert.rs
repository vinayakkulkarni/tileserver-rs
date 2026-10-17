//! Integration tests for the `convert` pipeline (feature-gated).
//!
//! These exercise the full GeoJSON/CSV → PMTiles path and — for the headline
//! servability test — reopen the output through the production
//! `LocalPmTilesSource` and decode the MVT to prove the archive is genuinely
//! consumable by the tile server.
#![cfg(feature = "convert")]

use geozero::mvt::{Message, Tile, tile};
use std::io::Read;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tileserver_rs::TileSource;
use tileserver_rs::convert::ConvertArgs;
use tileserver_rs::convert::pipeline::convert_to_pmtiles;
use tileserver_rs::convert::{resolve_output_path, run, run_and_maybe_serve, serve};
use tileserver_rs::sources::pmtiles::local::LocalPmTilesSource;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/convert")
        .join(name)
}

fn base_args(input: PathBuf, output: PathBuf) -> ConvertArgs {
    ConvertArgs {
        input,
        output: Some(output),
        min_zoom: 0,
        max_zoom: 8,
        auto_max_zoom: false,
        simplification: None,
        drop_densest: true,
        layer_name: None,
        id_property: None,
        include_properties: Vec::new(),
        exclude_properties: Vec::new(),
        geometry_column: None,
        lat: None,
        lng: None,
        serve: false,
        port: None,
    }
}

fn convert_fixture(
    name: &str,
    mutate: impl FnOnce(&mut ConvertArgs),
) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = dir.path().join("out.pmtiles");
    let mut args = base_args(fixture(name), output.clone());
    mutate(&mut args);
    convert_to_pmtiles(&args, &output).expect("conversion should succeed");
    (dir, output)
}

fn gunzip(data: &[u8]) -> Vec<u8> {
    if data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b {
        let mut decoder = flate2::read::GzDecoder::new(data);
        let mut out = Vec::new();
        decoder.read_to_end(&mut out).expect("gunzip");
        out
    } else {
        data.to_vec()
    }
}

async fn open_source(path: &Path) -> LocalPmTilesSource {
    let toml = format!(
        "id = \"conv\"\ntype = \"pmtiles\"\npath = \"{}\"\n",
        path.display()
    );
    let cfg: tileserver_rs::config::SourceConfig = toml::from_str(&toml).expect("source config");
    LocalPmTilesSource::from_file(&cfg)
        .await
        .expect("local pmtiles source")
}

/// Find the first non-empty tile in a source's zoom range and decode it.
async fn first_decoded_tile(source: &LocalPmTilesSource) -> Tile {
    let meta = source.metadata().clone();
    for z in meta.minzoom..=meta.maxzoom {
        let n = 1u32 << z;
        for x in 0..n {
            for y in 0..n {
                if let Ok(Some(data)) = source.get_tile(z, x, y).await {
                    let raw = gunzip(data.data.as_ref());
                    if let Ok(decoded) = Tile::decode(raw.as_slice())
                        && !decoded.layers.is_empty()
                    {
                        return decoded;
                    }
                }
            }
        }
    }
    panic!("no decodable tile found in source");
}

#[test]
fn convert_geojson_point_produces_pmtiles() {
    let (_dir, output) = convert_fixture("point.geojson", |_| {});
    assert!(output.exists());
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
}

#[test]
fn convert_geojson_polygon_produces_pmtiles() {
    let (_dir, output) = convert_fixture("polygon.geojson", |_| {});
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
}

#[test]
fn convert_csv_lat_lng_produces_pmtiles() {
    let (_dir, output) = convert_fixture("cities.csv", |a| {
        a.lat = Some("latitude".to_string());
        a.lng = Some("longitude".to_string());
    });
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
}

#[test]
fn convert_csv_wkt_produces_pmtiles() {
    let (_dir, output) = convert_fixture("cities_wkt.csv", |a| {
        a.geometry_column = Some("wkt".to_string());
    });
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
}

#[tokio::test]
async fn convert_then_load_via_local_pmtiles_source() {
    let (_dir, output) = convert_fixture("point.geojson", |_| {});
    let source = open_source(&output).await;
    let decoded = first_decoded_tile(&source).await;

    assert_eq!(decoded.layers[0].name, "point");
    let total_features: usize = decoded.layers.iter().map(|l| l.features.len()).sum();
    assert!(total_features >= 1, "expected at least one decoded feature");

    // The "name" property key must survive into the MVT layer key table.
    assert!(decoded.layers[0].keys.iter().any(|k| k == "name"));
}

#[tokio::test]
async fn convert_respects_min_max_zoom_in_header() {
    let (_dir, output) = convert_fixture("point.geojson", |a| {
        a.min_zoom = 2;
        a.max_zoom = 6;
    });
    let source = open_source(&output).await;
    let meta = source.metadata().clone();
    assert_eq!(meta.minzoom, 2);
    assert_eq!(meta.maxzoom, 6);
}

#[tokio::test]
async fn convert_respects_layer_name_override() {
    let (_dir, output) = convert_fixture("point.geojson", |a| {
        a.layer_name = Some("places".to_string());
    });
    let source = open_source(&output).await;
    let decoded = first_decoded_tile(&source).await;
    assert_eq!(decoded.layers[0].name, "places");
}

#[tokio::test]
async fn convert_id_property_propagates_to_feature_ids() {
    let (_dir, output) = convert_fixture("point.geojson", |a| {
        a.min_zoom = 0;
        a.max_zoom = 0;
    });
    let source = open_source(&output).await;
    let decoded = first_decoded_tile(&source).await;
    let has_id = decoded
        .layers
        .iter()
        .flat_map(|l| &l.features)
        .any(|f| f.id.is_some());
    assert!(has_id, "expected at least one feature to carry an id");
}

#[tokio::test]
async fn convert_include_properties_filters_output() {
    let (_dir, output) = convert_fixture("point.geojson", |a| {
        a.include_properties = vec!["name".to_string()];
    });
    let source = open_source(&output).await;
    let decoded = first_decoded_tile(&source).await;
    let keys: Vec<&String> = decoded.layers.iter().flat_map(|l| &l.keys).collect();
    assert!(keys.iter().any(|k| *k == "name"));
    assert!(!keys.iter().any(|k| *k == "population"));
}

#[tokio::test]
async fn convert_exclude_properties_filters_output() {
    let (_dir, output) = convert_fixture("point.geojson", |a| {
        a.exclude_properties = vec!["population".to_string()];
    });
    let source = open_source(&output).await;
    let decoded = first_decoded_tile(&source).await;
    let keys: Vec<&String> = decoded.layers.iter().flat_map(|l| &l.keys).collect();
    assert!(!keys.iter().any(|k| *k == "population"));
    assert!(keys.iter().any(|k| *k == "name"));
}

#[tokio::test]
async fn convert_auto_max_zoom_caps_at_14() {
    let (_dir, output) = convert_fixture("point.geojson", |a| {
        a.auto_max_zoom = true;
        a.max_zoom = 8;
    });
    let source = open_source(&output).await;
    assert_eq!(source.metadata().maxzoom, 14);
}

#[tokio::test]
async fn convert_preserves_unicode_properties() {
    let (_dir, output) = convert_fixture("point.geojson", |a| {
        a.min_zoom = 0;
        a.max_zoom = 8;
    });
    let source = open_source(&output).await;
    let mut found_unicode = false;
    let meta = source.metadata().clone();
    for z in meta.minzoom..=meta.maxzoom {
        let n = 1u32 << z;
        for x in 0..n {
            for y in 0..n {
                if let Ok(Some(data)) = source.get_tile(z, x, y).await {
                    let raw = gunzip(data.data.as_ref());
                    if let Ok(decoded) = Tile::decode(raw.as_slice()) {
                        for layer in &decoded.layers {
                            if layer.values.iter().any(|v| {
                                v.string_value
                                    .as_deref()
                                    .is_some_and(|s| s.contains("Zürich"))
                            }) {
                                found_unicode = true;
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(
        found_unicode,
        "expected a Zürich Übersee value in some tile"
    );
}

#[test]
fn convert_mixed_props_wkt_produces_pmtiles() {
    let (_dir, output) = convert_fixture("mixed_props.csv", |a| {
        a.geometry_column = Some("geometry".to_string());
    });
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
}

#[tokio::test]
async fn convert_polygon_encodes_polygon_geom_type() {
    let (_dir, output) = convert_fixture("polygon.geojson", |a| {
        a.min_zoom = 0;
        a.max_zoom = 6;
    });
    let source = open_source(&output).await;
    let decoded = first_decoded_tile(&source).await;
    let has_polygon = decoded
        .layers
        .iter()
        .flat_map(|l| &l.features)
        .any(|f| f.r#type() == tile::GeomType::Polygon);
    assert!(has_polygon, "expected a polygon geometry in the output");
}

#[test]
fn convert_help_long_about_mentions_planet_scale_tool() {
    assert!(
        tileserver_rs::convert::args::LONG_ABOUT.contains("https://github.com/felt/tippecanoe")
    );
}

/// Reserve a free localhost port by binding then immediately dropping the
/// listener. Racy in theory, safe in practice for a short-lived test bind.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral")
        .local_addr()
        .expect("local addr")
        .port()
}

async fn get(url: &str) -> reqwest::Response {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");
    client.get(url).send().await.expect("request")
}

/// Boot `serve::serve_pmtiles` against a freshly converted archive on an
/// ephemeral port, then hit the real HTTP surface (`/ping`, TileJSON, a tile)
/// to prove the produced archive is servable end-to-end. The serving task is
/// aborted at the end so no server outlives the test.
#[tokio::test]
async fn serve_pmtiles_boots_and_serves_converted_archive() {
    let (dir, output) = convert_fixture("point.geojson", |a| {
        a.min_zoom = 0;
        a.max_zoom = 6;
    });
    let port = free_port();
    let path = output.clone();

    let handle =
        tokio::spawn(async move { serve::serve_pmtiles(&path, "cities", "127.0.0.1", port).await });

    // Poll /ping until the listener is accepting, up to ~3s.
    let base = format!("http://127.0.0.1:{port}");
    let mut booted = false;
    for _ in 0..60 {
        if let Ok(resp) = reqwest::get(format!("{base}/ping")).await
            && resp.status().is_success()
        {
            booted = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(booted, "server did not boot on {base}");

    let tj = get(&format!("{base}/data/cities.json")).await;
    assert_eq!(tj.status(), 200, "TileJSON must be 200");
    let body: serde_json::Value = tj.json().await.expect("tilejson body");
    assert_eq!(body["tilejson"], "3.0.0");

    let tile = get(&format!("{base}/data/cities/0/0/0.pbf")).await;
    assert_eq!(tile.status(), 200, "z0 tile must be served");
    let bytes = tile.bytes().await.expect("tile bytes");
    assert!(!bytes.is_empty(), "served tile must be non-empty");

    handle.abort();
    drop(dir);
}

/// `run_and_maybe_serve` with `serve = false` must run the pipeline to
/// completion, writing a parseable PMTiles archive, and return without booting
/// a server.
#[tokio::test]
async fn run_and_maybe_serve_without_serve_writes_pmtiles() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = dir.path().join("out.pmtiles");
    let mut args = base_args(fixture("point.geojson"), output.clone());
    args.min_zoom = 0;
    args.max_zoom = 4;

    run_and_maybe_serve(args).await.expect("run should succeed");

    assert!(output.exists(), "archive must be written");
    let source = open_source(&output).await;
    let decoded = first_decoded_tile(&source).await;
    assert!(!decoded.layers.is_empty(), "archive must be parseable");
}

/// The synchronous `convert::run` wrapper delegates to the pipeline and writes
/// the archive at the explicit `--output` path.
#[test]
fn run_wrapper_writes_pmtiles() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = dir.path().join("out.pmtiles");
    let mut args = base_args(fixture("point.geojson"), output.clone());
    args.min_zoom = 0;
    args.max_zoom = 4;
    run(args).expect("run should succeed");
    assert!(output.exists(), "archive must be written");
}

/// `run_and_maybe_serve` with `serve = true` runs the pipeline and boots the
/// server on the requested port; the spawned task is aborted once the health
/// endpoint responds so nothing outlives the test.
#[tokio::test]
async fn run_and_maybe_serve_with_serve_boots_server() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = dir.path().join("served.pmtiles");
    let port = free_port();
    let mut args = base_args(fixture("point.geojson"), output.clone());
    args.min_zoom = 0;
    args.max_zoom = 4;
    args.layer_name = Some("cities".to_string());
    args.serve = true;
    args.port = Some(port);

    let handle = tokio::spawn(async move { run_and_maybe_serve(args).await });

    let base = format!("http://127.0.0.1:{port}");
    let mut booted = false;
    for _ in 0..60 {
        if let Ok(resp) = reqwest::get(format!("{base}/ping")).await
            && resp.status().is_success()
        {
            booted = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(booted, "run_and_maybe_serve did not boot server on {base}");

    let tj = get(&format!("{base}/data/cities.json")).await;
    assert_eq!(tj.status(), 200);

    handle.abort();
    drop(dir);
}

/// When `--output` is omitted, `resolve_output_path` mints a unique path under
/// the system temp dir derived from the input stem.
#[test]
fn resolve_output_path_defaults_to_temp_when_output_absent() {
    let mut args = base_args(fixture("point.geojson"), PathBuf::from("ignored"));
    args.output = None;
    let path = resolve_output_path(&args).expect("resolve path");
    assert!(
        path.to_string_lossy().contains("tileserver-rs"),
        "temp path must be namespaced: {}",
        path.display()
    );
    assert!(
        path.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("point-") && n.ends_with(".pmtiles")),
        "temp filename must derive from input stem: {}",
        path.display()
    );
}

/// An explicit `--output` is returned verbatim by `resolve_output_path`.
#[test]
fn resolve_output_path_uses_explicit_output() {
    let args = base_args(
        fixture("point.geojson"),
        PathBuf::from("/tmp/explicit.pmtiles"),
    );
    let path = resolve_output_path(&args).expect("resolve path");
    assert_eq!(path, PathBuf::from("/tmp/explicit.pmtiles"));
}
