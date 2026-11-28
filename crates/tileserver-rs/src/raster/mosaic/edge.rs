//! Sub-pixel edge repair for mosaicked (multi-asset) rasters.
//!
//! # Why this module exists
//!
//! STAC supercell products (e.g. Impact Observatory LULC) carry one COG per
//! UTM/MGRS cell. Each cell's valid data ends on its own grid edge — a
//! meridian that is *slanted* relative to the Web-Mercator tile lattice and
//! reverses slant at a zone boundary. When two neighbouring cells are warped
//! into the same 256×256 tile, each marks the fractional-pixel coverage at
//! its footprint edge as nodata. Their two edges land on nearly the same
//! output column with opposite sub-pixel phase, so the union leaves a
//! 1–2 px transparent slot that neither asset fills.
//!
//! Measured (2026-11-27, z7/z9/z11 at 49.5°N across the 12°E UTM 32/33
//! boundary): the slot's *ground* width shrinks 16× with zoom (≈2380 m at z7
//! → ≈150 m at z11) while its *pixel* width stays flat at ~3 px. That pins
//! the defect to the rendering/pixel domain, not to genuinely missing ground
//! — a data gap would hold constant in metres and grow in pixels.
//!
//! Compositing itself is innocent: an isolation test showed the composite is
//! the exact pixel union of the single-asset renders (0 px discrepancy), so
//! the repair belongs at the per-asset edge and as a post-composite net.
//!
//! # Two guards, in order
//!
//! 1. [`dilate_masked_edges`] — run on *each* decoded asset before it is fed
//!    to the mosaic method. Restores one pixel of the fractional edge from an
//!    opaque inward neighbour, so each neighbour contributes the sliver it
//!    actually covers. This is the real fix.
//! 2. [`close_holes_1px`] — run *once* on the finalised composite. Fills a
//!    masked pixel that has opaque pixels on *both* sides of an axis (a 1-px
//!    morphological close). This catches any residual where the slot straddles
//!    slightly more than one column, and by construction cannot smear
//!    anything wider than one pixel.
//!
//! Both copy an existing neighbour's value verbatim, so a categorical class id
//! is never blended into a non-existent id (unlike bilinear, which would invent
//! intermediate values between two classes).

use crate::raster::RasterImage;

/// Restore one pixel of masked footprint edge from an opaque 4-neighbour.
///
/// For every pixel that was masked on entry, if any of its four orthogonal
/// neighbours was opaque on entry, copy that neighbour's band values in and
/// clear the mask. Neighbour lookup uses a snapshot of the *original* mask so
/// the dilation is exactly one pixel deep and does not cascade inward.
///
/// Returns the number of pixels repaired (useful for tests and tracing).
///
/// # Scope
///
/// Applied per asset inside the STAC mosaic path only, never to a single-COG
/// source: a lone COG that ends at a real coastline has no neighbour to meet,
/// so extending it would only add a halo with no seam to close.
pub fn dilate_masked_edges(img: &mut RasterImage) -> usize {
    let (bands, height, width) = img.data().dim();
    let (mut data, mut mask) = img.views_mut();

    // Snapshot opacity so dilation cannot cascade more than one pixel.
    let mut was_opaque = vec![false; height * width];
    for y in 0..height {
        for x in 0..width {
            was_opaque[y * width + x] = !mask[[y, x]];
        }
    }

    let mut repaired = 0usize;
    for y in 0..height {
        for x in 0..width {
            if was_opaque[y * width + x] {
                continue;
            }
            // Prefer a vertical source, then horizontal: seam slivers run
            // along either axis and either neighbour is equally valid.
            let src = if y > 0 && was_opaque[(y - 1) * width + x] {
                Some((y - 1, x))
            } else if y + 1 < height && was_opaque[(y + 1) * width + x] {
                Some((y + 1, x))
            } else if x > 0 && was_opaque[y * width + x - 1] {
                Some((y, x - 1))
            } else if x + 1 < width && was_opaque[y * width + x + 1] {
                Some((y, x + 1))
            } else {
                None
            };
            let Some((sy, sx)) = src else { continue };
            for b in 0..bands {
                data[[b, y, x]] = data[[b, sy, sx]];
            }
            mask[[y, x]] = false;
            repaired += 1;
        }
    }
    repaired
}

