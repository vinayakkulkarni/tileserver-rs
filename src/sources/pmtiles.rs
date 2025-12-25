use crate::config::SourceConfig;
use crate::error::{Result, TileServerError};

pub mod http;

/// PMTiles tile source (local file via memory-mapped I/O)
///
/// Note: Local file support requires the `mmap-async-tokio` feature in pmtiles crate.
/// Currently only HTTP PMTiles sources are supported.
pub struct PmTilesSource;

impl PmTilesSource {
    /// Create a new PMTiles source from a local file
    ///
    /// Note: Local file support is not yet implemented.
    /// Use HTTP URLs for PMTiles sources.
    pub async fn from_file(config: &SourceConfig) -> Result<http::HttpPmTilesSource> {
        // For now, we don't support local files - only HTTP
        // To add local file support, enable the `mmap-async-tokio` feature in pmtiles crate
        Err(TileServerError::ConfigError(format!(
            "Local PMTiles files not supported yet. Use HTTP URL instead. Path: {}",
            config.path
        )))
    }
}
