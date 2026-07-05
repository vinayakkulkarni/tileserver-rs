//! GeoJSON input streaming via a geozero [`FeatureProcessor`] sink.

#[cfg(test)]
use super::Geometry;
use super::geom::GeomCollector;
use super::{ConvertFeature, PropValue, resolve_id};
use crate::error::{Result, TileServerError};
use geozero::{
    ColumnValue, FeatureProcessor, GeomProcessor, GeozeroDatasource, PropertyProcessor,
    error::GeozeroError, geojson::GeoJson,
};
use std::collections::BTreeMap;

/// Collects geozero feature callbacks into owned [`ConvertFeature`]s. Geometry
/// callbacks are delegated to a shared [`GeomCollector`]; property callbacks and
/// feature boundaries are handled here.
#[derive(Default)]
pub struct FeatureSink {
    features: Vec<ConvertFeature>,
    id_property: Option<String>,
    current_props: BTreeMap<String, PropValue>,
    builder: GeomCollector,
}

impl FeatureSink {
    /// Create a sink resolving IDs from `id_property` (or the auto-detect
    /// candidates when `None`).
    #[must_use]
    pub fn new(id_property: Option<String>) -> Self {
        Self {
            id_property,
            ..Self::default()
        }
    }

    /// Consume the sink and return the collected features.
    #[must_use]
    pub fn into_features(self) -> Vec<ConvertFeature> {
        self.features
    }
}

impl GeomProcessor for FeatureSink {
    fn xy(&mut self, x: f64, y: f64, idx: usize) -> geozero::error::Result<()> {
        self.builder.xy(x, y, idx)
    }

    fn point_begin(&mut self, idx: usize) -> geozero::error::Result<()> {
        self.builder.point_begin(idx)
    }

    fn multipoint_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.builder.multipoint_begin(size, idx)
    }

    fn linestring_begin(
        &mut self,
        tagged: bool,
        size: usize,
        idx: usize,
    ) -> geozero::error::Result<()> {
        self.builder.linestring_begin(tagged, size, idx)
    }

    fn linestring_end(&mut self, tagged: bool, idx: usize) -> geozero::error::Result<()> {
        self.builder.linestring_end(tagged, idx)
    }

    fn multilinestring_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.builder.multilinestring_begin(size, idx)
    }

    fn polygon_begin(
        &mut self,
        tagged: bool,
        size: usize,
        idx: usize,
    ) -> geozero::error::Result<()> {
        self.builder.polygon_begin(tagged, size, idx)
    }

    fn polygon_end(&mut self, tagged: bool, idx: usize) -> geozero::error::Result<()> {
        self.builder.polygon_end(tagged, idx)
    }

    fn multipolygon_begin(&mut self, size: usize, idx: usize) -> geozero::error::Result<()> {
        self.builder.multipolygon_begin(size, idx)
    }
}

impl PropertyProcessor for FeatureSink {
    fn property(
        &mut self,
        _idx: usize,
        name: &str,
        value: &ColumnValue,
    ) -> geozero::error::Result<bool> {
        if let Some(v) = column_to_prop(value) {
            self.current_props.insert(name.to_string(), v);
        }
        Ok(false)
    }
}

impl FeatureProcessor for FeatureSink {
    fn feature_begin(&mut self, _idx: u64) -> geozero::error::Result<()> {
        self.current_props.clear();
        self.builder.reset();
        Ok(())
    }

    fn feature_end(&mut self, _idx: u64) -> geozero::error::Result<()> {
        if let Some(geometry) = self.builder.finish() {
            let properties = std::mem::take(&mut self.current_props);
            let id = resolve_id(&properties, self.id_property.as_deref());
            self.features.push(ConvertFeature {
                geometry,
                properties,
                id,
            });
        }
        Ok(())
    }
}

/// Map a geozero [`ColumnValue`] to a [`PropValue`], dropping unsupported types.
pub(crate) fn column_to_prop(value: &ColumnValue) -> Option<PropValue> {
    match value {
        ColumnValue::Bool(b) => Some(PropValue::Bool(*b)),
        ColumnValue::Byte(v) => Some(PropValue::Int(i64::from(*v))),
        ColumnValue::UByte(v) => Some(PropValue::Int(i64::from(*v))),
        ColumnValue::Short(v) => Some(PropValue::Int(i64::from(*v))),
        ColumnValue::UShort(v) => Some(PropValue::Int(i64::from(*v))),
        ColumnValue::Int(v) => Some(PropValue::Int(i64::from(*v))),
        ColumnValue::UInt(v) => Some(PropValue::Int(i64::from(*v))),
        ColumnValue::Long(v) => Some(PropValue::Int(*v)),
        ColumnValue::ULong(v) => i64::try_from(*v).ok().map(PropValue::Int),
        ColumnValue::Float(v) => Some(PropValue::Float(f64::from(*v))),
        ColumnValue::Double(v) => Some(PropValue::Float(*v)),
        ColumnValue::String(s) | ColumnValue::Json(s) | ColumnValue::DateTime(s) => {
            Some(PropValue::String((*s).to_string()))
        }
        ColumnValue::Binary(_) => None,
    }
}

