//! Conversion pipeline orchestrator: read input, build tiles, write PMTiles.

use crate::convert::args::ConvertArgs;
use crate::error::{Result, TileServerError};

/// Execute the end-to-end conversion described by `args`.
///
/// # Errors
///
/// Returns [`TileServerError::ConvertError`] on any pipeline failure.
pub fn run(_args: ConvertArgs) -> Result<()> {
    Err(TileServerError::ConvertError(
        "convert pipeline not yet implemented".to_string(),
    ))
}
