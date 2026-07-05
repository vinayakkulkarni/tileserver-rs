//! Conversion pipeline orchestrator: read input, build tiles, write PMTiles.

use super::args::ConvertArgs;
use super::input::{self, ConvertFeature, InputFormat, PropValue};
use super::progress::Progress;
use super::tile_builder::{TileBuilder, TileOptions};
use crate::error::{Result, TileServerError};
use pmtiles::{PmTilesWriter, TileType};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// Files at or above this size print the planet-scale tooling hint.
pub const LARGE_FILE_BYTES: u64 = 1_073_741_824;

/// Planet-scale builder link shown for large inputs and in `--help`.
pub const PLANET_SCALE_TOOL_URL: &str = "https://github.com/felt/tippecanoe";

/// Whether an input of `len` bytes should trigger the planet-scale hint.
#[must_use]
pub fn is_large_file(len: u64) -> bool {
    len >= LARGE_FILE_BYTES
}

/// Default layer name derived from the input filename stem.
#[must_use]
pub fn default_layer_name(input: &Path) -> String {
    input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("layer")
        .to_string()
}

/// WGS84 bounds `[min_lon, min_lat, max_lon, max_lat]` spanning all features, or
/// the world extent when there are none.
#[must_use]
pub fn compute_bounds(features: &[ConvertFeature]) -> [f64; 4] {
    let mut bounds = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for feature in features {
        let (min_lon, min_lat, max_lon, max_lat) =
            super::tile_builder::geometry_bbox(&feature.geometry);
        bounds[0] = bounds[0].min(min_lon);
        bounds[1] = bounds[1].min(min_lat);
        bounds[2] = bounds[2].max(max_lon);
        bounds[3] = bounds[3].max(max_lat);
    }
    if bounds[0] > bounds[2] {
        return [-180.0, -85.051_129, 180.0, 85.051_129];
    }
    bounds
}

/// The TileJSON `type` string for a property value.
fn field_type(value: &PropValue) -> &'static str {
    match value {
        PropValue::String(_) => "String",
        PropValue::Int(_) | PropValue::Float(_) => "Number",
        PropValue::Bool(_) => "Boolean",
    }
}

/// Build the TileJSON 3.0 metadata JSON string, including the `vector_layers`
/// block synthesized from the union of feature property keys.
#[must_use]
pub fn build_metadata(
    features: &[ConvertFeature],
    layer_name: &str,
    min_zoom: u8,
    max_zoom: u8,
    bounds: [f64; 4],
) -> String {
    let mut fields = std::collections::BTreeMap::<String, &'static str>::new();
    for feature in features {
        for (key, value) in &feature.properties {
            fields
                .entry(key.clone())
                .or_insert_with(|| field_type(value));
        }
    }
    let fields_json = serde_json::Value::Object(
        fields
            .into_iter()
            .map(|(k, v)| (k, serde_json::Value::String(v.to_string())))
            .collect(),
    );
    let metadata = serde_json::json!({
        "name": layer_name,
        "format": "pbf",
        "minzoom": min_zoom,
        "maxzoom": max_zoom,
        "bounds": bounds,
        "vector_layers": [{
            "id": layer_name,
            "minzoom": min_zoom,
            "maxzoom": max_zoom,
            "fields": fields_json,
        }],
    });
    metadata.to_string()
}

/// Read the input file into owned features, dispatching on detected format.
fn read_features(args: &ConvertArgs) -> Result<Vec<ConvertFeature>> {
    let text = std::fs::read_to_string(&args.input)
        .map_err(|e| TileServerError::ConvertError(format!("read input: {e}")))?;
    match input::detect(&args.input)? {
        InputFormat::GeoJson => input::geojson::read_geojson(&text, args.id_property.clone()),
        InputFormat::Csv => input::csv::read_csv(
            &text,
            args.lat.as_deref(),
            args.lng.as_deref(),
            args.geometry_column.as_deref(),
            args.id_property.as_deref(),
        ),
    }
}

