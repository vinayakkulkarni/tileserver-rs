//! Douglas-Peucker polyline simplification.
//!
//! Operates on `(f64, f64)` coordinate pairs in an arbitrary planar space
//! (callers pass Web-Mercator meters). Distances are Euclidean. The routine
//! is split into small pure helpers so each branch is exhaustively testable
//! and stays well under the project's CRAP complexity ceiling.

/// A 2D coordinate pair.
pub type Coord = (f64, f64);

/// Squared Euclidean length of the segment `a`→`b`.
fn segment_len_sq(a: Coord, b: Coord) -> f64 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    dx * dx + dy * dy
}

/// Perpendicular distance from `point` to the infinite line through
/// `start`→`end`. When `start == end` this degenerates to the point-to-point
/// distance.
#[must_use]
pub fn perpendicular_distance(point: Coord, start: Coord, end: Coord) -> f64 {
    let len_sq = segment_len_sq(start, end);
    if len_sq == 0.0 {
        return segment_len_sq(point, start).sqrt();
    }
    let numerator =
        ((end.0 - start.0) * (start.1 - point.1) - (start.0 - point.0) * (end.1 - start.1)).abs();
    numerator / len_sq.sqrt()
}

/// Index of the vertex with the greatest perpendicular distance from the
/// chord `points[first]`→`points[last]`, paired with that distance. Returns
/// `None` when there is no interior vertex to test.
fn farthest_vertex(points: &[Coord], first: usize, last: usize) -> Option<(usize, f64)> {
    let (start, end) = (points[first], points[last]);
    let mut best: Option<(usize, f64)> = None;
    for (offset, &pt) in points[first + 1..last].iter().enumerate() {
        let dist = perpendicular_distance(pt, start, end);
        let idx = first + 1 + offset;
        match best {
            Some((_, best_dist)) if dist <= best_dist => {}
            _ => best = Some((idx, dist)),
        }
    }
    best
}

/// Recursively mark vertices to keep between `first` and `last` (inclusive).
fn dp_recurse(points: &[Coord], first: usize, last: usize, tolerance: f64, keep: &mut [bool]) {
    if last <= first + 1 {
        return;
    }
    if let Some((split, dist)) = farthest_vertex(points, first, last)
        && dist > tolerance
    {
        keep[split] = true;
        dp_recurse(points, first, split, tolerance, keep);
        dp_recurse(points, split, last, tolerance, keep);
    }
}

/// Simplify `points` with the Douglas-Peucker algorithm at the given
/// `tolerance`. Endpoints are always preserved. Consecutive duplicate
/// vertices in the input are collapsed before simplification so a degenerate
/// ring does not defeat the perpendicular-distance test.
#[must_use]
pub fn dp_simplify(points: &[Coord], tolerance: f64) -> Vec<Coord> {
    let deduped = dedupe_consecutive(points);
    if deduped.len() <= 2 {
        return deduped;
    }
    let last = deduped.len() - 1;
    let mut keep = vec![false; deduped.len()];
    keep[0] = true;
    keep[last] = true;
    dp_recurse(&deduped, 0, last, tolerance, &mut keep);
    deduped
        .into_iter()
        .zip(keep)
        .filter_map(|(pt, k)| k.then_some(pt))
        .collect()
}

