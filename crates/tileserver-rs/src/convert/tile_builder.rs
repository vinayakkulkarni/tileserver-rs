//! Web-Mercator tile partitioning and MVT encoding.
//!
//! The projection, tile-coordinate, bbox-overlap, tolerance, and Hilbert-order
//! helpers are pure functions with exhaustive unit tests. The [`TileBuilder`]
//! shell threads features through them and delegates MVT command encoding to
//! geozero's [`MvtWriter`], keeping every function well under the CRAP ceiling.

use super::input::{ConvertFeature, Geometry, PropValue, Ring};
use super::simplify::dp_simplify;
use crate::error::{Result, TileServerError};
use geozero::mvt::{Message, MvtWriter, Tile};
use geozero::{ColumnValue, FeatureProcessor, GeomProcessor, PropertyProcessor};
use pmtiles::{TileCoord, TileId};
use std::collections::BTreeMap;

/// MVT tile extent (integer coordinate resolution per tile).
pub const EXTENT: u32 = 4096;

/// Earth radius used by the spherical Web-Mercator projection (EPSG:3857).
const EARTH_RADIUS: f64 = 6_378_137.0;

/// Half the Web-Mercator planar extent in meters (`π · EARTH_RADIUS`).
const MERCATOR_MAX: f64 = std::f64::consts::PI * EARTH_RADIUS;

/// Project a WGS84 longitude to a Web-Mercator X meter coordinate.
#[must_use]
pub fn lon_to_mercator_x(lon: f64) -> f64 {
    lon.to_radians() * EARTH_RADIUS
}

/// Project a WGS84 latitude to a Web-Mercator Y meter coordinate. Latitude is
/// clamped to the Web-Mercator limit (±85.051129°) to avoid infinities.
#[must_use]
pub fn lat_to_mercator_y(lat: f64) -> f64 {
    let clamped = lat.clamp(-85.051_128_779_806_59, 85.051_128_779_806_59);
    let rad = clamped.to_radians();
    EARTH_RADIUS * (std::f64::consts::FRAC_PI_4 + rad / 2.0).tan().ln()
}

/// Number of tiles per axis at zoom `z`.
#[must_use]
pub fn tiles_per_axis(z: u8) -> u32 {
    1u32 << z
}

/// Slippy-map tile X column for a longitude at zoom `z`.
#[must_use]
pub fn lon_to_tile_x(lon: f64, z: u8) -> u32 {
    let n = f64::from(tiles_per_axis(z));
    let x = (lon + 180.0) / 360.0 * n;
    (x.floor() as i64).clamp(0, i64::from(tiles_per_axis(z)) - 1) as u32
}