/// Close one-pixel holes in the finalised composite.
///
/// A masked pixel is filled only when it has opaque pixels on **both** sides
/// along the same axis (left+right, or up+down). This is a morphological
/// close with a 1-px structuring element, so it can never bridge a gap wider
/// than one pixel and therefore cannot smooth away real voids, cloud gaps or
/// coastlines.
///
/// The filled value is copied from the left (or top) opaque neighbour — an
/// existing class id, never an interpolated one.
///
/// Returns the number of pixels filled.
pub fn close_holes_1px(img: &mut RasterImage) -> usize {
    let (bands, height, width) = img.data().dim();
    let (mut data, mut mask) = img.views_mut();

    let mut was_opaque = vec![false; height * width];
    for y in 0..height {
        for x in 0..width {
            was_opaque[y * width + x] = !mask[[y, x]];
        }
    }

    let mut filled = 0usize;
    for y in 0..height {
        for x in 0..width {
            if was_opaque[y * width + x] {
                continue;
            }
            let left = x > 0 && was_opaque[y * width + x - 1];
            let right = x + 1 < width && was_opaque[y * width + x + 1];
            let up = y > 0 && was_opaque[(y - 1) * width + x];
            let down = y + 1 < height && was_opaque[(y + 1) * width + x];

            let src = if left && right {
                Some((y, x - 1))
            } else if up && down {
                Some((y - 1, x))
            } else {
                None
            };
            let Some((sy, sx)) = src else { continue };
            for b in 0..bands {
                data[[b, y, x]] = data[[b, sy, sx]];
            }
            mask[[y, x]] = false;
            filled += 1;
        }
    }
    filled
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::{Array2, Array3};

    /// Build an image from a per-pixel "is opaque" map; opaque pixels carry a
    /// deterministic class value equal to their linear index so copied values
    /// are easy to assert.
    fn from_opacity(rows: &[&str]) -> RasterImage {
        let h = rows.len();
        let w = rows[0].len();
        let mut data = Array3::<f32>::zeros((1, h, w));
        let mut mask = Array2::<bool>::from_elem((h, w), false);
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if ch == '.' {
                    mask[[y, x]] = true;
                } else {
                    data[[0, y, x]] = (y * w + x) as f32;
                }
            }
        }
        RasterImage::new(data, mask, None)
    }

    fn mask_row(img: &RasterImage, y: usize) -> String {
        (0..img.width())
            .map(|x| if img.mask()[[y, x]] { '.' } else { '#' })
            .collect()
    }

    #[test]
    fn dilate_fills_one_pixel_into_masked_side() {
        // Two opaque columns then a masked run: dilation extends by exactly 1.
        let mut img = from_opacity(&["##......"]);
        let n = dilate_masked_edges(&mut img);
        assert_eq!(n, 1, "only the pixel adjacent to opaque is filled");
        assert_eq!(mask_row(&img, 0), "###.....");
        // The copied value comes from the inward neighbour (index 1).
        assert!((img.data()[[0, 0, 2]] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn dilate_does_not_cascade() {
        // A lone opaque pixel must grow by 1 in each direction, not flood.
        let mut img = from_opacity(&["...", ".#.", "..."]);
        let n = dilate_masked_edges(&mut img);
        assert_eq!(n, 4, "four orthogonal neighbours, one pixel deep");
        assert_eq!(mask_row(&img, 0), ".#.");
        assert_eq!(mask_row(&img, 1), "###");
        assert_eq!(mask_row(&img, 2), ".#.");
    }

    #[test]
    fn dilate_leaves_fully_opaque_untouched() {
        let mut img = from_opacity(&["###", "###"]);
        assert_eq!(dilate_masked_edges(&mut img), 0);
        assert_eq!(mask_row(&img, 0), "###");
    }

    #[test]
    fn dilate_leaves_fully_masked_untouched() {
        let mut img = from_opacity(&["...", "..."]);
        assert_eq!(
            dilate_masked_edges(&mut img),
            0,
            "no opaque seed to dilate from"
        );
    }

    #[test]
    fn close_fills_one_pixel_vertical_slot() {
        // A single masked column between two opaque ones is a 1-px hole.
        let mut img = from_opacity(&["#.#", "#.#", "#.#"]);
        let n = close_holes_1px(&mut img);
        assert_eq!(n, 3);
        assert_eq!(mask_row(&img, 0), "###");
        // Value copied from the left neighbour (index 0), not invented.
        assert!((img.data()[[0, 0, 1]] - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn close_ignores_gap_wider_than_one_pixel() {
        // A two-column void must NOT be closed: it may be a real gap.
        let mut img = from_opacity(&["#..#", "#..#"]);
        assert_eq!(close_holes_1px(&mut img), 0);
        assert_eq!(mask_row(&img, 0), "#..#");
    }

    #[test]
    fn close_fills_single_row_hole() {
        let mut img = from_opacity(&["###", "...", "###"]);
        let n = close_holes_1px(&mut img);
        assert_eq!(n, 3);
        assert_eq!(mask_row(&img, 1), "###");
    }

    #[test]
    fn close_leaves_edges_alone() {
        // A masked pixel touching the tile border has no neighbour on that
        // side, so it must not be filled.
        let mut img = from_opacity(&[".#", ".#"]);
        assert_eq!(close_holes_1px(&mut img), 0);
    }

    #[test]
    fn close_on_the_measured_seam_shape() {
        // Reproduces the real defect: two assets each ending mid-column with
        // opposite sub-pixel phase leave a 1-px slot that close() seals.
        // After per-asset dilation the slot is exactly one column wide.
        let mut left = from_opacity(&["####....", "####...."]);
        let mut right = from_opacity(&["....####", "....####"]);
        dilate_masked_edges(&mut left);
        dilate_masked_edges(&mut right);
        // Union: left now covers 0..5, right covers 3..8 -> already opaque
        // everywhere via the mosaic; assert the union has no hole.
        let mut union = left;
        let (data_r, mask_r) = right.into_parts();
        {
            let (mut data_u, mut mask_u) = union.views_mut();
            let (_, h, w) = data_u.dim();
            for y in 0..h {
                for x in 0..w {
                    if mask_u[[y, x]] && !mask_r[[y, x]] {
                        for b in 0..1 {
                            data_u[[b, y, x]] = data_r[[b, y, x]];
                        }
                        mask_u[[y, x]] = false;
                    }
                }
            }
        }
        assert_eq!(mask_row(&union, 0), "########");
        assert_eq!(close_holes_1px(&mut union), 0, "nothing left to close");
    }
}
