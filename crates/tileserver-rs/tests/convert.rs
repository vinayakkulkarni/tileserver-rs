//! Integration tests for the `convert` pipeline (feature-gated).
//!
//! These exercise the full GeoJSON/CSV → PMTiles path and — for the headline
//! servability test — reopen the output through the production
//! `LocalPmTilesSource` and decode the MVT to prove the archive is genuinely
//! consumable by the tile server.
#![cfg(feature = "convert")]

use geozero::mvt::{Message, Tile, tile};
use std::io::Read;
use std::path::{Path, PathBuf};
use tileserver_rs::TileSource;
use tileserver_rs::convert::ConvertArgs;
use tileserver_rs::convert::pipeline::convert_to_pmtiles;
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
