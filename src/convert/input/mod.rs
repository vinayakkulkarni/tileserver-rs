use anyhow::Result;
use geo::Geometry;
use serde_json::{Map, Value};
use std::path::Path;

pub mod csv;
pub mod geojson;

/// A single geospatial feature with WGS-84 geometry and JSON properties.
#[derive(Debug, Clone)]
pub struct Feature {
    pub geometry: Geometry<f64>,
    pub properties: Map<String, Value>,
}

/// Supported input formats (for future extension).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Csv,
    GeoJson,
}

impl InputFormat {
    /// Detect format from file extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "csv" => Some(Self::Csv),
            "geojson" | "json" => Some(Self::GeoJson),
            _ => None,
        }
    }
}

/// Read all features from a file, dispatching by format.
pub fn read_features(path: &Path) -> Result<Vec<Feature>> {
    let fmt = InputFormat::from_path(path)
        .ok_or_else(|| anyhow::anyhow!("Unsupported file extension: {}", path.display()))?;

    match fmt {
        InputFormat::Csv => csv::read(path),
        InputFormat::GeoJson => geojson::read(path),
    }
}
