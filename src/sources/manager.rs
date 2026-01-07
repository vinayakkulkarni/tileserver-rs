use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "postgres")]
use crate::config::PostgresConfig;
#[cfg(feature = "raster")]
use crate::config::ResamplingMethod;
use crate::config::{SourceConfig, SourceType};
use crate::error::{Result, TileServerError};
#[cfg(feature = "raster")]
use crate::sources::cog::CogSource;
use crate::sources::mbtiles::MbTilesSource;
use crate::sources::pmtiles::http::HttpPmTilesSource;
use crate::sources::pmtiles::local::LocalPmTilesSource;
#[cfg(all(feature = "postgres", feature = "raster"))]
use crate::sources::postgres::PostgresOutDbRasterSource;
#[cfg(feature = "postgres")]
use crate::sources::postgres::{
    PoolSettings, PostgresFunctionSource, PostgresPool, PostgresTableSource, TileCache,
};
use crate::sources::{TileMetadata, TileSource};
#[cfg(feature = "postgres")]
use tokio_postgres::types::Type;

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
            pre_warm: config.pool_pre_warm,
        };

        let pool = Arc::new(
            PostgresPool::new(
                &config.connection_string,
                pool_settings.clone(),
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

        let mut function_sources: Vec<PostgresFunctionSource> = Vec::new();
        for func_config in &config.functions {
            match PostgresFunctionSource::new(pool.clone(), func_config, tile_cache.clone()).await {
                Ok(source) => {
                    tracing::info!(
                        "Loaded PostgreSQL function source: {} ({}.{})",
                        func_config.id,
                        func_config.schema,
                        func_config.function
                    );
                    function_sources.push(source);
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

        let mut table_sources: Vec<PostgresTableSource> = Vec::new();
        for table_config in &config.tables {
            match PostgresTableSource::new(pool.clone(), table_config, tile_cache.clone()).await {
                Ok(source) => {
                    tracing::info!(
                        "Loaded PostgreSQL table source: {} ({}.{})",
                        table_config.id,
                        table_config.schema,
                        table_config.table
                    );
                    table_sources.push(source);
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

        #[cfg(feature = "raster")]
        let mut outdb_raster_sources: Vec<PostgresOutDbRasterSource> = Vec::new();
        #[cfg(feature = "raster")]
        for outdb_config in &config.outdb_rasters {
            match PostgresOutDbRasterSource::new(pool.clone(), outdb_config).await {
                Ok(source) => {
                    tracing::info!(
                        "Loaded PostgreSQL out-db raster source: {} ({}.{})",
                        outdb_config.id,
                        outdb_config.schema,
                        outdb_config.function.as_ref().unwrap_or(&outdb_config.id)
                    );
                    outdb_raster_sources.push(source);
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to load PostgreSQL out-db raster source {}: {}",
                        outdb_config.id,
                        e
                    );
                }
            }
        }

        if pool_settings.pre_warm {
            let tile_param_types: &[Type] = &[Type::INT4, Type::INT4, Type::INT4];
            let mut queries: Vec<(&str, &[Type])> = Vec::new();

            for source in &function_sources {
                queries.push((source.tile_query(), tile_param_types));
            }
            for source in &table_sources {
                queries.push((source.tile_query(), tile_param_types));
            }
            #[cfg(feature = "raster")]
            for source in &outdb_raster_sources {
                let outdb_param_types: &[Type] =
                    &[Type::INT4, Type::INT4, Type::INT4, Type::TEXT, Type::JSONB];
                queries.push((source.tile_query(), outdb_param_types));
            }

            pool.warmup(&queries).await?;
        }

        for source in function_sources {
            self.sources
                .insert(source.metadata().id.clone(), Arc::new(source));
        }
        for source in table_sources {
            self.sources
                .insert(source.metadata().id.clone(), Arc::new(source));
        }
        #[cfg(feature = "raster")]
        for source in outdb_raster_sources {
            self.sources
                .insert(source.metadata().id.clone(), Arc::new(source));
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
                return Err(TileServerError::ConfigError(
                    "PostgreSQL sources should be configured in the [postgres] section, not as regular sources".to_string(),
                ));
            }
            #[cfg(feature = "raster")]
            SourceType::Cog | SourceType::Vrt => Arc::new(CogSource::from_file(config).await?),
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

    #[cfg(feature = "raster")]
    pub async fn get_raster_tile(
        &self,
        id: &str,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
        resampling: Option<ResamplingMethod>,
    ) -> crate::error::Result<Option<crate::sources::TileData>> {
        self.get_raster_tile_with_params(id, z, x, y, tile_size, resampling, None)
            .await
    }

    #[cfg(feature = "raster")]
    #[allow(clippy::too_many_arguments)]
    pub async fn get_raster_tile_with_params(
        &self,
        id: &str,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
        resampling: Option<ResamplingMethod>,
        query_params: Option<serde_json::Value>,
    ) -> crate::error::Result<Option<crate::sources::TileData>> {
        let source = self
            .sources
            .get(id)
            .ok_or_else(|| TileServerError::SourceNotFound(id.to_string()))?;

        if let Some(cog) = source.as_ref().as_any().downcast_ref::<CogSource>() {
            let resample = resampling.unwrap_or(cog.resampling());
            cog.get_tile_with_resampling(z, x, y, tile_size, resample)
                .await
        } else if let Some(outdb) = source
            .as_ref()
            .as_any()
            .downcast_ref::<PostgresOutDbRasterSource>()
        {
            outdb
                .get_tile_with_params(z, x, y, tile_size, resampling, query_params)
                .await
        } else {
            source.get_tile(z, x, y).await
        }
    }

    #[cfg(all(feature = "postgres", feature = "raster"))]
    pub fn is_outdb_raster_source(&self, id: &str) -> bool {
        self.sources
            .get(id)
            .map(|s| {
                s.as_ref()
                    .as_any()
                    .downcast_ref::<PostgresOutDbRasterSource>()
                    .is_some()
            })
            .unwrap_or(false)
    }
}

impl Default for SourceManager {
    fn default() -> Self {
        Self::new()
    }
}
