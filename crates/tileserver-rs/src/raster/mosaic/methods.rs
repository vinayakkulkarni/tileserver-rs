//! The 8 mosaic pixel-selection methods.
//!
//! Each method is a unit-struct that implements [`MosaicMethod`].
//! See the parent module for the overall design.

use ndarray::{Array2, Array3};

use crate::config::PixelSelectionMethod;
use crate::raster::RasterImage;

/// Trait implemented by every mosaic pixel-selection method.
///
/// Methods are stateful across calls to [`Self::feed`]; construction
/// defers allocation of the accumulator until the first layer arrives
/// so shape information can be inferred from the input.
pub trait MosaicMethod: Send {
    /// Ingest one input layer in priority order (index 0 = highest).
    fn feed(&mut self, layer: RasterImage);

    /// Returns true when the method is short-circuit-ready.
    ///
    /// The mosaic loop polls this after every [`Self::feed`] and skips
    /// further layer rendering when true.  Methods that need every
    /// input (e.g., mean, median, stdev) always return false.
    fn is_done(&self) -> bool;

    /// Returns the final composited image, or `None` when no layer was fed.
    fn finalize(self: Box<Self>) -> Option<RasterImage>;
}

/// Construct a [`MosaicMethod`] trait object for the given config variant.
#[must_use]
pub fn build(method: PixelSelectionMethod) -> Box<dyn MosaicMethod> {
    match method {
        PixelSelectionMethod::First => Box::new(FirstMethod::default()),
        PixelSelectionMethod::Highest => Box::new(HighestMethod::default()),
        PixelSelectionMethod::Lowest => Box::new(LowestMethod::default()),
        PixelSelectionMethod::Mean => Box::new(MeanMethod::default()),
        PixelSelectionMethod::Median => Box::new(MedianMethod::default()),
        PixelSelectionMethod::Stdev => Box::new(StdevMethod::default()),
        PixelSelectionMethod::Count => Box::new(CountMethod::default()),
        PixelSelectionMethod::LowestCloudCover => Box::new(LowestCloudCoverMethod::default()),
    }
}

// =============================================================================
// FirstMethod — highest-priority pixel wins where it is valid.
// =============================================================================

/// `first`: the highest-priority layer wins per-pixel where it is unmasked;
/// lower-priority layers fill in where their pixels are unmasked.
///
/// Short-circuits once the canvas has zero masked pixels.
#[derive(Debug, Default)]
pub struct FirstMethod {
    canvas: Option<RasterImage>,
}

impl MosaicMethod for FirstMethod {
    fn feed(&mut self, layer: RasterImage) {
        match self.canvas.as_mut() {
            None => self.canvas = Some(layer),
            Some(canvas) => fill_masked_from(canvas, &layer),
        }
    }

    fn is_done(&self) -> bool {
        self.canvas
            .as_ref()
            .is_some_and(RasterImage::is_fully_opaque)
    }

    fn finalize(self: Box<Self>) -> Option<RasterImage> {
        self.canvas
    }
}

// =============================================================================
// HighestMethod / LowestMethod — per-pixel max / min.
// =============================================================================

/// `highest`: per-pixel maximum across all valid inputs.
#[derive(Debug, Default)]
pub struct HighestMethod {
    canvas: Option<RasterImage>,
}

impl MosaicMethod for HighestMethod {
    fn feed(&mut self, layer: RasterImage) {
        reduce_by(self, layer, f32::max);
    }
    fn is_done(&self) -> bool {
        false
    }
    fn finalize(self: Box<Self>) -> Option<RasterImage> {
        self.canvas
    }
}

impl PerPixelCanvas for HighestMethod {
    fn canvas_mut(&mut self) -> &mut Option<RasterImage> {
        &mut self.canvas
    }
}

/// `lowest`: per-pixel minimum across all valid inputs.
#[derive(Debug, Default)]
pub struct LowestMethod {
    canvas: Option<RasterImage>,
}

impl MosaicMethod for LowestMethod {
    fn feed(&mut self, layer: RasterImage) {
        reduce_by(self, layer, f32::min);
    }
    fn is_done(&self) -> bool {
        false
    }
    fn finalize(self: Box<Self>) -> Option<RasterImage> {
        self.canvas
    }
}

impl PerPixelCanvas for LowestMethod {
    fn canvas_mut(&mut self) -> &mut Option<RasterImage> {
        &mut self.canvas
    }
}

// =============================================================================
// MeanMethod / StdevMethod — statistical reductions over stacked layers.
// =============================================================================

