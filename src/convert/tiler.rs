use geo::{BoundingRect, Geometry, Rect};

/// Convert a longitude to a tile X coordinate at the given zoom level.
#[inline]
pub fn lon_to_tile_x(lon: f64, z: u8) -> u32 {
    let n = 2u32.pow(u32::from(z));
    let x = ((lon + 180.0) / 360.0 * f64::from(n)) as u32;
    x.min(n - 1)
}

/// Convert a latitude to a tile Y coordinate at the given zoom level.
#[inline]
pub fn lat_to_tile_y(lat: f64, z: u8) -> u32 {
    let n = 2u32.pow(u32::from(z));
    let lat_rad = lat.to_radians();
    let y = ((1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0
        * f64::from(n)) as u32;
    y.min(n - 1)
}

/// Return the geographic bounding box (west, south, east, north) for a tile.
pub fn tile_to_bbox(z: u8, x: u32, y: u32) -> Rect<f64> {
    let n = 2u32.pow(u32::from(z)) as f64;
    let west = x as f64 / n * 360.0 - 180.0;
    let east = (x + 1) as f64 / n * 360.0 - 180.0;
    let north = tile_y_to_lat(y, z);
    let south = tile_y_to_lat(y + 1, z);
    Rect::new(
        geo::Coord { x: west, y: south },
        geo::Coord { x: east, y: north },
    )
}

fn tile_y_to_lat(y: u32, z: u8) -> f64 {
    let n = 2u32.pow(u32::from(z)) as f64;
    let lat_rad = (std::f64::consts::PI * (1.0 - 2.0 * y as f64 / n))
        .sinh()
        .atan();
    lat_rad.to_degrees()
}

/// Compute a Douglas-Peucker simplification tolerance (in degrees) for a given zoom level.
///
/// When `base_tolerance` is `Some(t)`, the user-specified value is scaled per zoom:
/// `tolerance = t / 2^z` — same behaviour as before.
///
/// When `None`, a **pixel-based auto formula** is used:
/// `tolerance = tile_width_degrees(z) / 4096 * 1.5`
/// This equals ~1.5 pixels of tolerance at the given zoom — visually lossless for
/// most datasets while reducing coordinate count by 30–70%.
pub fn simplify_tolerance(z: u8, base_tolerance: Option<f64>) -> f64 {
    match base_tolerance {
        Some(base) => base / 2f64.powi(i32::from(z)),
        None => {
            // tile_width_degrees(z) = 360 / 2^z
            // 1 pixel = tile_width / MVT_EXTENT (4096)
            let tile_width = 360.0 / f64::from(1u32 << u32::from(z));
            tile_width / 4096.0 * 1.5
        }
    }
}

/// Auto-detect a sensible maximum zoom level from pre-computed feature bounding boxes.
///
/// **Algorithm:** find the feature with the smallest geographic extent (largest
/// bounding-box dimension). Compute the Web Mercator zoom level at which that
/// feature spans exactly one tile. That zoom is the minimum needed to resolve
/// the feature at tile-level granularity; clamped to `[0, 14]`.
///
/// For point-only datasets (no bounding-box area), a tippecanoe-style
/// count-based heuristic is used: `floor(log2(count)) + 7`, clamped to `[0, 14]`.
///
/// `bboxes` must be pre-computed (e.g. via a parallel `bounding_rect` sweep) so
/// that the per-coordinate cost is paid only once across the whole pipeline.
pub fn auto_max_zoom(bboxes: &[Option<Rect<f64>>], feature_count: usize) -> u8 {
    let mut min_extent_deg: f64 = f64::INFINITY;
    let mut has_non_point = false;

    for bbox in bboxes.iter().flatten() {
        let w = (bbox.max().x - bbox.min().x).abs();
        let h = (bbox.max().y - bbox.min().y).abs();
        // Use the *larger* dimension — represents the feature's overall scale
        let extent = w.max(h);
        if extent > 1e-7 {
            has_non_point = true;
            min_extent_deg = min_extent_deg.min(extent);
        }
    }

    if !has_non_point {
        // Point-only: count-based heuristic (tippecanoe-style)
        let z = (feature_count as f64).log2().floor() as i32 + 7;
        return z.clamp(0, 14) as u8;
    }

    // Zoom where the smallest feature spans approximately one tile:
    //   tile_width_at_z = 360 / 2^z
    //   min_extent = tile_width  →  z = log2(360 / min_extent)
    let z = (360.0_f64 / min_extent_deg).log2().ceil() as i32;
    z.clamp(0, 14) as u8
}

/// Tile assignment for a feature: returns the range of (x, y) tile coords at zoom `z`
/// that the feature's bounding box overlaps.
pub fn tile_range(geometry: &Geometry<f64>, z: u8) -> Option<TileRange> {
    let bbox = geometry.bounding_rect()?;
    Some(tile_range_from_bbox(&bbox, z))
}

/// Tile range from a pre-computed bounding box — avoids re-scanning all coordinates.
pub fn tile_range_from_bbox(bbox: &Rect<f64>, z: u8) -> TileRange {
    let min_x = lon_to_tile_x(bbox.min().x, z);
    let max_x = lon_to_tile_x(bbox.max().x, z);
    // Note: lat_to_tile_y is inverted (north has smaller Y)
    let min_y = lat_to_tile_y(bbox.max().y, z);
    let max_y = lat_to_tile_y(bbox.min().y, z);
    TileRange {
        min_x,
        max_x,
        min_y,
        max_y,
    }
}

/// A rectangular range of tile coordinates.
#[derive(Debug, Clone, Copy)]
pub struct TileRange {
    pub min_x: u32,
    pub max_x: u32,
    pub min_y: u32,
    pub max_y: u32,
}

impl TileRange {
    /// Total tile count in this range.
    pub fn count(&self) -> u64 {
        (self.max_x - self.min_x + 1) as u64 * (self.max_y - self.min_y + 1) as u64
    }

    /// Iterate over all (x, y) pairs in this range.
    pub fn iter(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        (self.min_y..=self.max_y).flat_map(move |y| (self.min_x..=self.max_x).map(move |x| (x, y)))
    }
}

/// Compute the overall dataset bounding box from pre-computed feature bounding boxes.
pub fn dataset_bbox(bboxes: &[Option<Rect<f64>>]) -> Option<Rect<f64>> {
    let mut west = f64::INFINITY;
    let mut south = f64::INFINITY;
    let mut east = f64::NEG_INFINITY;
    let mut north = f64::NEG_INFINITY;
    let mut any = false;

    for bbox in bboxes.iter().flatten() {
        west = west.min(bbox.min().x);
        south = south.min(bbox.min().y);
        east = east.max(bbox.max().x);
        north = north.max(bbox.max().y);
        any = true;
    }

    if any {
        Some(Rect::new(
            geo::Coord { x: west, y: south },
            geo::Coord { x: east, y: north },
        ))
    } else {
        None
    }
}

/// Expand a bounding rectangle by a fractional buffer (e.g. 0.02 = 2%).
pub fn expand_bbox(bbox: &Rect<f64>, fraction: f64) -> Rect<f64> {
    let dx = (bbox.max().x - bbox.min().x) * fraction;
    let dy = (bbox.max().y - bbox.min().y) * fraction;
    Rect::new(
        geo::Coord {
            x: (bbox.min().x - dx).max(-180.0),
            y: (bbox.min().y - dy).max(-90.0),
        },
        geo::Coord {
            x: (bbox.max().x + dx).min(180.0),
            y: (bbox.max().y + dy).min(90.0),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_x_at_zoom0() {
        assert_eq!(lon_to_tile_x(0.0, 0), 0);
        assert_eq!(lon_to_tile_x(-180.0, 0), 0);
    }

    #[test]
    fn tile_y_at_zoom0() {
        assert_eq!(lat_to_tile_y(0.0, 0), 0);
    }

    #[test]
    fn tile_bbox_roundtrip() {
        let bbox = tile_to_bbox(1, 1, 0);
        assert!((bbox.min().x - 0.0).abs() < 1.0);
        assert!(bbox.max().x > 0.0);
    }

    #[test]
    fn tile_x_boundary_values() {
        // -180° → tile 0 at any zoom
        assert_eq!(lon_to_tile_x(-180.0, 1), 0);
        // 180° wraps to last tile
        assert_eq!(lon_to_tile_x(180.0, 1), 1);
        // Prime meridian at zoom 1: left half
        assert_eq!(lon_to_tile_x(0.0, 1), 1);
    }

    #[test]
    fn tile_y_north_is_zero() {
        // Northern latitudes map to smaller tile Y
        let y_north = lat_to_tile_y(85.0, 2);
        let y_south = lat_to_tile_y(-85.0, 2);
        assert!(
            y_north < y_south,
            "Northern tiles should have lower Y index"
        );
    }

    #[test]
    fn simplify_tolerance_equals_base_at_zoom0() {
        let base = 0.5;
        let t = simplify_tolerance(0, Some(base));
        assert!((t - base).abs() < 1e-10);
    }

    #[test]
    fn simplify_tolerance_halves_each_zoom() {
        let base = 0.4;
        let t0 = simplify_tolerance(0, Some(base));
        let t1 = simplify_tolerance(1, Some(base));
        let t2 = simplify_tolerance(2, Some(base));
        assert!((t1 - t0 / 2.0).abs() < 1e-12);
        assert!((t2 - t0 / 4.0).abs() < 1e-12);
    }

    #[test]
    fn simplify_tolerance_auto_is_pixel_based() {
        // Auto formula: (360 / 2^z) / 4096 * 1.5
        // At z=0: 360 / 4096 * 1.5 ≈ 0.13183
        let t = simplify_tolerance(0, None);
        let expected = 360.0 / 4096.0 * 1.5;
        assert!((t - expected).abs() < 1e-10);
    }

    #[test]
    fn simplify_tolerance_stays_positive_at_high_zoom() {
        let t = simplify_tolerance(22, None);
        assert!(t > 0.0);
        assert!(t.is_finite());
    }

    #[test]
    fn tile_range_point_is_single_tile() {
        use geo::{Geometry, Point};
        let geom = Geometry::Point(Point::new(13.4, 52.5)); // Berlin
        let range = tile_range(&geom, 6).unwrap();
        assert_eq!(range.count(), 1, "A point should map to exactly one tile");
    }

    #[test]
    fn tile_range_world_bbox_at_zoom0() {
        let world = Rect::new(
            geo::Coord {
                x: -180.0,
                y: -90.0,
            },
            geo::Coord { x: 180.0, y: 90.0 },
        );
        let geom = Geometry::Rect(world);
        let range = tile_range(&geom, 0).unwrap();
        assert_eq!(range.min_x, 0);
        assert_eq!(range.max_x, 0);
        assert_eq!(range.min_y, 0);
        assert_eq!(range.max_y, 0);
    }

    #[test]
    fn tile_range_count_matches_iter_len() {
        use geo::{Geometry, Point};
        let geom = Geometry::Point(Point::new(0.0, 0.0));
        let range = tile_range(&geom, 4).unwrap();
        let iter_count = range.iter().count() as u64;
        assert_eq!(range.count(), iter_count);
    }

    #[test]
    fn dataset_bbox_empty_returns_none() {
        let result = dataset_bbox(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn dataset_bbox_single_point() {
        let pt_bbox = Some(Rect::new(
            geo::Coord { x: 10.0, y: 50.0 },
            geo::Coord { x: 10.0, y: 50.0 },
        ));
        let bbox = dataset_bbox(&[pt_bbox]).unwrap();
        assert!((bbox.min().x - 10.0).abs() < 1e-10);
        assert!((bbox.min().y - 50.0).abs() < 1e-10);
        assert!((bbox.max().x - 10.0).abs() < 1e-10);
        assert!((bbox.max().y - 50.0).abs() < 1e-10);
    }

    #[test]
    fn dataset_bbox_two_points_spans_both() {
        let bboxes = vec![
            Some(Rect::new(
                geo::Coord { x: -10.0, y: 20.0 },
                geo::Coord { x: -10.0, y: 20.0 },
            )),
            Some(Rect::new(
                geo::Coord { x: 30.0, y: 60.0 },
                geo::Coord { x: 30.0, y: 60.0 },
            )),
        ];
        let bbox = dataset_bbox(&bboxes).unwrap();
        assert!((bbox.min().x - (-10.0)).abs() < 1e-10);
        assert!((bbox.min().y - 20.0).abs() < 1e-10);
        assert!((bbox.max().x - 30.0).abs() < 1e-10);
        assert!((bbox.max().y - 60.0).abs() < 1e-10);
    }

    #[test]
    fn expand_bbox_increases_all_sides() {
        let bbox = Rect::new(
            geo::Coord { x: 0.0, y: 0.0 },
            geo::Coord { x: 10.0, y: 10.0 },
        );
        let expanded = expand_bbox(&bbox, 0.1);
        assert!(expanded.min().x < bbox.min().x);
        assert!(expanded.min().y < bbox.min().y);
        assert!(expanded.max().x > bbox.max().x);
        assert!(expanded.max().y > bbox.max().y);
    }

    #[test]
    fn expand_bbox_clamped_to_world_bounds() {
        let world = Rect::new(
            geo::Coord {
                x: -180.0,
                y: -90.0,
            },
            geo::Coord { x: 180.0, y: 90.0 },
        );
        let expanded = expand_bbox(&world, 1.0);
        assert!(expanded.min().x >= -180.0);
        assert!(expanded.min().y >= -90.0);
        assert!(expanded.max().x <= 180.0);
        assert!(expanded.max().y <= 90.0);
    }

    #[test]
    fn tile_to_bbox_covers_full_world_at_zoom0() {
        let bbox = tile_to_bbox(0, 0, 0);
        assert!((bbox.min().x - (-180.0)).abs() < 1e-6);
        assert!(bbox.max().x > 179.0);
        // Mercator north/south are clamped (~85.05°)
        assert!(bbox.max().y > 80.0);
        assert!(bbox.min().y < -80.0);
    }
}
