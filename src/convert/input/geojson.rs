use anyhow::{Context, Result};
use geojson::{Feature as GeoFeature, FeatureCollection, GeoJson};
use std::path::Path;

use super::Feature;

/// Read a GeoJSON file and return all features with their geometries and properties.
pub fn read(path: &Path) -> Result<Vec<Feature>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read GeoJSON file: {}", path.display()))?;

    let geojson: GeoJson = content
        .parse()
        .with_context(|| format!("Failed to parse GeoJSON: {}", path.display()))?;

    let collection = match geojson {
        GeoJson::FeatureCollection(fc) => fc,
        GeoJson::Feature(f) => FeatureCollection {
            bbox: None,
            features: vec![f],
            foreign_members: None,
        },
        GeoJson::Geometry(g) => FeatureCollection {
            bbox: None,
            features: vec![GeoFeature {
                bbox: None,
                geometry: Some(g),
                id: None,
                properties: None,
                foreign_members: None,
            }],
            foreign_members: None,
        },
    };

    let mut features = Vec::with_capacity(collection.features.len());
    for geo_feature in collection.features {
        if let Some(geom) = geo_feature.geometry {
            let geometry: geo::Geometry<f64> = geom
                .try_into()
                .context("Failed to convert GeoJSON geometry to geo_types")?;
            let properties = geo_feature.properties.unwrap_or_default();
            features.push(Feature {
                geometry,
                properties,
            });
        }
    }

    Ok(features)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::Builder;

    fn write_geojson(content: &str) -> tempfile::NamedTempFile {
        let mut f = Builder::new()
            .suffix(".geojson")
            .tempfile()
            .expect("create tempfile");
        f.write_all(content.as_bytes()).expect("write geojson");
        f
    }

    #[test]
    fn reads_single_point_from_feature_collection() {
        let f = write_geojson(
            r#"{"type":"FeatureCollection","features":[
                {"type":"Feature","geometry":{"type":"Point","coordinates":[13.4,52.5]},"properties":{"name":"Berlin"}}
            ]}"#,
        );
        let features = read(f.path()).unwrap();
        assert_eq!(features.len(), 1);
        assert!(matches!(features[0].geometry, geo::Geometry::Point(_)));
    }

    #[test]
    fn reads_properties_correctly() {
        let f = write_geojson(
            r#"{"type":"Feature","geometry":{"type":"Point","coordinates":[0.0,0.0]},
               "properties":{"string":"hello","number":42,"bool":true,"null_val":null}}"#,
        );
        let features = read(f.path()).unwrap();
        assert_eq!(features.len(), 1);
        let props = &features[0].properties;
        assert_eq!(props["string"], serde_json::json!("hello"));
        assert_eq!(props["number"], serde_json::json!(42));
        assert_eq!(props["bool"], serde_json::json!(true));
        assert_eq!(props["null_val"], serde_json::Value::Null);
    }

    #[test]
    fn wraps_bare_geometry_in_feature() {
        let f = write_geojson(r#"{"type":"Point","coordinates":[5.0,45.0]}"#);
        let features = read(f.path()).unwrap();
        assert_eq!(features.len(), 1);
        assert!(matches!(features[0].geometry, geo::Geometry::Point(_)));
    }

    #[test]
    fn wraps_bare_feature_without_collection() {
        let f = write_geojson(
            r#"{"type":"Feature","geometry":{"type":"Point","coordinates":[0.0,0.0]},"properties":{}}"#,
        );
        let features = read(f.path()).unwrap();
        assert_eq!(features.len(), 1);
    }

    #[test]
    fn skips_features_without_geometry() {
        let f = write_geojson(
            r#"{"type":"FeatureCollection","features":[
                {"type":"Feature","geometry":null,"properties":{}},
                {"type":"Feature","geometry":{"type":"Point","coordinates":[1.0,2.0]},"properties":{}}
            ]}"#,
        );
        let features = read(f.path()).unwrap();
        assert_eq!(features.len(), 1);
    }

    #[test]
    fn reads_multiple_geometry_types() {
        let f = write_geojson(
            r#"{"type":"FeatureCollection","features":[
                {"type":"Feature","geometry":{"type":"Point","coordinates":[0.0,0.0]},"properties":{}},
                {"type":"Feature","geometry":{"type":"LineString","coordinates":[[0,0],[1,1]]},"properties":{}},
                {"type":"Feature","geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,1],[0,0]]]},"properties":{}}
            ]}"#,
        );
        let features = read(f.path()).unwrap();
        assert_eq!(features.len(), 3);
    }

    #[test]
    fn returns_error_for_invalid_json() {
        let f = write_geojson("this is not json {{{");
        assert!(read(f.path()).is_err());
    }

    #[test]
    fn returns_error_for_unsupported_extension() {
        // Test via read_features dispatch (wrong extension)
        use super::super::read_features;
        let mut f = Builder::new()
            .suffix(".csv")
            .tempfile()
            .expect("create tempfile");
        f.write_all(b"lon,lat\n13.4,52.5").expect("write");
        let result = read_features(f.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }
}
