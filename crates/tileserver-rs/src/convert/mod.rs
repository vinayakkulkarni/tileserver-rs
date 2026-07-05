//! Built-in GeoJSON/CSV → PMTiles conversion pipeline (`tileserver-rs convert`).
//!
//! A pure-Rust pipeline: `geozero` reads feature streams, [`tile_builder`]
//! partitions features into Web-Mercator tiles and encodes MVT, and the
//! `pmtiles` writer emits a single Hilbert-ordered archive. Gated behind the
//! opt-in `convert` cargo feature so default builds stay slim.
//!
//! For planet-scale tile generation (OSM PBF, Overture Maps), see
//! <https://github.com/geolith/geolith>.

pub mod args;
pub mod input;
pub mod pipeline;
pub mod progress;
pub mod simplify;
pub mod tile_builder;

pub use args::ConvertArgs;

use crate::error::Result;

/// Run the conversion pipeline for the given CLI arguments.
///
/// # Errors
///
/// Returns [`crate::error::TileServerError::ConvertError`] when input parsing,
/// tile building, or PMTiles writing fails.
pub fn run(args: ConvertArgs) -> Result<()> {
    pipeline::run(args)
}
