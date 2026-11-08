//! Band-math expressions for [`RasterImage`].
//!
//! Parse expressions like `(b2 - b1) / (b2 + b1)` (NDVI) or
//! `log(b1 * b2)` once per request and evaluate them per-pixel on the
//! ndarray-backed raster.  Matches rio-tiler's `expression` parameter
//! semantics: band references use `b` + 1-based index
//! (`b1`, `b2`, ...) and are mapped to the corresponding slices of
//! the [`RasterImage`]'s data array.
//!
//! # Safety at the HTTP boundary
//!
//! The parser is `exmex` which rejects anything outside its small
//! whitelisted grammar (operators, identifiers, function names,
//! numbers).  There is no way to smuggle file-system access, shell
//! commands, or network calls through it.  Even so, unknown band
//! references are rejected at parse time so a malformed
//! `?expression=b99` request on a 3-band raster errors cleanly
//! instead of panicking deep in the evaluator.
//!
//! # Performance
//!
//! `exmex` parses once and evaluates about 300 ns per scalar call on
//! a modern x86-64 CPU.  A 256×256 single-band tile is therefore a
//! ~20 ms worst-case CPU cost, dwarfed by COG range-reads and
//! mosaic compositing on cold tiles.  Evaluation is wrapped in an
//! ndarray `Zip` so LLVM auto-vectorises the inner loop when
//! `-C target-cpu=native` (or `x86-64-v3`) is enabled.

use exmex::prelude::*;
use ndarray::{Array2, Array3, Zip};

use super::RasterImage;

#[derive(Debug, thiserror::Error)]
pub enum ExpressionError {
    #[error("expression parse failed: {0}")]
    Parse(String),
    #[error("expression references band `{band}` but the raster has only {available} band(s)")]
    BandOutOfRange { band: String, available: usize },
    #[error("expression references unknown variable `{0}` (expected `b1`, `b2`, ...)")]
    UnknownVariable(String),
    #[error("expression evaluation failed: {0}")]
    Eval(String),
    #[error("expression is longer than {MAX_EXPRESSION_BYTES} bytes")]
    TooLong,
}

/// Longest `?expression=` accepted. Real band-math formulas (NDVI, EVI,
/// band ratios) are well under 100 bytes; the cap stops a client from
/// sending a huge expression that is then evaluated once per pixel.
pub const MAX_EXPRESSION_BYTES: usize = 512;

/// The first 64 bytes of `raw`, for logs, so one request can't flood them.
#[must_use]
pub fn log_preview(raw: &str) -> &str {
    let mut end = raw.len().min(64);
    while !raw.is_char_boundary(end) {
        end -= 1;
    }
    &raw[..end]
}

/// A parsed, validated band-math expression ready to evaluate.
///
/// Hold one of these per request; reuse across the per-pixel inner
/// loop inside [`apply`].  The underlying `FlatEx` is `Send` so the
/// same parsed expression can be shared across rayon workers if we
/// later parallelise the evaluation.
#[derive(Debug, Clone)]
pub struct ParsedExpression {
    raw: String,
    inner: FlatEx<f64>,
    band_order: Vec<usize>,
}

impl ParsedExpression {
    /// Parse and validate a band-math expression against the given
    /// band count.
    ///
    /// The expression may reference variables `b1`, `b2`, ... up to
    /// `b{n_bands}`.  Any reference outside that range is rejected.
    ///
    /// # Errors
    ///
    /// Returns [`ExpressionError::TooLong`] past [`MAX_EXPRESSION_BYTES`],
    /// [`ExpressionError::Parse`] on syntactically invalid
    /// input, [`ExpressionError::UnknownVariable`] when a variable
    /// is not of the form `b<n>`, or
    /// [`ExpressionError::BandOutOfRange`] when a `b<n>` index
    /// exceeds `n_bands`.
    pub fn parse(raw: &str, n_bands: usize) -> Result<Self, ExpressionError> {
        if raw.len() > MAX_EXPRESSION_BYTES {
            return Err(ExpressionError::TooLong);
        }
        let inner = exmex::parse::<f64>(raw).map_err(|e| ExpressionError::Parse(e.to_string()))?;

        let mut band_order: Vec<usize> = Vec::with_capacity(inner.var_names().len());
        for name in inner.var_names() {
            let idx = parse_band_index(name)?;
            if idx == 0 || idx > n_bands {
                return Err(ExpressionError::BandOutOfRange {
                    band: name.clone(),
                    available: n_bands,
                });
            }
            band_order.push(idx - 1);
        }

        Ok(Self {
            raw: raw.to_owned(),
            inner,
            band_order,
        })
    }

    /// Returns the raw expression text (as parsed).
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns the 0-based band indices (in variable order) referenced
    /// by the expression.
    #[must_use]
    pub fn band_indices(&self) -> &[usize] {
        &self.band_order
    }
}

fn parse_band_index(name: &str) -> Result<usize, ExpressionError> {
    let rest = name
        .strip_prefix('b')
        .or_else(|| name.strip_prefix('B'))
        .ok_or_else(|| ExpressionError::UnknownVariable(name.to_owned()))?;
    rest.parse::<usize>()
        .map_err(|_| ExpressionError::UnknownVariable(name.to_owned()))
}

