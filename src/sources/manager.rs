use std::collections::HashMap;
use std::sync::Arc;

use crate::config::{SourceConfig, SourceType};
use crate::error::{Result, TileServerError};
use crate::sources::mbtiles::MbTilesSource;
use crate::sources::pmtiles::http::HttpPmTilesSource;
use crate::sources::pmtiles::local::LocalPmTilesSource;
use crate::sources::{TileMetadata, TileSource};

/// Manages all tile sources
pub struct SourceManager {
    sources: HashMap<String, Arc<dyn TileSource>>,
}

impl SourceManager {
    /// Create a new empty source manager
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    /// Load sources from configuration
    pub async fn from_configs(configs: &[SourceConfig]) -> Result<Self> {
        let mut manager = Self::new();

        for config in configs {
            match manager.load_source(config).await {
                Ok(_) => {
                    tracing::info!("Loaded source: {} ({})", config.id, config.path);
                }
                Err(e) => {
                    tracing::error!("Failed to load source {}: {}", config.id, e);
                    // Continue loading other sources
                }
            }
        }

        Ok(manager)
    }

    /// Load a single source from config
    pub async fn load_source(&mut self, config: &SourceConfig) -> Result<()> {
        let source: Arc<dyn TileSource> = match config.source_type {
            SourceType::PMTiles => {
                // Check if it's a URL or local file
                if config.path.starts_with("http://") || config.path.starts_with("https://") {
                    let client = reqwest::Client::builder()
                        .user_agent("tileserver-rs/0.1.0")
                        .build()
                        .map_err(|e| {
                            TileServerError::ConfigError(format!(
                                "Failed to create HTTP client: {}",
                                e
                            ))
                        })?;
                    Arc::new(HttpPmTilesSource::from_url(config, client).await?)
                } else if config.path.starts_with("s3://") {
                    // S3 support placeholder - would require aws-sdk-s3
                    return Err(TileServerError::ConfigError(
                        "S3 PMTiles support not yet implemented".to_string(),
                    ));
                } else {
                    // Local PMTiles file using memory-mapped I/O
                    Arc::new(LocalPmTilesSource::from_file(config).await?)
                }
            }
            SourceType::MBTiles => Arc::new(MbTilesSource::from_file(config).await?),
        };

        self.sources.insert(config.id.clone(), source);
        Ok(())
    }

    /// Get a source by ID
    pub fn get(&self, id: &str) -> Option<&Arc<dyn TileSource>> {
        self.sources.get(id)
    }

    /// Get all source IDs
    pub fn ids(&self) -> Vec<&String> {
        self.sources.keys().collect()
    }

    /// Get metadata for all sources
    pub fn all_metadata(&self) -> Vec<&TileMetadata> {
        self.sources.values().map(|s| s.metadata()).collect()
    }

    /// Check if a source exists
    pub fn exists(&self, id: &str) -> bool {
        self.sources.contains_key(id)
    }

    /// Get the number of sources
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Check if there are no sources
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

impl Default for SourceManager {
    fn default() -> Self {
        Self::new()
    }
}