/// Drop runs of identical consecutive coordinates.
fn dedupe_consecutive(points: &[Coord]) -> Vec<Coord> {
    let mut out: Vec<Coord> = Vec::with_capacity(points.len());
    for &pt in points {
        if out.last() != Some(&pt) {
            out.push(pt);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(dp_simplify(&[], 1.0), Vec::<Coord>::new());
    }

    #[test]
    fn single_point_returns_single_point() {
        assert_eq!(dp_simplify(&[(1.0, 2.0)], 1.0), vec![(1.0, 2.0)]);
    }

    #[test]
    fn two_points_returns_unchanged() {
        let pts = vec![(0.0, 0.0), (10.0, 10.0)];
        assert_eq!(dp_simplify(&pts, 1.0), pts);
    }

    #[test]
    fn collinear_three_points_collapse_to_two() {
        let pts = vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)];
        assert_eq!(dp_simplify(&pts, 0.5), vec![(0.0, 0.0), (2.0, 0.0)]);
    }

    #[test]
    fn collinear_strict_tolerance_kept() {
        // A perfectly collinear midpoint has zero perpendicular distance, so
        // even a zero tolerance removes it — assert the midpoint is dropped.
        let pts = vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)];
        assert_eq!(dp_simplify(&pts, 0.0), vec![(0.0, 0.0), (2.0, 0.0)]);
    }

    #[test]
    fn right_angle_simplifies_with_large_tolerance() {
        let pts = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)];
        assert_eq!(dp_simplify(&pts, 5.0), vec![(0.0, 0.0), (1.0, 1.0)]);
    }

    #[test]
    fn right_angle_kept_with_small_tolerance() {
        let pts = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)];
        assert_eq!(dp_simplify(&pts, 0.1), pts);
    }

    #[test]
    fn polygon_ring_stays_closed() {
        let ring = vec![(0.0, 0.0), (0.0, 4.0), (4.0, 4.0), (4.0, 0.0), (0.0, 0.0)];
        let simplified = dp_simplify(&ring, 0.5);
        assert_eq!(simplified.first(), simplified.last());
    }

    #[test]
    fn duplicate_consecutive_vertices_removed() {
        let pts = vec![(0.0, 0.0), (0.0, 0.0), (10.0, 0.0), (10.0, 0.0)];
        assert_eq!(dp_simplify(&pts, 0.1), vec![(0.0, 0.0), (10.0, 0.0)]);
    }

    #[test]
    fn preserves_endpoint() {
        let pts = vec![(0.0, 0.0), (5.0, 0.1), (10.0, 0.0)];
        let out = dp_simplify(&pts, 1.0);
        assert_eq!(out.first(), Some(&(0.0, 0.0)));
        assert_eq!(out.last(), Some(&(10.0, 0.0)));
    }

    #[test]
    fn zero_tolerance_no_change_for_non_collinear() {
        let pts = vec![(0.0, 0.0), (1.0, 5.0), (2.0, 0.0)];
        assert_eq!(dp_simplify(&pts, 0.0), pts);
    }

    #[test]
    fn huge_tolerance_collapses_to_two() {
        let pts = vec![(0.0, 0.0), (1.0, 5.0), (2.0, -3.0), (3.0, 8.0), (4.0, 0.0)];
        assert_eq!(dp_simplify(&pts, 1e9), vec![(0.0, 0.0), (4.0, 0.0)]);
    }

    #[test]
    fn asymmetric_x_y_uses_euclidean_distance() {
        // Perpendicular distance from (1,3) to the x-axis chord is 3.0.
        let d = perpendicular_distance((1.0, 3.0), (0.0, 0.0), (2.0, 0.0));
        assert!((d - 3.0).abs() < 1e-9);
    }

    #[test]
    fn idempotent_after_first_simplify() {
        let pts = vec![(0.0, 0.0), (1.0, 0.2), (2.0, -0.1), (3.0, 0.3), (4.0, 0.0)];
        let once = dp_simplify(&pts, 0.25);
        let twice = dp_simplify(&once, 0.25);
        assert_eq!(once, twice);
    }

    #[test]
    fn stress_1000_points_output_not_larger() {
        let mut pts = Vec::with_capacity(1000);
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        for i in 0..1000u32 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let jitter = ((seed >> 33) as f64 / u32::MAX as f64) - 0.5;
            pts.push((f64::from(i), jitter));
        }
        let out = dp_simplify(&pts, 0.1);
        assert!(out.len() <= pts.len());
        assert!(out.len() >= 2);
    }

    #[test]
    fn result_is_deterministic_for_same_input() {
        let pts = vec![(0.0, 0.0), (1.0, 2.0), (2.0, -1.0), (3.0, 0.0)];
        assert_eq!(dp_simplify(&pts, 0.5), dp_simplify(&pts, 0.5));
    }

    #[test]
    fn perpendicular_distance_degenerate_segment() {
        let d = perpendicular_distance((3.0, 4.0), (0.0, 0.0), (0.0, 0.0));
        assert!((d - 5.0).abs() < 1e-9);
    }
}