/// `mean`: per-pixel arithmetic mean across all valid (unmasked) inputs.
#[derive(Debug, Default)]
pub struct MeanMethod {
    stack: StatsAccumulator,
}

impl MosaicMethod for MeanMethod {
    fn feed(&mut self, layer: RasterImage) {
        self.stack.push(layer);
    }
    fn is_done(&self) -> bool {
        false
    }
    fn finalize(self: Box<Self>) -> Option<RasterImage> {
        self.stack.finalize_mean()
    }
}

/// `stdev`: per-pixel standard deviation across all valid inputs.
/// Uses the population estimator (divides by N, not N-1) to match rio-tiler.
#[derive(Debug, Default)]
pub struct StdevMethod {
    stack: StatsAccumulator,
}

impl MosaicMethod for StdevMethod {
    fn feed(&mut self, layer: RasterImage) {
        self.stack.push(layer);
    }
    fn is_done(&self) -> bool {
        false
    }
    fn finalize(self: Box<Self>) -> Option<RasterImage> {
        self.stack.finalize_stdev()
    }
}

// =============================================================================
// MedianMethod — exact median from stacked layers.
// =============================================================================

/// `median`: per-pixel exact median (requires retaining all layers).
#[derive(Debug, Default)]
pub struct MedianMethod {
    layers: Vec<RasterImage>,
}

