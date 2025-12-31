//! Tile cache for PostgreSQL sources using moka.

use moka::future::Cache;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

use crate::sources::TileData;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TileCacheKey {
    pub source_id: Arc<str>,
    pub z: u8,
    pub x: u32,
    pub y: u32,
}

impl Hash for TileCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source_id.hash(state);
        self.z.hash(state);
        self.x.hash(state);
        self.y.hash(state);
    }
}

#[derive(Clone)]
pub struct TileCache {
    cache: Cache<TileCacheKey, TileData>,
}

impl TileCache {
    pub fn new(max_size_mb: u64, ttl_seconds: u64) -> Self {
        let max_size_bytes = max_size_mb * 1024 * 1024;

        let cache = Cache::builder()
            .max_capacity(max_size_bytes)
            .weigher(|_key: &TileCacheKey, value: &TileData| -> u32 {
                value.data.len().try_into().unwrap_or(u32::MAX)
            })
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();

        Self { cache }
    }

    pub async fn get(&self, key: &TileCacheKey) -> Option<TileData> {
        self.cache.get(key).await
    }

    pub async fn insert(&self, key: TileCacheKey, value: TileData) {
        self.cache.insert(key, value).await;
    }

    #[allow(dead_code)]
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    #[allow(dead_code)]
    pub fn weighted_size(&self) -> u64 {
        self.cache.weighted_size()
    }
}

impl std::fmt::Debug for TileCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileCache")
            .field("entry_count", &self.cache.entry_count())
            .field("weighted_size_bytes", &self.cache.weighted_size())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::{TileCompression, TileFormat};
    use bytes::Bytes;

    fn make_tile_data(size: usize) -> TileData {
        TileData {
            data: Bytes::from(vec![0u8; size]),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        }
    }

    #[tokio::test]
    async fn test_cache_insert_and_get() {
        let cache = TileCache::new(1, 3600);
        let key = TileCacheKey {
            source_id: "test".into(),
            z: 14,
            x: 8580,
            y: 5737,
        };
        let tile = make_tile_data(1024);

        cache.insert(key.clone(), tile.clone()).await;
        let result = cache.get(&key).await;

        assert!(result.is_some());
        assert_eq!(result.unwrap().data.len(), 1024);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = TileCache::new(1, 3600);
        let key = TileCacheKey {
            source_id: "test".into(),
            z: 14,
            x: 8580,
            y: 5737,
        };

        let result = cache.get(&key).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_weighted_size() {
        let cache = TileCache::new(10, 3600);

        for i in 0..5 {
            let key = TileCacheKey {
                source_id: "test".into(),
                z: 14,
                x: i,
                y: 0,
            };
            cache.insert(key, make_tile_data(1000)).await;
        }

        cache.cache.run_pending_tasks().await;
        assert!(cache.weighted_size() >= 4000);
    }
}
