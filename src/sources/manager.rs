//! Source manager for loading, querying, and hot-reloading tile sources.

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
#[cfg(feature = "duckdb")]
use crate::sources::duckdb::DuckDbSource;
#[cfg(feature = "geoparquet")]
use crate::sources::geoparquet::GeoParquetSource;
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
    global_cache: Option<Arc<crate::cache::TileCache>>,
}

impl SourceManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            #[cfg(feature = "postgres")]
            postgres_pool: None,
            #[cfg(feature = "postgres")]
            tile_cache: None,
            global_cache: None,
        }
    }

    /// Attach a global tile cache. All non-postgres `get_tile` calls will use it.
    #[must_use]
    pub fn with_cache(mut self, cache: Arc<crate::cache::TileCache>) -> Self {
        self.global_cache = Some(cache);
        self
    }

    /// Reference to the global cache, if configured (used by admin flush endpoint).
    #[must_use]
    pub fn cache(&self) -> Option<&Arc<crate::cache::TileCache>> {
        self.global_cache.as_ref()
    }

    /// Get a vector tile, checking the global cache first when enabled.
    pub async fn get_tile(
        &self,
        id: &str,
        z: u8,
        x: u32,
        y: u32,
    ) -> crate::error::Result<Option<crate::sources::TileData>> {
        let source = self
            .sources
            .get(id)
            .ok_or_else(|| TileServerError::SourceNotFound(id.to_string()))?;

        if let Some(cache) = &self.global_cache {
            let key = crate::cache::TileCacheKey {
                source_id: id.into(),
                z,
                x,
                y,
            };
            if let Some(cached) = cache.get(&key).await {
                return Ok(Some(cached));
            }
            let result = source.get_tile(z, x, y).await?;
            if let Some(ref tile) = result {
                cache.insert(key, tile.clone()).await;
            }
            Ok(result)
        } else {
            source.get_tile(z, x, y).await
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

        let mut function_sources: Vec<PostgresFunctionSource> =
            Vec::with_capacity(config.functions.len());
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

        let mut table_sources: Vec<PostgresTableSource> = Vec::with_capacity(config.tables.len());
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
        let mut outdb_raster_sources: Vec<PostgresOutDbRasterSource> =
            Vec::with_capacity(config.outdb_rasters.len());
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
                #[cfg(feature = "cloud")]
                if crate::sources::pmtiles::cloud::is_cloud_url(&config.path) {
                    let source = Arc::new(
                        crate::sources::pmtiles::cloud::CloudPmTilesSource::from_url(config)
                            .await?,
                    );
                    self.sources.insert(config.id.clone(), source);
                    return Ok(());
                }

                if config.path.starts_with("http://") || config.path.starts_with("https://") {
                    let client = reqwest::Client::builder()
                        .user_agent("tileserver-rs/0.1.0")
                        .build()
                        .map_err(|e| {
                            TileServerError::ConfigError(format!(
                                "failed to create HTTP client: {e}"
                            ))
                        })?;
                    Arc::new(HttpPmTilesSource::from_url(config, client).await?)
                } else {
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
            #[cfg(feature = "geoparquet")]
            SourceType::GeoParquet => Arc::new(GeoParquetSource::from_config(config).await?),
            #[cfg(feature = "duckdb")]
            SourceType::DuckDB => Arc::new(DuckDbSource::from_config(config).await?),
        };

        self.sources.insert(config.id.clone(), source);
        Ok(())
    }

    /// Get a source by ID
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Arc<dyn TileSource>> {
        self.sources.get(id)
    }

    /// Get all source IDs
    #[must_use]
    pub fn ids(&self) -> Vec<&str> {
        self.sources.keys().map(|s| s.as_str()).collect()
    }

    /// Get metadata for all sources
    #[must_use]
    pub fn all_metadata(&self) -> Vec<&TileMetadata> {
        self.sources.values().map(|s| s.metadata()).collect()
    }

    /// Check if a source exists
    #[must_use]
    pub fn exists(&self, id: &str) -> bool {
        self.sources.contains_key(id)
    }

    /// Get the number of sources
    #[must_use]
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Check if there are no sources
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Clone the internal sources map (values are `Arc`, cheap to clone)
    #[must_use]
    pub fn clone_sources(&self) -> HashMap<String, Arc<dyn TileSource>> {
        self.sources.clone()
    }

    /// Create a SourceManager from an existing sources map
    #[must_use]
    pub fn from_sources(sources: HashMap<String, Arc<dyn TileSource>>) -> Self {
        Self {
            sources,
            #[cfg(feature = "postgres")]
            postgres_pool: None,
            #[cfg(feature = "postgres")]
            tile_cache: None,
            global_cache: None,
        }
    }

    /// Remove a source by ID. Returns `true` if it existed.
    pub fn remove_source(&mut self, id: &str) -> bool {
        self.sources.remove(id).is_some()
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
    #[must_use]
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

    #[cfg(feature = "postgres")]
    pub async fn get_vector_tile_with_query_params(
        &self,
        id: &str,
        z: u8,
        x: u32,
        y: u32,
        query_params: &serde_json::Value,
    ) -> crate::error::Result<Option<crate::sources::TileData>> {
        let source = self
            .sources
            .get(id)
            .ok_or_else(|| TileServerError::SourceNotFound(id.to_string()))?;

        if let Some(pg_func) = source
            .as_ref()
            .as_any()
            .downcast_ref::<PostgresFunctionSource>()
        {
            pg_func
                .get_tile_with_query_params(z, x, y, query_params)
                .await
        } else {
            source.get_tile(z, x, y).await
        }
    }

    #[cfg(feature = "postgres")]
    #[must_use]
    pub fn is_postgres_function_source(&self, id: &str) -> bool {
        self.sources
            .get(id)
            .map(|s| {
                s.as_ref()
                    .as_any()
                    .downcast_ref::<PostgresFunctionSource>()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};
    use async_trait::async_trait;
    use bytes::Bytes;

    /// A trivial in-memory tile source for unit tests.
    struct MockSource {
        meta: TileMetadata,
        tile: Option<TileData>,
    }

    impl MockSource {
        fn new(id: &str) -> Self {
            Self {
                meta: TileMetadata {
                    id: id.to_string(),
                    name: id.to_string(),
                    description: None,
                    attribution: None,
                    format: TileFormat::Pbf,
                    minzoom: 0,
                    maxzoom: 14,
                    bounds: None,
                    center: None,
                    vector_layers: None,
                },
                tile: Some(TileData {
                    data: Bytes::from_static(b"mock-tile-data"),
                    format: TileFormat::Pbf,
                    compression: TileCompression::None,
                }),
            }
        }
    }

    #[async_trait]
    impl TileSource for MockSource {
        async fn get_tile(
            &self,
            _z: u8,
            _x: u32,
            _y: u32,
        ) -> crate::error::Result<Option<TileData>> {
            Ok(self.tile.clone())
        }
        fn metadata(&self) -> &TileMetadata {
            &self.meta
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[test]
    fn test_source_manager_new_is_empty() {
        let mgr = SourceManager::new();
        assert!(mgr.is_empty());
        assert_eq!(mgr.len(), 0);
    }

    #[test]
    fn test_source_manager_default_is_empty() {
        let mgr = SourceManager::default();
        assert!(mgr.is_empty());
    }

    #[test]
    fn test_source_manager_get_returns_none_for_unknown() {
        let mgr = SourceManager::new();
        assert!(mgr.get("nonexistent").is_none());
    }

    #[test]
    fn test_source_manager_exists_returns_false_for_unknown() {
        let mgr = SourceManager::new();
        assert!(!mgr.exists("nonexistent"));
    }

    #[test]
    fn test_source_manager_from_sources() {
        let mut map = HashMap::new();
        map.insert(
            "src-a".to_string(),
            Arc::new(MockSource::new("src-a")) as Arc<dyn TileSource>,
        );
        map.insert(
            "src-b".to_string(),
            Arc::new(MockSource::new("src-b")) as Arc<dyn TileSource>,
        );

        let mgr = SourceManager::from_sources(map);
        assert_eq!(mgr.len(), 2);
        assert!(mgr.exists("src-a"));
        assert!(mgr.exists("src-b"));
        assert!(!mgr.exists("src-c"));
    }

    #[test]
    fn test_source_manager_ids() {
        let mut map = HashMap::new();
        map.insert(
            "alpha".to_string(),
            Arc::new(MockSource::new("alpha")) as Arc<dyn TileSource>,
        );
        map.insert(
            "beta".to_string(),
            Arc::new(MockSource::new("beta")) as Arc<dyn TileSource>,
        );

        let mgr = SourceManager::from_sources(map);
        let mut ids = mgr.ids();
        ids.sort();
        assert_eq!(ids, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_source_manager_all_metadata() {
        let mut map = HashMap::new();
        map.insert(
            "x".to_string(),
            Arc::new(MockSource::new("x")) as Arc<dyn TileSource>,
        );

        let mgr = SourceManager::from_sources(map);
        let metas = mgr.all_metadata();
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].id, "x");
    }

    #[test]
    fn test_source_manager_remove_source() {
        let mut map = HashMap::new();
        map.insert(
            "rem".to_string(),
            Arc::new(MockSource::new("rem")) as Arc<dyn TileSource>,
        );

        let mut mgr = SourceManager::from_sources(map);
        assert!(mgr.exists("rem"));
        assert!(mgr.remove_source("rem"));
        assert!(!mgr.exists("rem"));
        assert!(!mgr.remove_source("rem")); // second remove returns false
    }

    #[test]
    fn test_source_manager_clone_sources() {
        let mut map = HashMap::new();
        map.insert(
            "c".to_string(),
            Arc::new(MockSource::new("c")) as Arc<dyn TileSource>,
        );

        let mgr = SourceManager::from_sources(map);
        let cloned = mgr.clone_sources();
        assert_eq!(cloned.len(), 1);
        assert!(cloned.contains_key("c"));
    }

    #[test]
    fn test_source_manager_with_cache() {
        let mgr = SourceManager::new();
        assert!(mgr.cache().is_none());

        let cache = Arc::new(crate::cache::TileCache::new(1, 60));
        let mgr = mgr.with_cache(cache);
        assert!(mgr.cache().is_some());
    }

    #[tokio::test]
    async fn test_source_manager_get_tile_no_cache() {
        let mut map = HashMap::new();
        map.insert(
            "src".to_string(),
            Arc::new(MockSource::new("src")) as Arc<dyn TileSource>,
        );
        let mgr = SourceManager::from_sources(map);

        let tile = mgr.get_tile("src", 0, 0, 0).await.unwrap();
        assert!(tile.is_some());
        assert_eq!(tile.unwrap().data.as_ref(), b"mock-tile-data");
    }

    #[tokio::test]
    async fn test_source_manager_get_tile_source_not_found() {
        let mgr = SourceManager::new();
        let result = mgr.get_tile("missing", 0, 0, 0).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            crate::error::TileServerError::SourceNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_source_manager_get_tile_with_cache() {
        let mut map = HashMap::new();
        map.insert(
            "cached".to_string(),
            Arc::new(MockSource::new("cached")) as Arc<dyn TileSource>,
        );
        let cache = Arc::new(crate::cache::TileCache::new(1, 3600));
        let mgr = SourceManager::from_sources(map).with_cache(cache.clone());

        // First call populates cache
        let tile1 = mgr.get_tile("cached", 1, 2, 3).await.unwrap();
        assert!(tile1.is_some());

        // Verify it's in the cache now
        let key = crate::cache::TileCacheKey {
            source_id: "cached".into(),
            z: 1,
            x: 2,
            y: 3,
        };
        let cached = cache.get(&key).await;
        assert!(cached.is_some());
    }
}