/// Parse a GeoJSON string into owned features.
///
/// # Errors
///
/// Returns [`TileServerError::ConvertError`] when the GeoJSON is malformed.
pub fn read_geojson(text: &str, id_property: Option<String>) -> Result<Vec<ConvertFeature>> {
    let mut sink = FeatureSink::new(id_property);
    let mut source = GeoJson(text);
    source
        .process(&mut sink)
        .map_err(|e: GeozeroError| TileServerError::ConvertError(format!("geojson parse: {e}")))?;
    Ok(sink.into_features())
}

#[cfg(test)]
mod tests {
    use super::*;

    const POINTS: &str = r#"{
        "type": "FeatureCollection",
        "features": [
            {"type":"Feature","geometry":{"type":"Point","coordinates":[8.5,47.3]},
             "properties":{"name":"A","id":1}},
            {"type":"Feature","geometry":{"type":"Point","coordinates":[8.6,47.4]},
             "properties":{"name":"B"}}
        ]
    }"#;

    #[test]
    fn reads_point_feature_collection() {
        let feats = read_geojson(POINTS, None).unwrap();
        assert_eq!(feats.len(), 2);
        assert_eq!(feats[0].geometry, Geometry::Point((8.5, 47.3)));
    }

    #[test]
    fn reads_polygon_feature_collection() {
        let poly = r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":{"type":"Polygon","coordinates":[
              [[0,0],[0,1],[1,1],[1,0],[0,0]]]},"properties":{}}
        ]}"#;
        let feats = read_geojson(poly, None).unwrap();
        assert_eq!(feats.len(), 1);
        match &feats[0].geometry {
            Geometry::Polygon(rings) => {
                assert_eq!(rings.len(), 1);
                assert_eq!(rings[0].len(), 5);
            }
            other => panic!("expected polygon, got {other:?}"),
        }
    }

    #[test]
    fn skips_null_geometries() {
        let src = r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":null,"properties":{"name":"x"}},
            {"type":"Feature","geometry":{"type":"Point","coordinates":[1,2]},"properties":{}}
        ]}"#;
        let feats = read_geojson(src, None).unwrap();
        assert_eq!(feats.len(), 1);
    }

    #[test]
    fn reads_multipolygon() {
        let src = r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":{"type":"MultiPolygon","coordinates":[
              [[[0,0],[0,1],[1,1],[0,0]]],
              [[[2,2],[2,3],[3,3],[2,2]]]
            ]},"properties":{}}
        ]}"#;
        let feats = read_geojson(src, None).unwrap();
        match &feats[0].geometry {
            Geometry::MultiPolygon(polys) => assert_eq!(polys.len(), 2),
            other => panic!("expected multipolygon, got {other:?}"),
        }
    }

    #[test]
    fn auto_id_from_id_field() {
        let feats = read_geojson(POINTS, None).unwrap();
        assert_eq!(feats[0].id, Some(1));
    }

    #[test]
    fn auto_id_from_gid_field() {
        let src = r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":{"type":"Point","coordinates":[1,2]},
             "properties":{"gid":55}}
        ]}"#;
        let feats = read_geojson(src, None).unwrap();
        assert_eq!(feats[0].id, Some(55));
    }

    #[test]
    fn no_id_when_property_missing() {
        let feats = read_geojson(POINTS, None).unwrap();
        assert_eq!(feats[1].id, None);
    }

    #[test]
    fn handles_utf8_property_values() {
        let src = r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","geometry":{"type":"Point","coordinates":[1,2]},
             "properties":{"name":"Zürich ⛰"}}
        ]}"#;
        let feats = read_geojson(src, None).unwrap();
        assert_eq!(
            feats[0].properties.get("name"),
            Some(&PropValue::String("Zürich ⛰".to_string()))
        );
    }

    #[test]
    fn malformed_geojson_errors() {
        assert!(read_geojson("{not json", None).is_err());
    }
}
