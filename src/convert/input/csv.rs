use anyhow::{bail, Context, Result};
use geo::Geometry;
use std::path::Path;

use super::Feature;

/// Column-name aliases recognised as the longitude field (case-insensitive).
const LON_ALIASES: &[&str] = &["lon", "longitude", "x", "lng"];
/// Column-name aliases recognised as the latitude field (case-insensitive).
const LAT_ALIASES: &[&str] = &["lat", "latitude", "y"];

/// Read a CSV file and return one Point feature per row.
///
/// The file must have a header row. One column must be recognisable as longitude
/// and one as latitude (see `LON_ALIASES` / `LAT_ALIASES`). All other columns
/// are included as feature properties, with automatic type inference: values
/// that parse as `f64` become JSON numbers; everything else becomes a JSON string.
pub fn read(path: &Path) -> Result<Vec<Feature>> {
    let mut reader = csv::Reader::from_path(path)
        .with_context(|| format!("Failed to open CSV file: {}", path.display()))?;

    let headers = reader
        .headers()
        .with_context(|| format!("Failed to read CSV headers: {}", path.display()))?
        .clone();

    // Locate the lon/lat columns by matching aliases.
    let lon_idx = find_column(&headers, LON_ALIASES).with_context(|| {
        format!(
            "CSV file has no recognisable longitude column (tried: {}): {}",
            LON_ALIASES.join(", "),
            path.display()
        )
    })?;
    let lat_idx = find_column(&headers, LAT_ALIASES).with_context(|| {
        format!(
            "CSV file has no recognisable latitude column (tried: {}): {}",
            LAT_ALIASES.join(", "),
            path.display()
        )
    })?;

    // Build the list of property column indices (everything except lon/lat).
    let prop_cols: Vec<(usize, &str)> = headers
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != lon_idx && *i != lat_idx)
        .collect();

    let mut features = Vec::new();

    for (row_num, result) in reader.records().enumerate() {
        let record = result.with_context(|| {
            format!(
                "Failed to parse CSV row {} in: {}",
                row_num + 2, // +2: 1-based + header row
                path.display()
            )
        })?;

        let lon: f64 = record
            .get(lon_idx)
            .unwrap_or("")
            .trim()
            .parse()
            .with_context(|| {
                format!(
                    "Invalid longitude value at row {} in: {}",
                    row_num + 2,
                    path.display()
                )
            })?;

        let lat: f64 = record
            .get(lat_idx)
            .unwrap_or("")
            .trim()
            .parse()
            .with_context(|| {
                format!(
                    "Invalid latitude value at row {} in: {}",
                    row_num + 2,
                    path.display()
                )
            })?;

        if !(-180.0..=180.0).contains(&lon) || !(-90.0..=90.0).contains(&lat) {
            bail!(
                "Coordinates out of range at row {} (lon={lon}, lat={lat}) in: {}",
                row_num + 2,
                path.display()
            );
        }

        let mut properties = serde_json::Map::new();
        for (idx, name) in &prop_cols {
            let raw = record.get(*idx).unwrap_or("").trim();
            let value = infer_value(raw);
            properties.insert((*name).to_owned(), value);
        }

        features.push(Feature {
            geometry: Geometry::Point(geo::Point::new(lon, lat)),
            properties,
        });
    }

    Ok(features)
}

/// Find the first column whose lowercased name matches any of `aliases`.
fn find_column(headers: &csv::StringRecord, aliases: &[&str]) -> Option<usize> {
    headers.iter().position(|h| {
        let lower = h.to_ascii_lowercase();
        aliases.iter().any(|a| *a == lower)
    })
}

/// Infer a JSON value from a raw string: try `f64`, fall back to string.
/// Empty strings become `null`.
fn infer_value(raw: &str) -> serde_json::Value {
    if raw.is_empty() {
        return serde_json::Value::Null;
    }
    if let Ok(n) = raw.parse::<f64>() {
        if let Some(v) = serde_json::Number::from_f64(n) {
            return serde_json::Value::Number(v);
        }
    }
    serde_json::Value::String(raw.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::Builder;

    fn write_csv(content: &str) -> tempfile::NamedTempFile {
        let mut f = Builder::new()
            .suffix(".csv")
            .tempfile()
            .expect("create tempfile");
        f.write_all(content.as_bytes()).expect("write csv");
        f
    }

    #[test]
    fn reads_basic_lon_lat_columns() {
        let f = write_csv("lon,lat,name\n13.4,52.5,Berlin\n2.35,48.85,Paris\n");
        let features = read(f.path()).unwrap();
        assert_eq!(features.len(), 2);
        assert!(matches!(features[0].geometry, geo::Geometry::Point(_)));
        assert_eq!(features[0].properties["name"], serde_json::json!("Berlin"));
    }

    #[test]
    fn accepts_longitude_latitude_aliases() {
        let f = write_csv("longitude,latitude\n10.0,50.0\n");
        let features = read(f.path()).unwrap();
        assert_eq!(features.len(), 1);
        if let geo::Geometry::Point(p) = &features[0].geometry {
            assert!((p.x() - 10.0).abs() < 1e-10);
            assert!((p.y() - 50.0).abs() < 1e-10);
        } else {
            panic!("expected Point");
        }
    }

    #[test]
    fn accepts_x_y_aliases() {
        let f = write_csv("x,y,value\n5.0,45.0,42\n");
        let features = read(f.path()).unwrap();
        assert_eq!(features.len(), 1);
        assert_eq!(features[0].properties["value"], serde_json::json!(42.0));
    }

    #[test]
    fn infers_numeric_properties() {
        let f = write_csv("lon,lat,pop,name\n0.0,0.0,1000000,City\n");
        let features = read(f.path()).unwrap();
        let props = &features[0].properties;
        // Numeric strings become JSON numbers
        assert!(props["pop"].is_number());
        assert!(props["name"].is_string());
    }

    #[test]
    fn empty_property_becomes_null() {
        let f = write_csv("lon,lat,note\n0.0,0.0,\n");
        let features = read(f.path()).unwrap();
        assert_eq!(features[0].properties["note"], serde_json::Value::Null);
    }

    #[test]
    fn returns_error_for_missing_lon_column() {
        let f = write_csv("x_coord,lat\n10.0,50.0\n");
        let result = read(f.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("longitude"));
    }

    #[test]
    fn returns_error_for_missing_lat_column() {
        let f = write_csv("lon,y_coord\n10.0,50.0\n");
        let result = read(f.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("latitude"));
    }

    #[test]
    fn returns_error_for_invalid_coordinate() {
        let f = write_csv("lon,lat\nnot_a_number,50.0\n");
        let result = read(f.path());
        assert!(result.is_err());
    }

    #[test]
    fn returns_error_for_out_of_range_coordinates() {
        let f = write_csv("lon,lat\n999.0,50.0\n");
        let result = read(f.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of range"));
    }

    #[test]
    fn dispatches_via_read_features() {
        use super::super::read_features;
        let f = write_csv("lon,lat,name\n13.4,52.5,Berlin\n");
        let features = read_features(f.path()).unwrap();
        assert_eq!(features.len(), 1);
    }

    #[test]
    fn case_insensitive_column_names() {
        let f = write_csv("LON,LAT,Name\n1.0,2.0,Test\n");
        let features = read(f.path()).unwrap();
        assert_eq!(features.len(), 1);
    }
}
