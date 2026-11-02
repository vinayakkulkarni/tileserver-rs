//! Mosaic compositing for multi-asset raster sources (e.g., STAC).
//!
//! This module provides the [`MosaicMethod`] trait and its 8 implementations,
//! one per variant of [`crate::config::PixelSelectionMethod`].  Each
//! method consumes a stream of [`crate::raster::RasterImage`] layers and
//! produces a single composited [`crate::raster::RasterImage`] following
//! the method's combination rule (first-wins, per-pixel max/min, mean,
//! median, stdev, count, lowest-cloud-cover).
//!
//! # Design
//!
//! The trait mirrors rio-tiler's `MosaicMethodBase.feed() / is_done /
//! data` contract so operators moving from titiler see identical
//! semantics:
//!
//! - [`MosaicMethod::feed`] is called once per successfully decoded
//!   layer, in priority order (first layer is highest-priority).
//! - [`MosaicMethod::is_done`] returns true once the method is
//!   short-circuit-ready (e.g., the canvas is fully opaque for
//!   [`FirstMethod`]).  The mosaic loop polls this after each feed
//!   and stops early when true.
//! - [`MosaicMethod::finalize`] consumes the method and returns the
//!   final [`RasterImage`](crate::raster::RasterImage) ready for encoding.
//!
//! # Alpha handling
//!
//! All methods respect the `mask` channel on each input layer: masked
//! pixels (the source marked them as nodata) are excluded from the
//! per-pixel reduction.  Statistical methods (`mean`, `median`,
//! `stdev`) skip masked contributions so partially-covered tiles do
//! not pull the result toward zero.

mod methods;

pub use methods::{
    CountMethod, FirstMethod, HighestMethod, LowestCloudCoverMethod, LowestMethod, MeanMethod,
    MedianMethod, MosaicMethod, StdevMethod, build,
};
