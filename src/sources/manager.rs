use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "postgres")]
use crate::config::PostgresConfig;
use crate::config::{SourceConfig, SourceType};
use crate::error::{Result, TileServerError};
use crate::sources::mbtiles::MbTilesSource;
use crate::sources::pmtiles::http::HttpPmTilesSource;
use crate::sources::pmtiles::local::LocalPmTilesSource;
#[cfg(feature = "postgres")]
use crate::sources::postgres::{
    PoolSettings, PostgresFunctionSource, PostgresPool, PostgresTableSource, TileCache,
};
use crate::sources::{TileMetadata, TileSource};

pub struct SourceManager {
    sources: HashMap<String, Arc<dyn TileSource>>,
    #[cfg(feature = "postgres")]
    postgres_pool: Option<Arc<PostgresPool>>,
    #[cfg(feature = "postgres")]
    tile_cache: Option<Arc<TileCache>>,
}

impl SourceManager {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            #[cfg(feature = "postgres")]
            postgres_pool: None,
            #[cfg(feature = "postgres")]
            tile_cache: None,
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

    /// Load sources from configuration including PostgreSQL sources
    #[cfg(feature = "postgres")]
    pub async fn from_configs_with_postgres(
        configs: &[SourceConfig],
        postgres_config: Option<&PostgresConfig>,
    ) -> Result<Self> {
        let mut manager = Self::from_configs(configs).await?;

        if let Some(pg_config) = postgres_config {
            manager.load_postgres_sources(pg_config).await?;
        }

        Ok(manager)
    }

    #[cfg(feature = "postgres")]
    pub async fn load_postgres_sources(&mut self, config: &PostgresConfig) -> Result<()> {
        let pool_settings = PoolSettings {
            max_size: config.pool_size,
            wait_timeout_ms: config.pool_wait_timeout_ms,
            create_timeout_ms: config.pool_create_timeout_ms,
            recycle_timeout_ms: config.pool_recycle_timeout_ms,
        };

        let pool = Arc::new(
            PostgresPool::new(
                &config.connection_string,
                pool_settings,
                config.ssl_cert.as_ref(),
                config.ssl_key.as_ref(),
                config.ssl_root_cert.as_ref(),
            )
            .await?,
        );

        self.postgres_pool = Some(pool.clone());

        let tile_cache = config.cache.as_ref().map(|cache_config| {
            let cache = Arc::new(TileCache::new(
                cache_config.size_mb,
                cache_config.ttl_seconds,
            ));
            tracing::info!(
                "Initialized PostgreSQL tile cache: {}MB, TTL {}s",
                cache_config.size_mb,
                cache_config.ttl_seconds
            );
            cache
        });
        self.tile_cache = tile_cache.clone();

        for func_config in &config.functions {
            match PostgresFunctionSource::new(pool.clone(), func_config, tile_cache.clone()).await {
                Ok(source) => {
                    tracing::info!(
                        "Loaded PostgreSQL function source: {} ({}.{})",
                        func_config.id,
                        func_config.schema,
                        func_config.function
                    );
                    self.sources
                        .insert(func_config.id.clone(), Arc::new(source));
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to load PostgreSQL function source {}: {}",
                        func_config.id,
                        e
                    );
                }
            }
        }

        for table_config in &config.tables {
            match PostgresTableSource::new(pool.clone(), table_config, tile_cache.clone()).await {
                Ok(source) => {
                    tracing::info!(
                        "Loaded PostgreSQL table source: {} ({}.{})",
                        table_config.id,
                        table_config.schema,
                        table_config.table
                    );
                    self.sources
                        .insert(table_config.id.clone(), Arc::new(source));
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to load PostgreSQL table source {}: {}",
                        table_config.id,
                        e
                    );
                }
            }
        }

        Ok(())
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
            #[cfg(feature = "postgres")]
            SourceType::Postgres => {
                // PostgreSQL sources are loaded separately via load_postgres_sources
                return Err(TileServerError::ConfigError(
                    "PostgreSQL sources should be configured in the [postgres] section, not as regular sources".to_string(),
                ));
            }
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