/// Slippy-map tile Y row for a latitude at zoom `z`.
#[must_use]
pub fn lat_to_tile_y(lat: f64, z: u8) -> u32 {
    let n = f64::from(tiles_per_axis(z));
    let rad = lat.to_radians();
    let y = (1.0 - rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n;
    (y.floor() as i64).clamp(0, i64::from(tiles_per_axis(z)) - 1) as u32
}

/// Web-Mercator bounds `(left, bottom, right, top)` in meters for tile
/// `(z, x, y)`.
#[must_use]
pub fn tile_bounds_mercator(z: u8, x: u32, y: u32) -> (f64, f64, f64, f64) {
    let n = f64::from(tiles_per_axis(z));
    let span = 2.0 * MERCATOR_MAX / n;
    let left = -MERCATOR_MAX + f64::from(x) * span;
    let right = left + span;
    let top = MERCATOR_MAX - f64::from(y) * span;
    let bottom = top - span;
    (left, bottom, right, top)
}

/// WGS84 bounding box `(min_lon, min_lat, max_lon, max_lat)` of a geometry.
#[must_use]
pub fn geometry_bbox(geom: &Geometry) -> (f64, f64, f64, f64) {
    let mut bbox = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for_each_vertex(geom, &mut |(lon, lat)| {
        bbox.0 = bbox.0.min(lon);
        bbox.1 = bbox.1.min(lat);
        bbox.2 = bbox.2.max(lon);
        bbox.3 = bbox.3.max(lat);
    });
    bbox
}

/// Inclusive tile range `(tx0, ty0, tx1, ty1)` covering a WGS84 bbox at zoom
/// `z`. Note `ty0` is the northern (smaller-Y) row.
#[must_use]
pub fn tile_range_for_bbox(bbox: (f64, f64, f64, f64), z: u8) -> (u32, u32, u32, u32) {
    let tx0 = lon_to_tile_x(bbox.0, z);
    let tx1 = lon_to_tile_x(bbox.2, z);
    let ty0 = lat_to_tile_y(bbox.3, z);
    let ty1 = lat_to_tile_y(bbox.1, z);
    (tx0.min(tx1), ty0.min(ty1), tx0.max(tx1), ty0.max(ty1))
}

/// Douglas-Peucker tolerance (in Web-Mercator meters) for zoom `z`, following
/// tippecanoe's halving cadence with a floor. An explicit `override_tol` wins.
#[must_use]
pub fn tolerance_for_zoom(z: u8, min_zoom: u8, override_tol: Option<f64>) -> f64 {
    if let Some(t) = override_tol {
        return t;
    }
    let steps = z.saturating_sub(min_zoom);
    let base = 4.0 / 2.0_f64.powi(i32::from(steps));
    base.max(0.5)
}

/// The Hilbert-curve tile id for `(z, x, y)`, or `None` for out-of-range coords.
#[must_use]
pub fn hilbert_id(z: u8, x: u32, y: u32) -> Option<u64> {
    let coord = TileCoord::new(z, x, y).ok()?;
    Some(TileId::from(coord).value())
}

/// Visit every WGS84 vertex of a geometry.
fn for_each_vertex(geom: &Geometry, f: &mut impl FnMut((f64, f64))) {
    match geom {
        Geometry::Point(p) => f(*p),
        Geometry::MultiPoint(ps) => ps.iter().copied().for_each(f),
        Geometry::LineString(r) => r.iter().copied().for_each(&mut *f),
        Geometry::MultiLineString(rs) | Geometry::Polygon(rs) => {
            rs.iter().flatten().copied().for_each(f);
        }
        Geometry::MultiPolygon(polys) => polys.iter().flatten().flatten().copied().for_each(f),
    }
}

/// Project a ring's vertices to Web-Mercator meters and simplify it.
fn project_and_simplify(ring: &Ring, tolerance: f64) -> Vec<(f64, f64)> {
    let projected: Vec<(f64, f64)> = ring
        .iter()
        .map(|&(lon, lat)| (lon_to_mercator_x(lon), lat_to_mercator_y(lat)))
        .collect();
    dp_simplify(&projected, tolerance)
}

/// A feature whose geometry has been projected to Web-Mercator meters and
/// simplified, ready for MVT encoding at a specific zoom.
struct ProjectedFeature {
    geometry: Geometry,
    properties: BTreeMap<String, PropValue>,
    id: Option<u64>,
}

/// Project and simplify a feature's geometry into Web-Mercator meters.
fn project_feature(feature: &ConvertFeature, tolerance: f64) -> ProjectedFeature {
    let geometry = match &feature.geometry {
        Geometry::Point((lon, lat)) => {
            Geometry::Point((lon_to_mercator_x(*lon), lat_to_mercator_y(*lat)))
        }
        Geometry::MultiPoint(ps) => Geometry::MultiPoint(
            ps.iter()
                .map(|&(lon, lat)| (lon_to_mercator_x(lon), lat_to_mercator_y(lat)))
                .collect(),
        ),
        Geometry::LineString(r) => Geometry::LineString(project_and_simplify(r, tolerance)),
        Geometry::MultiLineString(rs) => Geometry::MultiLineString(
            rs.iter()
                .map(|r| project_and_simplify(r, tolerance))
                .collect(),
        ),
        Geometry::Polygon(rs) => Geometry::Polygon(
            rs.iter()
                .map(|r| project_and_simplify(r, tolerance))
                .collect(),
        ),
        Geometry::MultiPolygon(polys) => Geometry::MultiPolygon(
            polys
                .iter()
                .map(|rings| {
                    rings
                        .iter()
                        .map(|r| project_and_simplify(r, tolerance))
                        .collect()
                })
                .collect(),
        ),
    };
    ProjectedFeature {
        geometry,
        properties: feature.properties.clone(),
        id: feature.id,
    }
}

/// Emit a projected geometry into an [`MvtWriter`] via its geom callbacks.
fn emit_geometry(writer: &mut MvtWriter, geom: &Geometry) -> geozero::error::Result<()> {
    writer.geometry_begin()?;
    match geom {
        Geometry::Point((x, y)) => {
            writer.point_begin(0)?;
            writer.xy(*x, *y, 0)?;
            writer.point_end(0)?;
        }
        Geometry::MultiPoint(ps) => {
            writer.multipoint_begin(ps.len(), 0)?;
            for (i, (x, y)) in ps.iter().enumerate() {
                writer.xy(*x, *y, i)?;
            }
            writer.multipoint_end(0)?;
        }
        Geometry::LineString(r) => emit_linestring(writer, r, true)?,
        Geometry::MultiLineString(rs) => {
            writer.multilinestring_begin(rs.len(), 0)?;
            for r in rs {
                emit_linestring(writer, r, false)?;
            }
            writer.multilinestring_end(0)?;
        }
        Geometry::Polygon(rings) => emit_polygon(writer, rings)?,
        Geometry::MultiPolygon(polys) => {
            writer.multipolygon_begin(polys.len(), 0)?;
            for rings in polys {
                emit_polygon(writer, rings)?;
            }
            writer.multipolygon_end(0)?;
        }
    }
    writer.geometry_end()
}

/// Emit one line as either a tagged linestring or an untagged ring.
fn emit_linestring(
    writer: &mut MvtWriter,
    ring: &Ring,
    tagged: bool,
) -> geozero::error::Result<()> {
    writer.linestring_begin(tagged, ring.len(), 0)?;
    for (i, (x, y)) in ring.iter().enumerate() {
        writer.xy(*x, *y, i)?;
    }
    writer.linestring_end(tagged, 0)
}

/// Emit a polygon (exterior ring + holes) into the writer.
fn emit_polygon(writer: &mut MvtWriter, rings: &[Ring]) -> geozero::error::Result<()> {
    writer.polygon_begin(true, rings.len(), 0)?;
    for ring in rings {
        emit_linestring(writer, ring, false)?;
    }
    writer.polygon_end(true, 0)
}

/// Encode one property value into the writer's tag table.
fn emit_property(
    writer: &mut MvtWriter,
    idx: usize,
    name: &str,
    value: &PropValue,
) -> geozero::error::Result<()> {
    let column = match value {
        PropValue::String(s) => ColumnValue::String(s),
        PropValue::Int(i) => ColumnValue::Long(*i),
        PropValue::Float(f) => ColumnValue::Double(*f),
        PropValue::Bool(b) => ColumnValue::Bool(*b),
    };
    writer.property(idx, name, &column)?;
    Ok(())
}

/// Whether a property key survives the include/exclude filters.
fn property_included(name: &str, include: &[String], exclude: &[String]) -> bool {
    if !include.is_empty() {
        return include.iter().any(|k| k == name);
    }
    !exclude.iter().any(|k| k == name)
}

/// Options controlling how features are turned into tiles.
#[derive(Debug, Clone)]
pub struct TileOptions {
    /// Inclusive minimum zoom.
    pub min_zoom: u8,
    /// Inclusive maximum zoom.
    pub max_zoom: u8,
    /// Output layer name.
    pub layer_name: String,
    /// Optional Douglas-Peucker tolerance override (Web-Mercator meters).
    pub simplification: Option<f64>,
    /// Property whitelist (empty = allow all).
    pub include_properties: Vec<String>,
    /// Property blacklist (ignored when the whitelist is non-empty).
    pub exclude_properties: Vec<String>,
    /// Drop features once a tile exceeds [`DENSITY_LIMIT`].
    pub drop_densest: bool,
}

/// Per-tile feature cap enforced when `drop_densest` is set.
pub const DENSITY_LIMIT: usize = 1_000;

/// Partitions features into Web-Mercator tiles and encodes them as MVT.
pub struct TileBuilder {
    options: TileOptions,
    tiles: BTreeMap<(u8, u32, u32), Vec<ConvertFeature>>,
}

impl TileBuilder {
    /// Create a builder with the given options.
    #[must_use]
    pub fn new(options: TileOptions) -> Self {
        Self {
            options,
            tiles: BTreeMap::new(),
        }
    }

    /// Assign a feature to every tile it overlaps across the zoom range.
    pub fn add_feature(&mut self, feature: ConvertFeature) {
        let bbox = geometry_bbox(&feature.geometry);
        for z in self.options.min_zoom..=self.options.max_zoom {
            let (tx0, ty0, tx1, ty1) = tile_range_for_bbox(bbox, z);
            for tx in tx0..=tx1 {
                for ty in ty0..=ty1 {
                    let bucket = self.tiles.entry((z, tx, ty)).or_default();
                    if self.options.drop_densest && bucket.len() >= DENSITY_LIMIT {
                        continue;
                    }
                    bucket.push(feature.clone());
                }
            }
        }
    }

    /// Encode all accumulated tiles, returning them in Hilbert (`TileId`) order.
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::ConvertError`] when MVT encoding fails.
    pub fn finish(self) -> Result<Vec<(TileCoord, Vec<u8>)>> {
        let mut encoded: Vec<(u64, TileCoord, Vec<u8>)> = Vec::new();
        for ((z, x, y), features) in &self.tiles {
            let Some(coord) = TileCoord::new(*z, *x, *y).ok() else {
                tracing::warn!(z, x, y, "skipping invalid tile coordinate");
                continue;
            };
            let bytes = self.encode_tile(*z, *x, *y, features)?;
            if bytes.is_empty() {
                continue;
            }
            encoded.push((TileId::from(coord).value(), coord, bytes));
        }
        encoded.sort_by_key(|(id, _, _)| *id);
        Ok(encoded.into_iter().map(|(_, c, b)| (c, b)).collect())
    }

    /// Encode a single tile's features into MVT bytes.
    fn encode_tile(&self, z: u8, x: u32, y: u32, features: &[ConvertFeature]) -> Result<Vec<u8>> {
        let (left, bottom, right, top) = tile_bounds_mercator(z, x, y);
        let tolerance = tolerance_for_zoom(z, self.options.min_zoom, self.options.simplification);
        let mut writer = MvtWriter::new(EXTENT, left, bottom, right, top)
            .map_err(|e| TileServerError::ConvertError(format!("mvt writer: {e}")))?;

        let mut ids: Vec<Option<u64>> = Vec::with_capacity(features.len());
        for (idx, feature) in features.iter().enumerate() {
            let projected = project_feature(feature, tolerance);
            ids.push(projected.id);
            self.write_feature(&mut writer, idx as u64, &projected)?;
        }

        let mut layer = writer.layer(&self.options.layer_name);
        if layer.features.is_empty() {
            return Ok(Vec::new());
        }
        for (feat, id) in layer.features.iter_mut().zip(ids) {
            feat.id = id;
        }
        let tile = Tile {
            layers: vec![layer],
        };
        Ok(tile.encode_to_vec())
    }

    /// Write one projected feature (geometry + filtered properties) to the tile.
    fn write_feature(
        &self,
        writer: &mut MvtWriter,
        idx: u64,
        feature: &ProjectedFeature,
    ) -> Result<()> {
        writer.feature_begin(idx).map_err(mvt_err)?;
        emit_geometry(writer, &feature.geometry).map_err(mvt_err)?;
        let mut prop_idx = 0;
        for (name, value) in &feature.properties {
            if property_included(
                name,
                &self.options.include_properties,
                &self.options.exclude_properties,
            ) {
                emit_property(writer, prop_idx, name, value).map_err(mvt_err)?;
                prop_idx += 1;
            }
        }
        writer.feature_end(idx).map_err(mvt_err)?;
        Ok(())
    }
}

/// Wrap a geozero MVT error as a [`TileServerError`].
fn mvt_err(e: geozero::error::GeozeroError) -> TileServerError {
    TileServerError::ConvertError(format!("mvt encode: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use geozero::mvt::tile;

    fn point(lon: f64, lat: f64) -> ConvertFeature {
        ConvertFeature {
            geometry: Geometry::Point((lon, lat)),
            properties: BTreeMap::new(),
            id: None,
        }
    }

    fn opts() -> TileOptions {
        TileOptions {
            min_zoom: 0,
            max_zoom: 14,
            layer_name: "test".to_string(),
            simplification: None,
            include_properties: Vec::new(),
            exclude_properties: Vec::new(),
            drop_densest: true,
        }
    }

    #[test]
    fn lon_lat_to_tile_xy_z0() {
        assert_eq!(lon_to_tile_x(0.0, 0), 0);
        assert_eq!(lat_to_tile_y(0.0, 0), 0);
    }

    #[test]
    fn lon_lat_to_tile_xy_z14_zurich() {
        // Zurich ~ (8.54, 47.37). Reference slippy values at z14.
        assert_eq!(lon_to_tile_x(8.54, 14), 8580);
        assert_eq!(lat_to_tile_y(47.37, 14), 5737);
    }

    #[test]
    fn tile_bounds_z0_is_full_mercator() {
        let (left, bottom, right, top) = tile_bounds_mercator(0, 0, 0);
        assert!((left + MERCATOR_MAX).abs() < 1.0);
        assert!((right - MERCATOR_MAX).abs() < 1.0);
        assert!((top - MERCATOR_MAX).abs() < 1.0);
        assert!((bottom + MERCATOR_MAX).abs() < 1.0);
    }

    #[test]
    fn mercator_projection_origin_is_zero() {
        assert!(lon_to_mercator_x(0.0).abs() < 1e-6);
        assert!(lat_to_mercator_y(0.0).abs() < 1e-6);
    }

    #[test]
    fn mercator_latitude_clamped_at_poles() {
        let north = lat_to_mercator_y(89.0);
        let clamp = lat_to_mercator_y(85.051_128_779_806_59);
        assert!((north - clamp).abs() < 1.0);
        assert!(north.is_finite());
    }

    #[test]
    fn geometry_bbox_of_points() {
        let geom = Geometry::MultiPoint(vec![(0.0, 0.0), (10.0, 5.0), (-3.0, 8.0)]);
        assert_eq!(geometry_bbox(&geom), (-3.0, 0.0, 10.0, 8.0));
    }

    #[test]
    fn tolerance_scales_per_zoom() {
        assert_eq!(tolerance_for_zoom(0, 0, None), 4.0);
        assert_eq!(tolerance_for_zoom(1, 0, None), 2.0);
        assert_eq!(tolerance_for_zoom(2, 0, None), 1.0);
        assert_eq!(tolerance_for_zoom(10, 0, None), 0.5);
    }

    #[test]
    fn tolerance_override_wins() {
        assert_eq!(tolerance_for_zoom(5, 0, Some(9.0)), 9.0);
    }

    #[test]
    fn point_feature_lands_in_expected_tile_at_z14() {
        let mut builder = TileBuilder::new(TileOptions {
            min_zoom: 14,
            max_zoom: 14,
            ..opts()
        });
        builder.add_feature(point(8.54, 47.37));
        assert!(builder.tiles.contains_key(&(14, 8580, 5737)));
    }

    #[test]
    fn hilbert_ordering_of_added_tiles() {
        let a = hilbert_id(1, 0, 0).unwrap();
        let b = hilbert_id(1, 1, 1).unwrap();
        assert_ne!(a, b);
        assert!(hilbert_id(0, 0, 0).unwrap() < a);
    }

    #[test]
    fn drop_densest_caps_tile_feature_count() {
        let mut builder = TileBuilder::new(TileOptions {
            min_zoom: 0,
            max_zoom: 0,
            ..opts()
        });
        for _ in 0..(DENSITY_LIMIT + 50) {
            builder.add_feature(point(0.0, 0.0));
        }
        assert_eq!(builder.tiles[&(0, 0, 0)].len(), DENSITY_LIMIT);
    }

    #[test]
    fn point_encodes_to_nonempty_mvt() {
        let mut builder = TileBuilder::new(TileOptions {
            min_zoom: 0,
            max_zoom: 0,
            ..opts()
        });
        builder.add_feature(point(8.54, 47.37));
        let tiles = builder.finish().unwrap();
        assert_eq!(tiles.len(), 1);
        assert!(!tiles[0].1.is_empty());
    }

    #[test]
    fn polygon_appears_as_polygon_geom_type() {
        let ring = vec![(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0), (0.0, 0.0)];
        let feature = ConvertFeature {
            geometry: Geometry::Polygon(vec![ring]),
            properties: BTreeMap::new(),
            id: None,
        };
        let mut builder = TileBuilder::new(TileOptions {
            min_zoom: 0,
            max_zoom: 0,
            ..opts()
        });
        builder.add_feature(feature);
        let tiles = builder.finish().unwrap();
        let decoded = Tile::decode(tiles[0].1.as_slice()).unwrap();
        assert_eq!(
            decoded.layers[0].features[0].r#type(),
            tile::GeomType::Polygon
        );
    }

    #[test]
    fn include_properties_filters_output() {
        assert!(property_included("keep", &["keep".to_string()], &[]));
        assert!(!property_included("drop", &["keep".to_string()], &[]));
    }

    #[test]
    fn exclude_properties_keeps_others() {
        assert!(!property_included("secret", &[], &["secret".to_string()]));
        assert!(property_included("name", &[], &["secret".to_string()]));
    }

    #[test]
    fn tile_range_covers_bbox() {
        let bbox = (-1.0, -1.0, 1.0, 1.0);
        let (tx0, ty0, tx1, ty1) = tile_range_for_bbox(bbox, 2);
        assert!(tx0 <= tx1);
        assert!(ty0 <= ty1);
    }

    #[test]
    fn extent_is_4096() {
        assert_eq!(EXTENT, 4096);
    }
}
