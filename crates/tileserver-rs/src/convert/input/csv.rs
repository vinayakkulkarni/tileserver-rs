//! CSV input streaming.
//!
//! Two modes:
//! - `--lat`/`--lng`: synthesize a POINT geometry from two numeric columns.
//! - `--geometry-column` (or auto-detected): parse a WKT column via geozero's
//!   WKT reader. Rows whose WKT fails to parse are skipped with a warning.

use super::geom::GeomCollector;
use super::{ConvertFeature, Geometry, PropValue, resolve_id};
use crate::error::{Result, TileServerError};
use geozero::{GeozeroGeometry, wkt::Wkt};
use std::collections::BTreeMap;

/// Column names that auto-detect as WKT geometry, checked case-insensitively.
const GEOMETRY_CANDIDATES: [&str; 4] = ["geometry", "wkt", "geom", "shape"];

/// How the CSV reader locates each row's geometry.
#[derive(Debug, Clone)]
pub enum CsvGeometryMode {
    /// Two numeric columns holding longitude and latitude.
    LatLng { lat: String, lng: String },
    /// A single column holding a WKT string.
    Wkt { column: String },
}

/// Infer the geometry mode from CLI flags and the header row.
///
/// # Errors
///
/// Returns [`TileServerError::ConvertError`] when no geometry column can be
/// located.
pub fn resolve_mode(
    headers: &[String],
    lat: Option<&str>,
    lng: Option<&str>,
    geometry_column: Option<&str>,
) -> Result<CsvGeometryMode> {
    if let (Some(lat), Some(lng)) = (lat, lng) {
        return Ok(CsvGeometryMode::LatLng {
            lat: lat.to_string(),
            lng: lng.to_string(),
        });
    }
    if let Some(col) = geometry_column {
        return Ok(CsvGeometryMode::Wkt {
            column: col.to_string(),
        });
    }
    if let Some(found) = auto_detect_geometry_column(headers) {
        return Ok(CsvGeometryMode::Wkt { column: found });
    }
    Err(TileServerError::ConvertError(
        "no geometry column found: pass --geometry-column or --lat/--lng".to_string(),
    ))
}

/// Return the first header matching a WKT geometry candidate name.
fn auto_detect_geometry_column(headers: &[String]) -> Option<String> {
    headers
        .iter()
        .find(|h| {
            GEOMETRY_CANDIDATES
                .iter()
                .any(|c| h.eq_ignore_ascii_case(c))
        })
        .cloned()
}

/// Infer a typed property value from a raw CSV cell: bool, then integer, then
/// float, falling back to string.
#[must_use]
pub fn infer_prop(cell: &str) -> PropValue {
    match cell.to_ascii_lowercase().as_str() {
        "true" => return PropValue::Bool(true),
        "false" => return PropValue::Bool(false),
        _ => {}
    }
    if let Ok(i) = cell.parse::<i64>() {
        return PropValue::Int(i);
    }
    if let Ok(f) = cell.parse::<f64>() {
        return PropValue::Float(f);
    }
    PropValue::String(cell.to_string())
}

/// Parse a single WKT string into an owned [`Geometry`], or `None` when it is
/// empty or fails to parse.
fn parse_wkt(cell: &str) -> Option<Geometry> {
    if cell.trim().is_empty() {
        return None;
    }
    let mut collector = GeomCollector::default();
    Wkt(cell).process_geom(&mut collector).ok()?;
    collector.finish()
}

/// Read CSV text into owned features.
///
/// # Errors
///
/// Returns [`TileServerError::ConvertError`] when the CSV cannot be parsed or no
/// geometry column is available.
pub fn read_csv(
    text: &str,
    lat: Option<&str>,
    lng: Option<&str>,
    geometry_column: Option<&str>,
    id_property: Option<&str>,
) -> Result<Vec<ConvertFeature>> {
    let mut reader = csv::Reader::from_reader(text.as_bytes());
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| TileServerError::ConvertError(format!("csv header: {e}")))?
        .iter()
        .map(str::to_string)
        .collect();

    let mode = resolve_mode(&headers, lat, lng, geometry_column)?;
    let mut features = Vec::new();

    for (row_idx, record) in reader.records().enumerate() {
        let record = record.map_err(|e| TileServerError::ConvertError(format!("csv row: {e}")))?;
        let cells: BTreeMap<&str, &str> = headers
            .iter()
            .map(String::as_str)
            .zip(record.iter())
            .collect();

        let Some(geometry) = extract_geometry(&mode, &cells) else {
            tracing::warn!(
                row = row_idx + 2,
                "skipping row: invalid or missing geometry"
            );
            continue;
        };

        let properties = collect_properties(&mode, &headers, &record);
        let id = resolve_id(&properties, id_property);
        features.push(ConvertFeature {
            geometry,
            properties,
            id,
        });
    }

    Ok(features)
}

/// Build the geometry for one row given the resolved mode and its cells.
fn extract_geometry(mode: &CsvGeometryMode, cells: &BTreeMap<&str, &str>) -> Option<Geometry> {
    match mode {
        CsvGeometryMode::LatLng { lat, lng } => {
            let lat_val = cells.get(lat.as_str())?.parse::<f64>().ok()?;
            let lng_val = cells.get(lng.as_str())?.parse::<f64>().ok()?;
            Some(Geometry::Point((lng_val, lat_val)))
        }
        CsvGeometryMode::Wkt { column } => parse_wkt(cells.get(column.as_str())?),
    }
}

