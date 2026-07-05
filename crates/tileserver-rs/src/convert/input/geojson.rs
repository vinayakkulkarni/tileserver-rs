//! GeoJSON input streaming via a geozero [`FeatureProcessor`] sink.

use super::{ConvertFeature, Geometry, PropValue, Ring, resolve_id};
use crate::error::{Result, TileServerError};
use geozero::{
    ColumnValue, FeatureProcessor, GeomProcessor, GeozeroDatasource, PropertyProcessor,
    error::GeozeroError, geojson::GeoJson,
};
use std::collections::BTreeMap;

/// Collects geozero feature callbacks into owned [`ConvertFeature`]s. Geometry
/// vertices arrive via [`GeomProcessor::xy`]; nested begin/end callbacks track
/// which ring/part is currently being filled.
#[derive(Default)]
pub struct FeatureSink {
    features: Vec<ConvertFeature>,
    id_property: Option<String>,
    current_props: BTreeMap<String, PropValue>,
    builder: GeometryBuilder,
}

/// Accumulates geometry vertices across geozero's nested part/ring callbacks.
#[derive(Default)]
struct GeometryBuilder {
    points: Vec<(f64, f64)>,
    rings: Vec<Ring>,
    polygons: Vec<Vec<Ring>>,
    kind: GeomKind,
}

/// Which multi-level container the builder is currently populating.
#[derive(Default, Clone, Copy, PartialEq)]
enum GeomKind {
    #[default]
    Point,
    MultiPoint,
    LineString,
    MultiLineString,
    Polygon,
    MultiPolygon,
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

impl GeometryBuilder {
    fn reset(&mut self) {
        self.points.clear();
        self.rings.clear();
        self.polygons.clear();
        self.kind = GeomKind::Point;
    }

    /// Finalize the in-progress geometry into a [`Geometry`], or `None` when no
    /// vertices were collected (null/empty geometry).
    fn finish(&mut self) -> Option<Geometry> {
        let geom = match self.kind {
            GeomKind::Point => self.points.first().copied().map(Geometry::Point),
            GeomKind::MultiPoint => (!self.points.is_empty())
                .then(|| Geometry::MultiPoint(std::mem::take(&mut self.points))),
            GeomKind::LineString => (self.points.len() >= 2)
                .then(|| Geometry::LineString(std::mem::take(&mut self.points))),
            GeomKind::MultiLineString => (!self.rings.is_empty())
                .then(|| Geometry::MultiLineString(std::mem::take(&mut self.rings))),
            GeomKind::Polygon => {
                (!self.rings.is_empty()).then(|| Geometry::Polygon(std::mem::take(&mut self.rings)))
            }
            GeomKind::MultiPolygon => (!self.polygons.is_empty())
                .then(|| Geometry::MultiPolygon(std::mem::take(&mut self.polygons))),
        };
        self.reset();
        geom
    }
}

impl GeomProcessor for FeatureSink {
    fn xy(&mut self, x: f64, y: f64, _idx: usize) -> geozero::error::Result<()> {
        self.builder.points.push((x, y));
        Ok(())
    }

    fn point_begin(&mut self, _idx: usize) -> geozero::error::Result<()> {
        self.builder.kind = GeomKind::Point;
        Ok(())
    }

    fn multipoint_begin(&mut self, _size: usize, _idx: usize) -> geozero::error::Result<()> {
        self.builder.kind = GeomKind::MultiPoint;
        Ok(())
    }

    fn linestring_begin(
        &mut self,
        tagged: bool,
        _size: usize,
        _idx: usize,
    ) -> geozero::error::Result<()> {
        if tagged {
            self.builder.kind = GeomKind::LineString;
        }
        self.builder.points = Vec::new();
        Ok(())
    }

    fn linestring_end(&mut self, tagged: bool, _idx: usize) -> geozero::error::Result<()> {
        if !tagged {
            let ring = std::mem::take(&mut self.builder.points);
            self.builder.rings.push(ring);
        }
        Ok(())
    }

    fn multilinestring_begin(&mut self, _size: usize, _idx: usize) -> geozero::error::Result<()> {
        self.builder.kind = GeomKind::MultiLineString;
        Ok(())
    }

    fn polygon_begin(
        &mut self,
        _tagged: bool,
        _size: usize,
        _idx: usize,
    ) -> geozero::error::Result<()> {
        if self.builder.kind != GeomKind::MultiPolygon {
            self.builder.kind = GeomKind::Polygon;
        }
        self.builder.rings = Vec::new();
        Ok(())
    }

    fn polygon_end(&mut self, _tagged: bool, _idx: usize) -> geozero::error::Result<()> {
        if self.builder.kind == GeomKind::MultiPolygon {
            let rings = std::mem::take(&mut self.builder.rings);
            self.builder.polygons.push(rings);
        }
        Ok(())
    }

    fn multipolygon_begin(&mut self, _size: usize, _idx: usize) -> geozero::error::Result<()> {
        self.builder.kind = GeomKind::MultiPolygon;
        Ok(())
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
fn column_to_prop(value: &ColumnValue) -> Option<PropValue> {
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