impl MosaicMethod for MedianMethod {
    fn feed(&mut self, layer: RasterImage) {
        self.layers.push(layer);
    }
    fn is_done(&self) -> bool {
        false
    }
    fn finalize(self: Box<Self>) -> Option<RasterImage> {
        let mut layers = self.layers;
        if layers.is_empty() {
            return None;
        }
        let first = layers.remove(0);
        let (bands, height, width) = first.data().dim();
        let mut samples: Vec<f32> = Vec::with_capacity(layers.len() + 1);
        let mut out = Array3::<f32>::zeros((bands, height, width));
        let mut mask_out = Array2::from_elem((height, width), true);
        for y in 0..height {
            for x in 0..width {
                for b in 0..bands {
                    samples.clear();
                    if !first.mask()[[y, x]] {
                        samples.push(first.data()[[b, y, x]]);
                    }
                    for layer in &layers {
                        if !layer.mask()[[y, x]] {
                            samples.push(layer.data()[[b, y, x]]);
                        }
                    }
                    if !samples.is_empty() {
                        samples.sort_unstable_by(|a, b| {
                            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        out[[b, y, x]] = samples[samples.len() / 2];
                        mask_out[[y, x]] = false;
                    }
                }
            }
        }
        Some(RasterImage::new(out, mask_out, first.nodata()))
    }
}

// =============================================================================
// CountMethod — QA helper: count of valid layer contributions per pixel.
// =============================================================================

/// `count`: encode the per-pixel count of valid (unmasked) layer contributions.
///
/// The result is a single-band image with count values in `[0, N]` where
/// `N` is the number of fed layers.  Primarily a debug/QA visualisation.
#[derive(Debug, Default)]
pub struct CountMethod {
    counts: Option<Array2<f32>>,
    height: usize,
    width: usize,
    nodata: Option<f64>,
}

impl MosaicMethod for CountMethod {
    fn feed(&mut self, layer: RasterImage) {
        if self.counts.is_none() {
            let (_, h, w) = layer.data().dim();
            self.counts = Some(Array2::<f32>::zeros((h, w)));
            self.height = h;
            self.width = w;
            self.nodata = layer.nodata();
        }
        let counts = self
            .counts
            .as_mut()
            .expect("counts initialised above on first feed");
        for y in 0..self.height {
            for x in 0..self.width {
                if !layer.mask()[[y, x]] {
                    counts[[y, x]] += 1.0;
                }
            }
        }
    }
    fn is_done(&self) -> bool {
        false
    }
    fn finalize(self: Box<Self>) -> Option<RasterImage> {
        let data = self.counts?.insert_axis(ndarray::Axis(0));
        let mask = Array2::from_elem((self.height, self.width), false);
        Some(RasterImage::new(data, mask, self.nodata))
    }
}

// =============================================================================
// LowestCloudCoverMethod — honoured by the pipeline via pre-sort.
// =============================================================================

/// `lowest_cloud_cover`: behaves identically to [`FirstMethod`] at the
/// mosaic layer, but the pipeline sorts incoming assets by
/// `eo:cloud_cover` ascending before feeding, so the first-wins semantics
/// yield the clearest-sky asset on top.
///
/// Assets with missing `cloud_cover` metadata sort to the end (treated
/// as worst-case 101% cloud cover) so explicit-clear assets always
/// win when present.
#[derive(Debug, Default)]
pub struct LowestCloudCoverMethod {
    inner: FirstMethod,
}

impl MosaicMethod for LowestCloudCoverMethod {
    fn feed(&mut self, layer: RasterImage) {
        self.inner.feed(layer);
    }
    fn is_done(&self) -> bool {
        self.inner.is_done()
    }
    fn finalize(self: Box<Self>) -> Option<RasterImage> {
        Box::new(self.inner).finalize()
    }
}

// =============================================================================
// Shared helpers.
// =============================================================================

/// Trait used by per-pixel reducers (Highest/Lowest) to share their feed
/// implementation via [`reduce_by`].
trait PerPixelCanvas {
    fn canvas_mut(&mut self) -> &mut Option<RasterImage>;
}

fn reduce_by<M: PerPixelCanvas + ?Sized>(
    method: &mut M,
    layer: RasterImage,
    reducer: fn(f32, f32) -> f32,
) {
    let canvas_slot = method.canvas_mut();
    if canvas_slot.is_none() {
        *canvas_slot = Some(layer);
        return;
    }
    // Safe: we just verified canvas_slot is Some.
    let canvas = canvas_slot
        .as_mut()
        .expect("canvas_slot is Some per prior is_none check");
    let (bands, height, width) = canvas.data().dim();
    let (_, lh, lw) = layer.data().dim();
    if height != lh || width != lw {
        return;
    }
    let (mut data, mut mask) = unsafe_parts_mut(canvas);
    for y in 0..height {
        for x in 0..width {
            let layer_masked = layer.mask()[[y, x]];
            if layer_masked {
                continue;
            }
            let canvas_masked = mask[[y, x]];
            for b in 0..bands {
                let new_val = layer.data()[[b, y, x]];
                data[[b, y, x]] = if canvas_masked {
                    new_val
                } else {
                    reducer(data[[b, y, x]], new_val)
                };
            }
            if canvas_masked {
                mask[[y, x]] = false;
            }
        }
    }
}

/// Unsafe-free mutable-view helper: splits a [`RasterImage`]'s data and
/// mask into concurrent `&mut` views via consume-reconstruct.
///
/// # Panics
///
/// Never panics in practice because the method is only called while
/// `canvas_slot` is `Some`; kept pure-safe via ndarray view APIs.
fn unsafe_parts_mut(
    img: &mut RasterImage,
) -> (
    ndarray::ArrayViewMut3<'_, f32>,
    ndarray::ArrayViewMut2<'_, bool>,
) {
    img.views_mut()
}

// =============================================================================
// Statistics accumulator shared by MeanMethod / StdevMethod.
// =============================================================================

#[derive(Debug, Default)]
struct StatsAccumulator {
    // Running sum per pixel per band — f64 to avoid catastrophic precision loss.
    sum: Option<Array3<f64>>,
    sum_sq: Option<Array3<f64>>,
    counts: Option<Array2<u32>>,
    nodata: Option<f64>,
    bands: usize,
    height: usize,
    width: usize,
}

impl StatsAccumulator {
    fn push(&mut self, layer: RasterImage) {
        if self.sum.is_none() {
            let (b, h, w) = layer.data().dim();
            self.bands = b;
            self.height = h;
            self.width = w;
            self.sum = Some(Array3::<f64>::zeros((b, h, w)));
            self.sum_sq = Some(Array3::<f64>::zeros((b, h, w)));
            self.counts = Some(Array2::<u32>::zeros((h, w)));
            self.nodata = layer.nodata();
        }
        let sum = self
            .sum
            .as_mut()
            .expect("sum initialised on first push above");
        let sum_sq = self
            .sum_sq
            .as_mut()
            .expect("sum_sq initialised on first push above");
        let counts = self
            .counts
            .as_mut()
            .expect("counts initialised on first push above");
        for y in 0..self.height {
            for x in 0..self.width {
                if layer.mask()[[y, x]] {
                    continue;
                }
                counts[[y, x]] += 1;
                for b in 0..self.bands {
                    let v = f64::from(layer.data()[[b, y, x]]);
                    sum[[b, y, x]] += v;
                    sum_sq[[b, y, x]] += v * v;
                }
            }
        }
    }

    fn finalize_mean(self) -> Option<RasterImage> {
        let sum = self.sum?;
        let counts = self.counts?;
        let (b, h, w) = sum.dim();
        let mut data = Array3::<f32>::zeros((b, h, w));
        let mut mask = Array2::from_elem((h, w), true);
        for y in 0..h {
            for x in 0..w {
                let n = counts[[y, x]];
                if n == 0 {
                    continue;
                }
                mask[[y, x]] = false;
                let n_f64 = f64::from(n);
                for band in 0..b {
                    data[[band, y, x]] = (sum[[band, y, x]] / n_f64) as f32;
                }
            }
        }
        Some(RasterImage::new(data, mask, self.nodata))
    }

    fn finalize_stdev(self) -> Option<RasterImage> {
        let sum = self.sum?;
        let sum_sq = self.sum_sq?;
        let counts = self.counts?;
        let (b, h, w) = sum.dim();
        let mut data = Array3::<f32>::zeros((b, h, w));
        let mut mask = Array2::from_elem((h, w), true);
        for y in 0..h {
            for x in 0..w {
                let n = counts[[y, x]];
                if n == 0 {
                    continue;
                }
                mask[[y, x]] = false;
                let n_f64 = f64::from(n);
                for band in 0..b {
                    let mean = sum[[band, y, x]] / n_f64;
                    let variance = (sum_sq[[band, y, x]] / n_f64) - (mean * mean);
                    data[[band, y, x]] = variance.max(0.0).sqrt() as f32;
                }
            }
        }
        Some(RasterImage::new(data, mask, self.nodata))
    }
}

// =============================================================================
// Shared helper for FirstMethod / LowestCloudCoverMethod — in-place fill.
// =============================================================================

fn fill_masked_from(canvas: &mut RasterImage, layer: &RasterImage) {
    let (bands, height, width) = canvas.data().dim();
    let (_, lh, lw) = layer.data().dim();
    if height != lh || width != lw {
        return;
    }
    let (mut data, mut mask) = canvas.views_mut();
    for y in 0..height {
        for x in 0..width {
            if !mask[[y, x]] {
                continue;
            }
            if layer.mask()[[y, x]] {
                continue;
            }
            for b in 0..bands {
                data[[b, y, x]] = layer.data()[[b, y, x]];
            }
            mask[[y, x]] = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    fn layer(data: Array3<f32>, mask: Array2<bool>) -> RasterImage {
        RasterImage::new(data, mask, None)
    }

    #[test]
    fn first_method_returns_single_layer_unchanged() {
        let img = RasterImage::from_opaque(array![[[1.0_f32, 2.0], [3.0, 4.0]]], None);
        let mut m: Box<dyn MosaicMethod> = Box::new(FirstMethod::default());
        m.feed(img);
        assert!(m.is_done(), "fully-opaque single layer short-circuits");
        let out = m.finalize().expect("a layer was fed");
        assert!((out.data()[[0, 0, 0]] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn first_method_fills_from_lower_priority_where_high_is_masked() {
        let top_data = array![[[10.0_f32, 0.0]]];
        let top_mask = array![[false, true]];
        let low = RasterImage::from_opaque(array![[[1.0_f32, 2.0]]], None);
        let mut m: Box<dyn MosaicMethod> = Box::new(FirstMethod::default());
        m.feed(layer(top_data, top_mask));
        assert!(!m.is_done(), "not done while canvas has masked pixels");
        m.feed(low);
        assert!(m.is_done(), "done after fill");
        let out = m.finalize().expect("a layer was fed");
        assert!(
            (out.data()[[0, 0, 0]] - 10.0).abs() < f32::EPSILON,
            "top wins where unmasked"
        );
        assert!(
            (out.data()[[0, 0, 1]] - 2.0).abs() < f32::EPSILON,
            "fill from low where top masked"
        );
    }

    #[test]
    fn highest_method_picks_per_pixel_max() {
        let a = RasterImage::from_opaque(array![[[1.0_f32, 5.0]]], None);
        let b = RasterImage::from_opaque(array![[[3.0_f32, 2.0]]], None);
        let mut m: Box<dyn MosaicMethod> = Box::new(HighestMethod::default());
        m.feed(a);
        m.feed(b);
        let out = m.finalize().expect("a layer was fed");
        assert!((out.data()[[0, 0, 0]] - 3.0).abs() < f32::EPSILON);
        assert!((out.data()[[0, 0, 1]] - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn lowest_method_picks_per_pixel_min() {
        let a = RasterImage::from_opaque(array![[[1.0_f32, 5.0]]], None);
        let b = RasterImage::from_opaque(array![[[3.0_f32, 2.0]]], None);
        let mut m: Box<dyn MosaicMethod> = Box::new(LowestMethod::default());
        m.feed(a);
        m.feed(b);
        let out = m.finalize().expect("a layer was fed");
        assert!((out.data()[[0, 0, 0]] - 1.0).abs() < f32::EPSILON);
        assert!((out.data()[[0, 0, 1]] - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn mean_method_averages_valid_contributions() {
        let a = RasterImage::from_opaque(array![[[10.0_f32, 20.0]]], None);
        let b = RasterImage::from_opaque(array![[[30.0_f32, 40.0]]], None);
        let mut m: Box<dyn MosaicMethod> = Box::new(MeanMethod::default());
        m.feed(a);
        m.feed(b);
        let out = m.finalize().expect("a layer was fed");
        assert!((out.data()[[0, 0, 0]] - 20.0).abs() < 0.01);
        assert!((out.data()[[0, 0, 1]] - 30.0).abs() < 0.01);
    }

    #[test]
    fn mean_method_skips_masked_in_denominator() {
        let a_data = array![[[10.0_f32, 20.0]]];
        let a_mask = array![[false, true]];
        let b = RasterImage::from_opaque(array![[[30.0_f32, 40.0]]], None);
        let mut m: Box<dyn MosaicMethod> = Box::new(MeanMethod::default());
        m.feed(layer(a_data, a_mask));
        m.feed(b);
        let out = m.finalize().expect("a layer was fed");
        assert!(
            (out.data()[[0, 0, 0]] - 20.0).abs() < 0.01,
            "avg of two valid"
        );
        assert!(
            (out.data()[[0, 0, 1]] - 40.0).abs() < 0.01,
            "avg of one valid (a masked)"
        );
    }

    #[test]
    fn median_method_returns_middle_value() {
        let a = RasterImage::from_opaque(array![[[1.0_f32]]], None);
        let b = RasterImage::from_opaque(array![[[5.0_f32]]], None);
        let c = RasterImage::from_opaque(array![[[3.0_f32]]], None);
        let mut m: Box<dyn MosaicMethod> = Box::new(MedianMethod::default());
        m.feed(a);
        m.feed(b);
        m.feed(c);
        let out = m.finalize().expect("a layer was fed");
        assert!((out.data()[[0, 0, 0]] - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn stdev_method_zero_for_constant_inputs() {
        let a = RasterImage::from_opaque(array![[[5.0_f32]]], None);
        let b = RasterImage::from_opaque(array![[[5.0_f32]]], None);
        let mut m: Box<dyn MosaicMethod> = Box::new(StdevMethod::default());
        m.feed(a);
        m.feed(b);
        let out = m.finalize().expect("a layer was fed");
        assert!(out.data()[[0, 0, 0]].abs() < 0.001);
    }

    #[test]
    fn count_method_records_valid_contributions() {
        let a_data = array![[[1.0_f32, 2.0]]];
        let a_mask = array![[false, true]];
        let b = RasterImage::from_opaque(array![[[10.0_f32, 20.0]]], None);
        let mut m: Box<dyn MosaicMethod> = Box::new(CountMethod::default());
        m.feed(layer(a_data, a_mask));
        m.feed(b);
        let out = m.finalize().expect("a layer was fed");
        assert!((out.data()[[0, 0, 0]] - 2.0).abs() < f32::EPSILON);
        assert!((out.data()[[0, 0, 1]] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn build_returns_correct_impl_per_variant() {
        for variant in [
            PixelSelectionMethod::First,
            PixelSelectionMethod::Highest,
            PixelSelectionMethod::Lowest,
            PixelSelectionMethod::Mean,
            PixelSelectionMethod::Median,
            PixelSelectionMethod::Stdev,
            PixelSelectionMethod::Count,
            PixelSelectionMethod::LowestCloudCover,
        ] {
            let mut m = build(variant);
            assert!(
                build(variant).finalize().is_none(),
                "{variant:?}: nothing fed yields None"
            );
            m.feed(RasterImage::from_opaque(array![[[2.0_f32]]], None));
            let out = m.finalize().expect("a layer was fed");
            let expected = match variant {
                PixelSelectionMethod::Stdev => 0.0,
                PixelSelectionMethod::Count => 1.0,
                _ => 2.0,
            };
            assert!(
                (out.data()[[0, 0, 0]] - expected).abs() < f32::EPSILON,
                "{variant:?}: got {}",
                out.data()[[0, 0, 0]]
            );
        }
    }
}