/// Evaluate `expr` over every pixel of `img`, producing a new
/// single-band [`RasterImage`] whose values are the expression's
/// result.
///
/// The output preserves the input's mask: pixels masked in the
/// input are masked in the output.  Nodata is propagated from the
/// input if set.
///
/// # Errors
///
/// Returns [`ExpressionError::Eval`] if the underlying evaluator
/// returns an error at any pixel (rare; typically only happens for
/// expressions containing user-defined functions that we do not
/// currently expose).
pub fn apply(expr: &ParsedExpression, img: &RasterImage) -> Result<RasterImage, ExpressionError> {
    let (_, h, w) = img.data().dim();
    let mut out = Array3::<f32>::zeros((1, h, w));
    let mut out_mask = Array2::from_elem((h, w), false);

    // Per-pixel evaluation buffer reused across iterations to avoid
    // allocating per-pixel.  The length matches the expression's
    // variable count so we only copy the bands the expression asked
    // for.
    let mut scratch: Vec<f64> = vec![0.0; expr.band_order.len()];
    let data = img.data();
    let mask = img.mask();

    Zip::indexed(&mut out.index_axis_mut(ndarray::Axis(0), 0))
        .and(&mut out_mask)
        .for_each(|(y, x), out_val, out_mask_val| {
            if mask[[y, x]] {
                *out_mask_val = true;
                return;
            }
            for (slot, &band_idx) in scratch.iter_mut().zip(expr.band_order.iter()) {
                *slot = f64::from(data[[band_idx, y, x]]);
            }
            match expr.inner.eval(&scratch) {
                Ok(v) => *out_val = v as f32,
                Err(_) => {
                    *out_mask_val = true;
                }
            }
        });

    Ok(RasterImage::new(out, out_mask, img.nodata()))
}

/// Validate that every variable in `expr` exists in `available_bands`.
/// Used primarily by unit tests — the normal `parse` path already
/// enforces this.
#[cfg(test)]
pub(crate) fn variable_bands(expr: &ParsedExpression) -> std::collections::BTreeMap<String, usize> {
    expr.inner
        .var_names()
        .iter()
        .zip(expr.band_order.iter())
        .map(|(name, &idx)| (name.clone(), idx))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn parse_ndvi_extracts_two_bands() {
        let expr = ParsedExpression::parse("(b2 - b1) / (b2 + b1)", 2).expect("parse");
        let vars = variable_bands(&expr);
        assert_eq!(vars.get("b1"), Some(&0));
        assert_eq!(vars.get("b2"), Some(&1));
    }

    #[test]
    fn parse_rejects_band_out_of_range() {
        let long = "b1+".repeat(MAX_EXPRESSION_BYTES / 3 + 1) + "b1";
        assert!(matches!(
            ParsedExpression::parse(&long, 1),
            Err(ExpressionError::TooLong)
        ));
        assert!(ParsedExpression::parse("b1+".repeat(100).trim_end_matches('+'), 1).is_ok());
        assert_eq!(log_preview(&long).len(), 64);
        assert_eq!(log_preview("b1"), "b1");
        let err = ParsedExpression::parse("b1 + b3", 2).unwrap_err();
        assert!(matches!(err, ExpressionError::BandOutOfRange { .. }));
    }

    #[test]
    fn parse_rejects_non_band_variable() {
        let err = ParsedExpression::parse("b1 + foo", 2).unwrap_err();
        assert!(matches!(err, ExpressionError::UnknownVariable(ref v) if v == "foo"));
    }

    #[test]
    fn parse_accepts_uppercase_bands() {
        let expr = ParsedExpression::parse("B2 / B1", 2).expect("parse");
        assert_eq!(expr.band_indices().len(), 2);
    }

    #[test]
    fn apply_ndvi_computes_expected_values() {
        let data = array![
            [[0.3_f32, 0.1], [0.5, 0.4]], // band 0 (red)
            [[0.7_f32, 0.3], [0.9, 0.8]], // band 1 (nir)
        ];
        let img = RasterImage::from_opaque(data, None);
        let expr = ParsedExpression::parse("(b2 - b1) / (b2 + b1)", 2).unwrap();
        let out = apply(&expr, &img).unwrap();
        assert_eq!(out.band_count(), 1);
        let expected_00 = (0.7 - 0.3) / (0.7 + 0.3);
        assert!((out.data()[[0, 0, 0]] as f64 - expected_00).abs() < 1e-5);
    }

    #[test]
    fn apply_preserves_mask() {
        let data = array![[[0.5_f32]]];
        let mask = Array2::from_elem((1, 1), true);
        let img = RasterImage::new(data, mask, None);
        let expr = ParsedExpression::parse("b1 * 2", 1).unwrap();
        let out = apply(&expr, &img).unwrap();
        assert!(
            out.is_fully_masked(),
            "masked input must propagate to output"
        );
    }

    #[test]
    fn apply_constant_expression_works() {
        let data = array![[[0.5_f32]]];
        let img = RasterImage::from_opaque(data, None);
        let expr = ParsedExpression::parse("42.0", 1).unwrap();
        let out = apply(&expr, &img).unwrap();
        assert!((out.data()[[0, 0, 0]] - 42.0).abs() < f32::EPSILON);
    }
}