/// Collect a row's non-geometry columns into typed properties.
fn collect_properties(
    mode: &CsvGeometryMode,
    headers: &[String],
    record: &csv::StringRecord,
) -> BTreeMap<String, PropValue> {
    let geometry_cols = geometry_column_names(mode);
    headers
        .iter()
        .zip(record.iter())
        .filter(|(header, _)| !geometry_cols.iter().any(|g| g.eq_ignore_ascii_case(header)))
        .map(|(header, cell)| (header.clone(), infer_prop(cell)))
        .collect()
}

/// The column names that hold geometry (and must be excluded from properties).
fn geometry_column_names(mode: &CsvGeometryMode) -> Vec<String> {
    match mode {
        CsvGeometryMode::LatLng { lat, lng } => vec![lat.clone(), lng.clone()],
        CsvGeometryMode::Wkt { column } => vec![column.clone()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LATLNG: &str = "city,latitude,longitude,population\n\
        Zurich,47.37,8.54,400000\n\
        Bern,46.95,7.44,130000\n";

    #[test]
    fn lat_lng_shortcut_produces_points() {
        let feats = read_csv(LATLNG, Some("latitude"), Some("longitude"), None, None).unwrap();
        assert_eq!(feats.len(), 2);
        assert_eq!(feats[0].geometry, Geometry::Point((8.54, 47.37)));
    }

    #[test]
    fn wkt_column_produces_polygons() {
        let csv = "wkt,name\n\"POLYGON((0 0,0 1,1 1,1 0,0 0))\",box\n";
        let feats = read_csv(csv, None, None, Some("wkt"), None).unwrap();
        assert_eq!(feats.len(), 1);
        match &feats[0].geometry {
            Geometry::Polygon(rings) => assert_eq!(rings[0].len(), 5),
            other => panic!("expected polygon, got {other:?}"),
        }
    }

    #[test]
    fn auto_detect_geometry_column_wkt() {
        let csv = "geometry,name\nPOINT(1 2),a\n";
        let feats = read_csv(csv, None, None, None, None).unwrap();
        assert_eq!(feats[0].geometry, Geometry::Point((1.0, 2.0)));
    }

    #[test]
    fn auto_detect_id_column_id() {
        let csv = "id,latitude,longitude\n7,47.0,8.0\n";
        let feats = read_csv(csv, Some("latitude"), Some("longitude"), None, None).unwrap();
        assert_eq!(feats[0].id, Some(7));
    }

    #[test]
    fn auto_detect_id_column_objectid_case_insensitive() {
        let csv = "OBJECTID,latitude,longitude\n99,47.0,8.0\n";
        let feats = read_csv(csv, Some("latitude"), Some("longitude"), None, None).unwrap();
        assert_eq!(feats[0].id, Some(99));
    }

    #[test]
    fn id_property_override_wins_over_auto() {
        let csv = "id,custom,latitude,longitude\n1,42,47.0,8.0\n";
        let feats = read_csv(
            csv,
            Some("latitude"),
            Some("longitude"),
            None,
            Some("custom"),
        )
        .unwrap();
        assert_eq!(feats[0].id, Some(42));
    }

    #[test]
    fn empty_csv_returns_no_features() {
        let csv = "latitude,longitude\n";
        let feats = read_csv(csv, Some("latitude"), Some("longitude"), None, None).unwrap();
        assert!(feats.is_empty());
    }

    #[test]
    fn malformed_wkt_skips_row() {
        let csv = "wkt,name\nNOT_WKT,a\nPOINT(1 2),b\n";
        let feats = read_csv(csv, None, None, Some("wkt"), None).unwrap();
        assert_eq!(feats.len(), 1);
        assert_eq!(feats[0].geometry, Geometry::Point((1.0, 2.0)));
    }

    #[test]
    fn csv_with_header_and_many_rows() {
        let mut csv = String::from("latitude,longitude\n");
        for i in 0..1000 {
            csv.push_str(&format!("{}.0,{}.0\n", 40 + i % 5, 7 + i % 3));
        }
        let feats = read_csv(&csv, Some("latitude"), Some("longitude"), None, None).unwrap();
        assert_eq!(feats.len(), 1000);
    }

    #[test]
    fn int_string_float_bool_property_types_round_trip() {
        let csv = "wkt,count,label,ratio,active\nPOINT(0 0),5,hello,2.5,true\n";
        let feats = read_csv(csv, None, None, Some("wkt"), None).unwrap();
        let p = &feats[0].properties;
        assert_eq!(p.get("count"), Some(&PropValue::Int(5)));
        assert_eq!(
            p.get("label"),
            Some(&PropValue::String("hello".to_string()))
        );
        assert_eq!(p.get("ratio"), Some(&PropValue::Float(2.5)));
        assert_eq!(p.get("active"), Some(&PropValue::Bool(true)));
    }

    #[test]
    fn infer_prop_precedence() {
        assert_eq!(infer_prop("true"), PropValue::Bool(true));
        assert_eq!(infer_prop("42"), PropValue::Int(42));
        assert_eq!(infer_prop("4.5"), PropValue::Float(4.5));
        assert_eq!(infer_prop("hi"), PropValue::String("hi".to_string()));
    }

    #[test]
    fn no_geometry_column_errors() {
        let csv = "name,value\na,1\n";
        assert!(read_csv(csv, None, None, None, None).is_err());
    }

    #[test]
    fn empty_wkt_cell_skips_row() {
        let csv = "wkt,name\n\"\",blank\n\"POINT(1 2)\",kept\n";
        let feats = read_csv(csv, None, None, Some("wkt"), None).unwrap();
        assert_eq!(feats.len(), 1);
        assert_eq!(feats[0].geometry, Geometry::Point((1.0, 2.0)));
    }
}
