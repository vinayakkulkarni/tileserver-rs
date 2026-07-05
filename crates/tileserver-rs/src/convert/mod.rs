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
pub mod serve;
pub mod simplify;
pub mod tile_builder;

pub use args::{ConvertArgs, LONG_ABOUT};

use crate::error::{Result, TileServerError};
use std::path::PathBuf;

/// Run the conversion pipeline for the given CLI arguments (no `--serve`).
///
/// # Errors
///
/// Returns [`TileServerError::ConvertError`] when input parsing, tile building,
/// or PMTiles writing fails.
pub fn run(args: ConvertArgs) -> Result<()> {
    pipeline::run(args)
}

/// Resolve the output path for a conversion. When `--serve` is set without an
/// explicit `--output`, a unique path under the system temp dir is used.
///
/// # Errors
///
/// Returns [`TileServerError::ConvertError`] when the temp directory cannot be
/// created.
pub fn resolve_output_path(args: &ConvertArgs) -> Result<PathBuf> {
    if let Some(output) = args.output.clone() {
        return Ok(output);
    }
    let dir = std::env::temp_dir().join("tileserver-rs").join("convert");
    std::fs::create_dir_all(&dir)
        .map_err(|e| TileServerError::ConvertError(format!("create temp dir: {e}")))?;
    let stem = pipeline::default_layer_name(&args.input);
    Ok(dir.join(format!("{stem}-{}.pmtiles", uuid::Uuid::new_v4())))
}

/// Run the conversion and, when `--serve` is set, boot the HTTP server against
/// the result. This is the async entry used by the CLI dispatcher.
///
/// # Errors
///
/// Returns an error when conversion or serving fails.
pub async fn run_and_maybe_serve(args: ConvertArgs) -> anyhow::Result<()> {
    let output = resolve_output_path(&args)?;
    pipeline::convert_to_pmtiles(&args, &output)?;
    tracing::info!("wrote {}", output.display());

    if args.serve {
        let source_id = args
            .layer_name
            .clone()
            .unwrap_or_else(|| pipeline::default_layer_name(&args.input));
        let port = args.port.unwrap_or(serve::DEFAULT_SERVE_PORT);
        serve::serve_pmtiles(&output, &source_id, "127.0.0.1", port).await?;
    }
    Ok(())
}
