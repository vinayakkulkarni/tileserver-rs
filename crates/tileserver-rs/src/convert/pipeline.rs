//! Conversion pipeline orchestrator: read input, build tiles, write PMTiles.

use crate::convert::args::ConvertArgs;
use crate::error::Result;

/// Execute the end-to-end conversion described by `args`.
///
/// # Errors
///
/// Returns an error on any pipeline failure.
pub fn run(_args: ConvertArgs) -> Result<()> {
    Err(crate::error::TileServerError::Internal(anyhow::anyhow!(
        "convert pipeline not yet implemented"
    )))
}
