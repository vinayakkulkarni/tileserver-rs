//! CLI argument struct for `tileserver-rs convert`.

use std::path::PathBuf;

/// Arguments accepted by the `convert` subcommand.
#[derive(Debug, Clone)]
pub struct ConvertArgs {
    /// Input file (`.geojson`, `.json`, `.csv`).
    pub input: PathBuf,
}
