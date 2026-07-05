//! Input format detection, the parsed-feature intermediate representation,
//! and streaming readers for GeoJSON and CSV.

pub mod csv;
pub mod geojson;

use crate::error::{Result, TileServerError};
use std::collections::BTreeMap;
use std::path::Path;

/// A single ring or line: an ordered list of WGS84 `(lon, lat)` vertices.
pub type Ring = Vec<(f64, f64)>;

/// A geometry in WGS84 lon/lat, restricted to the OGC simple-feature types the
/// convert pipeline emits. Curves and 3D/measured coordinates are flattened to
/// their 2D planar equivalents by the reader.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    /// A single `(lon, lat)` point.
    Point((f64, f64)),
    /// Multiple points.
    MultiPoint(Vec<(f64, f64)>),
    /// A single line.
    LineString(Ring),
    /// Multiple lines.
    MultiLineString(Vec<Ring>),
    /// A polygon: first ring is the exterior, the rest are holes.
    Polygon(Vec<Ring>),
    /// Multiple polygons.
    MultiPolygon(Vec<Vec<Ring>>),
}

/// A property value carried through from the input into MVT tags.
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    /// UTF-8 string.
    String(String),
    /// 64-bit signed integer.
    Int(i64),
    /// Double-precision float.
    Float(f64),
    /// Boolean.
    Bool(bool),
}

/// A parsed feature: geometry plus ordered properties and an optional feature
/// ID resolved from the configured or auto-detected ID column.
#[derive(Debug, Clone, PartialEq)]
pub struct ConvertFeature {
    /// The feature geometry in WGS84.
    pub geometry: Geometry,
    /// Properties in deterministic (sorted) key order.
    pub properties: BTreeMap<String, PropValue>,
    /// Optional MVT feature ID.
    pub id: Option<u64>,
}

/// Supported input formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    /// GeoJSON (`.geojson` or `.json`).
    GeoJson,
    /// Comma-separated values (`.csv`).
    Csv,
}

/// Detect the input format from a path's extension.
///
/// # Errors
///
/// Returns [`TileServerError::ConvertError`] for unknown or missing extensions.
pub fn detect(path: &Path) -> Result<InputFormat> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("geojson" | "json") => Ok(InputFormat::GeoJson),
        Some("csv") => Ok(InputFormat::Csv),
        other => Err(TileServerError::ConvertError(format!(
            "unsupported input extension: {}",
            other.unwrap_or("<none>")
        ))),
    }
}

/// Candidate ID column names, highest precedence first. Compared
/// case-insensitively against property keys.
pub const ID_CANDIDATES: [&str; 4] = ["id", "gid", "ogc_fid", "objectid"];

/// Resolve a feature ID from a property map: an explicit `id_property` wins,
/// otherwise the first present [`ID_CANDIDATES`] entry (case-insensitive) whose
/// value coerces to an unsigned integer.
#[must_use]
pub fn resolve_id(
    properties: &BTreeMap<String, PropValue>,
    id_property: Option<&str>,
) -> Option<u64> {
    if let Some(explicit) = id_property {
        return properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(explicit))
            .and_then(|(_, v)| prop_to_u64(v));
    }
    for candidate in ID_CANDIDATES {
        if let Some(v) = properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(candidate))
            .map(|(_, v)| v)
            && let Some(id) = prop_to_u64(v)
        {
            return Some(id);
        }
    }
    None
}

/// Coerce a property value to `u64` when it losslessly represents a
/// non-negative integer.
fn prop_to_u64(value: &PropValue) -> Option<u64> {
    match value {
        PropValue::Int(i) if *i >= 0 => Some(*i as u64),
        PropValue::Float(f) if *f >= 0.0 && f.fract() == 0.0 => Some(*f as u64),
        PropValue::String(s) => s.parse::<u64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_geojson_extension() {
        assert_eq!(
            detect(Path::new("a/b.geojson")).unwrap(),
            InputFormat::GeoJson
        );
        assert_eq!(detect(Path::new("a/b.json")).unwrap(), InputFormat::GeoJson);
        assert_eq!(
            detect(Path::new("A.GEOJSON")).unwrap(),
            InputFormat::GeoJson
        );
    }

    #[test]
    fn detect_csv_extension() {
        assert_eq!(detect(Path::new("cities.csv")).unwrap(), InputFormat::Csv);
    }

    #[test]
    fn detect_unknown_extension_errors() {
        assert!(detect(Path::new("a.shp")).is_err());
        assert!(detect(Path::new("noext")).is_err());
    }

    #[test]
    fn resolve_id_explicit_property_wins() {
        let mut props = BTreeMap::new();
        props.insert("id".to_string(), PropValue::Int(1));
        props.insert("custom".to_string(), PropValue::Int(42));
        assert_eq!(resolve_id(&props, Some("custom")), Some(42));
    }

    #[test]
    fn resolve_id_auto_from_candidates_precedence() {
        let mut props = BTreeMap::new();
        props.insert("gid".to_string(), PropValue::Int(7));
        assert_eq!(resolve_id(&props, None), Some(7));
    }

    #[test]
    fn resolve_id_case_insensitive_objectid() {
        let mut props = BTreeMap::new();
        props.insert("OBJECTID".to_string(), PropValue::Int(99));
        assert_eq!(resolve_id(&props, None), Some(99));
    }

    #[test]
    fn resolve_id_from_string_value() {
        let mut props = BTreeMap::new();
        props.insert("id".to_string(), PropValue::String("13".to_string()));
        assert_eq!(resolve_id(&props, None), Some(13));
    }

    #[test]
    fn resolve_id_none_when_absent() {
        let mut props = BTreeMap::new();
        props.insert("name".to_string(), PropValue::String("x".to_string()));
        assert_eq!(resolve_id(&props, None), None);
    }
}
