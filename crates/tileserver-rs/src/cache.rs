//! In-process tile cache for all source types.
//!
//! Provides a byte-weighted, TTL-evicting cache backed by `moka`.
//! All source types (PMTiles, MBTiles, etc.) share this cache.

use moka::future::Cache;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

use crate::sources::{TileCompression, TileData};

/// Cache key: uniquely identifies a tile across all sources.
///
/// `encoding` makes each `(tile, content-encoding)` pair a distinct entry so a
/// brotli re-encode never collides with the gzip source bytes. Source-level
/// caching stores the tile under its native encoding; the tile route caches
/// re-encoded variants under their negotiated encoding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TileCacheKey {
    pub source_id: Arc<str>,
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub encoding: TileCompression,
}

impl Hash for TileCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source_id.hash(state);
        self.z.hash(state);
        self.x.hash(state);
        self.y.hash(state);
        self.encoding.hash(state);
    }
}

/// Byte-weighted, TTL-evicting tile cache.
#[derive(Clone)]
pub struct TileCache {
    cache: Cache<TileCacheKey, TileData>,
}

impl TileCache {
    /// Create a new cache capped at `max_size_mb` megabytes with per-entry TTL.
    #[must_use]
    pub fn new(max_size_mb: u64, ttl_seconds: u64) -> Self {
        let max_bytes = max_size_mb * 1024 * 1024;
        let cache = Cache::builder()
            .max_capacity(max_bytes)
            .weigher(|_k: &TileCacheKey, v: &TileData| -> u32 {
                v.data.len().try_into().unwrap_or(u32::MAX)
            })
            .time_to_live(Duration::from_secs(ttl_seconds))
            .build();
        Self { cache }
    }

    /// Look up a tile. Returns `None` on miss or after TTL expiry.
    pub async fn get(&self, key: &TileCacheKey) -> Option<TileData> {
        self.cache.get(key).await
    }

    /// Insert a tile. Eviction happens asynchronously in the background.
    pub async fn insert(&self, key: TileCacheKey, value: TileData) {
        self.cache.insert(key, value).await;
    }

    /// Invalidate all entries. Eviction is eventually consistent.
    pub fn invalidate_all(&self) {
        self.cache.invalidate_all();
    }

    /// Current number of cached entries (approximate).
    #[must_use]
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Current weighted size in bytes (approximate).
    #[must_use]
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

