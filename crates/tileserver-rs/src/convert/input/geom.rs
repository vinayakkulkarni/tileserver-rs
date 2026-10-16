//! Shared geometry collector: turns geozero [`GeomProcessor`] callbacks into an
//! owned [`Geometry`]. Reused by the GeoJSON and CSV/WKT readers so the nested
//! part/ring bookkeeping lives in exactly one place.

use super::{Geometry, Ring};
use geozero::GeomProcessor;

/// Accumulates geometry vertices across geozero's nested part/ring callbacks.
#[derive(Default)]
pub struct GeomCollector {
    points: Vec<(f64, f64)>,
    rings: Vec<Ring>,
    polygons: Vec<Vec<Ring>>,
    kind: GeomKind,
}

/// Which multi-level container the collector is currently populating.
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

impl GeomCollector {
    /// Discard any partially-collected geometry so the collector can be reused
    /// for the next feature.
    pub fn reset(&mut self) {
        self.points.clear();
        self.rings.clear();
        self.polygons.clear();
        self.kind = GeomKind::Point;
    }

    /// Finalize the in-progress geometry, or `None` when no vertices were
    /// collected (a null or empty geometry).
    pub fn finish(&mut self) -> Option<Geometry> {
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

impl GeomProcessor for GeomCollector {
    fn xy(&mut self, x: f64, y: f64, _idx: usize) -> geozero::error::Result<()> {
        self.points.push((x, y));
        Ok(())
    }

    fn point_begin(&mut self, _idx: usize) -> geozero::error::Result<()> {
        self.kind = GeomKind::Point;
        Ok(())
    }

    fn multipoint_begin(&mut self, _size: usize, _idx: usize) -> geozero::error::Result<()> {
        self.kind = GeomKind::MultiPoint;
        Ok(())
    }

    fn linestring_begin(
        &mut self,
        tagged: bool,
        _size: usize,
        _idx: usize,
    ) -> geozero::error::Result<()> {
        if tagged {
            self.kind = GeomKind::LineString;
        }
        self.points = Vec::new();
        Ok(())
    }

    fn linestring_end(&mut self, tagged: bool, _idx: usize) -> geozero::error::Result<()> {
        if !tagged {
            let ring = std::mem::take(&mut self.points);
            self.rings.push(ring);
        }
        Ok(())
    }

    fn multilinestring_begin(&mut self, _size: usize, _idx: usize) -> geozero::error::Result<()> {
        self.kind = GeomKind::MultiLineString;
        Ok(())
    }

    fn polygon_begin(
        &mut self,
        _tagged: bool,
        _size: usize,
        _idx: usize,
    ) -> geozero::error::Result<()> {
        if self.kind != GeomKind::MultiPolygon {
            self.kind = GeomKind::Polygon;
        }
        self.rings = Vec::new();
        Ok(())
    }

    fn polygon_end(&mut self, _tagged: bool, _idx: usize) -> geozero::error::Result<()> {
        if self.kind == GeomKind::MultiPolygon {
            let rings = std::mem::take(&mut self.rings);
            self.polygons.push(rings);
        }
        Ok(())
    }

    fn multipolygon_begin(&mut self, _size: usize, _idx: usize) -> geozero::error::Result<()> {
        self.kind = GeomKind::MultiPolygon;
        Ok(())
    }
}