/// Build [`TileOptions`] from CLI args and the resolved layer name.
fn tile_options(args: &ConvertArgs, layer_name: String) -> TileOptions {
    TileOptions {
        min_zoom: args.min_zoom,
        max_zoom: effective_max_zoom(args),
        layer_name,
        simplification: args.simplification,
        include_properties: args.include_properties.clone(),
        exclude_properties: args.exclude_properties.clone(),
        drop_densest: args.drop_densest,
    }
}

/// Resolve the max zoom, capping `--auto-max-zoom` at 14 per the issue default.
fn effective_max_zoom(args: &ConvertArgs) -> u8 {
    if args.auto_max_zoom {
        14
    } else {
        args.max_zoom
    }
}

/// Write encoded tiles into a PMTiles archive at `output`.
fn write_pmtiles(
    output: &Path,
    tiles: Vec<(pmtiles::TileCoord, Vec<u8>)>,
    min_zoom: u8,
    max_zoom: u8,
    bounds: [f64; 4],
    metadata: &str,
) -> Result<()> {
    let center_lon = (bounds[0] + bounds[2]) / 2.0;
    let center_lat = (bounds[1] + bounds[3]) / 2.0;
    let file = File::create(output)
        .map_err(|e| TileServerError::ConvertError(format!("create output: {e}")))?;
    let mut writer = PmTilesWriter::new(TileType::Mvt)
        .min_zoom(min_zoom)
        .max_zoom(max_zoom)
        .bounds(bounds[0], bounds[1], bounds[2], bounds[3])
        .center_zoom(min_zoom.midpoint(max_zoom))
        .center(center_lon, center_lat)
        .metadata(metadata)
        .create(BufWriter::new(file))
        .map_err(|e| TileServerError::ConvertError(format!("pmtiles create: {e}")))?;

    for (coord, bytes) in tiles {
        writer
            .add_tile(coord, &bytes)
            .map_err(|e| TileServerError::ConvertError(format!("add tile: {e}")))?;
    }
    writer
        .finalize()
        .map_err(|e| TileServerError::ConvertError(format!("finalize: {e}")))?;
    Ok(())
}

fn maybe_print_large_file_hint(input: &Path) {
    if let Ok(meta) = std::fs::metadata(input)
        && is_large_file(meta.len())
    {
        eprintln!(
            "Input is larger than 1 GiB. For planet-scale tile generation (OSM PBF, Overture Maps), see {PLANET_SCALE_TOOL_URL}"
        );
    }
}

/// Convert `args.input` into a PMTiles archive at `output`, returning the
/// written path.
///
/// # Errors
///
/// Returns [`TileServerError::ConvertError`] when reading, tiling, or writing
/// fails.
pub fn convert_to_pmtiles(args: &ConvertArgs, output: &Path) -> Result<()> {
    maybe_print_large_file_hint(&args.input);
    let mut progress = Progress::hidden();

    let features = read_features(args)?;
    progress.tick_features(features.len() as u64);
    if features.is_empty() {
        return Err(TileServerError::ConvertError(
            "no features found in input".to_string(),
        ));
    }

    let layer_name = args
        .layer_name
        .clone()
        .unwrap_or_else(|| default_layer_name(&args.input));
    let min_zoom = args.min_zoom;
    let max_zoom = effective_max_zoom(args);
    let bounds = compute_bounds(&features);
    let metadata = build_metadata(&features, &layer_name, min_zoom, max_zoom, bounds);

    let mut builder = TileBuilder::new(tile_options(args, layer_name));
    for feature in features {
        builder.add_feature(feature);
    }
    let tiles = builder.finish()?;
    progress.tick_tiles(tiles.len() as u64);

    write_pmtiles(output, tiles, min_zoom, max_zoom, bounds, &metadata)?;
    progress.finish();
    Ok(())
}