    fn make_tile(size: usize) -> TileData {
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
            encoding: TileCompression::None,
        };
        cache.insert(key.clone(), make_tile(1024)).await;
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
            encoding: TileCompression::None,
        };
        assert!(cache.get(&key).await.is_none());
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
                encoding: TileCompression::None,
            };
            cache.insert(key, make_tile(1000)).await;
        }
        cache.cache.run_pending_tasks().await;
        assert!(cache.weighted_size() >= 4000);
    }

    #[tokio::test]
    async fn test_cache_different_sources_do_not_collide() {
        let cache = TileCache::new(10, 3600);
        let k1 = TileCacheKey {
            source_id: "source-a".into(),
            z: 1,
            x: 0,
            y: 0,
            encoding: TileCompression::None,
        };
        let k2 = TileCacheKey {
            source_id: "source-b".into(),
            z: 1,
            x: 0,
            y: 0,
            encoding: TileCompression::None,
        };
        cache.insert(k1.clone(), make_tile(100)).await;
        assert!(
            cache.get(&k2).await.is_none(),
            "different source must not collide"
        );
    }

    #[tokio::test]
    async fn test_cache_invalidate_all() {
        let cache = TileCache::new(10, 3600);
        let key = TileCacheKey {
            source_id: "src".into(),
            z: 0,
            x: 0,
            y: 0,
            encoding: TileCompression::None,
        };
        cache.insert(key.clone(), make_tile(512)).await;
        cache.cache.run_pending_tasks().await;
        assert!(cache.get(&key).await.is_some());

        cache.invalidate_all();
        cache.cache.run_pending_tasks().await;
        assert!(
            cache.get(&key).await.is_none(),
            "entry should be gone after invalidate_all"
        );
    }

    #[tokio::test]
    async fn test_cache_entry_count() {
        let cache = TileCache::new(10, 3600);
        for i in 0..3 {
            let key = TileCacheKey {
                source_id: "src".into(),
                z: 0,
                x: i,
                y: 0,
                encoding: TileCompression::None,
            };
            cache.insert(key, make_tile(100)).await;
        }
        cache.cache.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 3);
    }

    #[test]
    fn test_cache_key_equality() {
        let k1 = TileCacheKey {
            source_id: "a".into(),
            z: 1,
            x: 2,
            y: 3,
            encoding: TileCompression::None,
        };
        let k2 = TileCacheKey {
            source_id: "a".into(),
            z: 1,
            x: 2,
            y: 3,
            encoding: TileCompression::None,
        };
        assert_eq!(k1, k2);
    }

    #[test]
    fn test_cache_key_inequality_z() {
        let k1 = TileCacheKey {
            source_id: "a".into(),
            z: 1,
            x: 2,
            y: 3,
            encoding: TileCompression::None,
        };
        let k2 = TileCacheKey {
            source_id: "a".into(),
            z: 2,
            x: 2,
            y: 3,
            encoding: TileCompression::None,
        };
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_cache_key_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let k1 = TileCacheKey {
            source_id: "src".into(),
            z: 5,
            x: 10,
            y: 20,
            encoding: TileCompression::None,
        };
        let k2 = TileCacheKey {
            source_id: "src".into(),
            z: 5,
            x: 10,
            y: 20,
            encoding: TileCompression::None,
        };
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        k1.hash(&mut h1);
        k2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_cache_debug_format() {
        let cache = TileCache::new(1, 60);
        let debug = format!("{:?}", cache);
        assert!(debug.contains("TileCache"));
        assert!(debug.contains("entry_count"));
        assert!(debug.contains("weighted_size_bytes"));
    }

    #[tokio::test]
    async fn test_cache_new_is_empty() {
        let cache = TileCache::new(1, 3600);
        assert_eq!(cache.entry_count(), 0);
        assert_eq!(cache.weighted_size(), 0);
    }

    #[tokio::test]
    async fn test_cache_overwrite_same_key() {
        let cache = TileCache::new(10, 3600);
        let key = TileCacheKey {
            source_id: "src".into(),
            z: 0,
            x: 0,
            y: 0,
            encoding: TileCompression::None,
        };
        cache.insert(key.clone(), make_tile(100)).await;
        cache.insert(key.clone(), make_tile(200)).await;
        let result = cache.get(&key).await.unwrap();
        assert_eq!(result.data.len(), 200);
    }

    #[tokio::test]
    async fn test_cache_different_encodings_do_not_collide() {
        let cache = TileCache::new(10, 3600);
        let gzip_key = TileCacheKey {
            source_id: "src".into(),
            z: 4,
            x: 8,
            y: 5,
            encoding: TileCompression::Gzip,
        };
        let brotli_key = TileCacheKey {
            source_id: "src".into(),
            z: 4,
            x: 8,
            y: 5,
            encoding: TileCompression::Brotli,
        };
        cache.insert(gzip_key.clone(), make_tile(100)).await;
        assert!(
            cache.get(&brotli_key).await.is_none(),
            "same tile, different encoding must be a distinct cache entry"
        );
        assert!(cache.get(&gzip_key).await.is_some());
    }

    #[test]
    fn test_cache_key_inequality_encoding() {
        let gzip = TileCacheKey {
            source_id: "a".into(),
            z: 1,
            x: 2,
            y: 3,
            encoding: TileCompression::Gzip,
        };
        let zstd = TileCacheKey {
            source_id: "a".into(),
            z: 1,
            x: 2,
            y: 3,
            encoding: TileCompression::Zstd,
        };
        assert_ne!(gzip, zstd);
    }

    #[test]
    fn test_cache_key_hash_differs_by_encoding() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut h_gzip = DefaultHasher::new();
        let mut h_brotli = DefaultHasher::new();
        TileCacheKey {
            source_id: "src".into(),
            z: 5,
            x: 10,
            y: 20,
            encoding: TileCompression::Gzip,
        }
        .hash(&mut h_gzip);
        TileCacheKey {
            source_id: "src".into(),
            z: 5,
            x: 10,
            y: 20,
            encoding: TileCompression::Brotli,
        }
        .hash(&mut h_brotli);
        assert_ne!(h_gzip.finish(), h_brotli.finish());
    }
}