/// Execute the end-to-end conversion described by `args`.
///
/// # Errors
///
/// Returns [`TileServerError::ConvertError`] on any pipeline failure.
pub fn run(args: ConvertArgs) -> Result<()> {
    let output = args.output.clone().ok_or_else(|| {
        TileServerError::ConvertError("--output is required (or use --serve)".to_string())
    })?;
    convert_to_pmtiles(&args, &output)?;
    tracing::info!("wrote {}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::input::Geometry;
    use super::*;
    use std::collections::BTreeMap;

    fn feature(lon: f64, lat: f64, props: &[(&str, PropValue)]) -> ConvertFeature {
        let mut properties = BTreeMap::new();
        for (k, v) in props {
            properties.insert((*k).to_string(), v.clone());
        }
        ConvertFeature {
            geometry: Geometry::Point((lon, lat)),
            properties,
            id: None,
        }
    }

    fn args_for(input: &str, output: &str) -> ConvertArgs {
        ConvertArgs {
            input: input.into(),
            output: Some(output.into()),
            min_zoom: 0,
            max_zoom: 6,
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

    #[test]
    fn is_large_file_threshold() {
        assert!(!is_large_file(1_000));
        assert!(is_large_file(LARGE_FILE_BYTES));
        assert!(is_large_file(LARGE_FILE_BYTES + 1));
    }

    #[test]
    fn default_layer_name_from_stem() {
        assert_eq!(default_layer_name(Path::new("/a/cities.geojson")), "cities");
    }

    #[test]
    fn compute_bounds_spans_features() {
        let feats = vec![feature(8.5, 47.3, &[]), feature(8.6, 47.4, &[])];
        let b = compute_bounds(&feats);
        assert_eq!(b, [8.5, 47.3, 8.6, 47.4]);
    }

    #[test]
    fn compute_bounds_empty_is_world() {
        let b = compute_bounds(&[]);
        assert_eq!(b[0], -180.0);
        assert_eq!(b[2], 180.0);
    }

    #[test]
    fn metadata_has_vector_layers_and_fields() {
        let feats = vec![feature(
            8.5,
            47.3,
            &[
                ("name", PropValue::String("A".into())),
                ("pop", PropValue::Int(5)),
            ],
        )];
        let meta = build_metadata(&feats, "places", 0, 14, [8.5, 47.3, 8.5, 47.3]);
        let parsed: serde_json::Value = serde_json::from_str(&meta).unwrap();
        assert_eq!(parsed["vector_layers"][0]["id"], "places");
        assert_eq!(parsed["vector_layers"][0]["fields"]["name"], "String");
        assert_eq!(parsed["vector_layers"][0]["fields"]["pop"], "Number");
    }

    #[test]
    fn effective_max_zoom_caps_auto_at_14() {
        let mut args = args_for("x.geojson", "y.pmtiles");
        args.auto_max_zoom = true;
        args.max_zoom = 3;
        assert_eq!(effective_max_zoom(&args), 14);
    }

    #[test]
    fn convert_geojson_produces_pmtiles_file() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("pts.geojson");
        let output = dir.path().join("out.pmtiles");
        std::fs::write(
            &input,
            r#"{"type":"FeatureCollection","features":[
                {"type":"Feature","geometry":{"type":"Point","coordinates":[8.5,47.3]},
                 "properties":{"name":"A"}}
            ]}"#,
        )
        .unwrap();
        let args = args_for(input.to_str().unwrap(), output.to_str().unwrap());
        convert_to_pmtiles(&args, &output).unwrap();
        assert!(output.exists());
        assert!(std::fs::metadata(&output).unwrap().len() > 0);
    }

    #[test]
    fn convert_csv_lat_lng_produces_pmtiles_file() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("c.csv");
        let output = dir.path().join("c.pmtiles");
        std::fs::write(&input, "latitude,longitude\n47.3,8.5\n47.4,8.6\n").unwrap();
        let mut args = args_for(input.to_str().unwrap(), output.to_str().unwrap());
        args.lat = Some("latitude".to_string());
        args.lng = Some("longitude".to_string());
        convert_to_pmtiles(&args, &output).unwrap();
        assert!(output.exists());
    }

    #[test]
    fn convert_empty_features_errors() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("empty.geojson");
        let output = dir.path().join("e.pmtiles");
        std::fs::write(&input, r#"{"type":"FeatureCollection","features":[]}"#).unwrap();
        let args = args_for(input.to_str().unwrap(), output.to_str().unwrap());
        assert!(convert_to_pmtiles(&args, &output).is_err());
    }
}
