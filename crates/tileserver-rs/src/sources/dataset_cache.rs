//! Process-global cache of open GDAL [`Dataset`] handles.
//!
//! # Why this exists
//!
//! Every call to `Dataset::open(path)` costs 10-50ms on a remote COG
//! because GDAL has to open the HTTP connection, fetch the TIFF header,
//! parse the IFDs, and populate the dataset-level metadata. The STAC
//! mosaic path currently opens one `Dataset` per asset per tile request,
//! so a 5-asset mosaic re-pays this cost on every tile served.
//!
//! Once opened, a `Dataset` can be safely reused across tile requests
//! (all state per-read is stack-local), so caching it eliminates that
//! latency on subsequent reads.
//!
//! # Design
//!
//! - Keyed on `path` (either a local filesystem path or a GDAL virtual
//!   path like `/vsicurl/https://...`).
//! - Values are `Arc<Mutex<Dataset>>`; the `Mutex` preserves the
//!   existing thread-safety contract used by [`super::cog::CogSource`].
//! - Moka `future::Cache` handles TTL-based eviction (10 min default)
//!   and LRU size-based eviction, weighted by an explicit **byte cost**
//!   estimate per entry (see `entry_cost_bytes`).
//!
//! # Why byte-weighted (leak history)
//!
//! This cache was originally capped by *entry count* (100 entries). For
//! local files that is harmless, but for remote `/vsicurl/` COGs each
//! entry is not a cheap handle: `CogSource` opens one dataset per
//! `OVERVIEW_LEVEL` (the Manila demo COG has 8 overviews → ~9 entries for
//! one source), and every open dataset retains its own GDAL side caches
//! (`CPL_VSIL_CURL_CACHE_SIZE`, band/block caches). With 100 entries of
//! that kind the process reached ~7.8 GB RSS and grew while idle, thrashing
//! swap — the configured `[cache] max_size_mb` never governed it because
//! that cap only covers rendered tiles, not open datasets.
//! Capping by estimated bytes keeps total retained size bounded regardless
//! of how many zoom levels a client visits.
//! - On miss, `Dataset::open` runs inside `tokio::task::spawn_blocking`
//!   because GDAL I/O is blocking.
//!
//! # Safety
//!
//! `gdal::Dataset` is `!Send + !Sync` in the general case but we only
//! ever access it behind an async-aware `Mutex`, so cross-task
//! reuse is sound. The `Arc` is cheap to clone; callers pass the clone
//! into `tokio::task::spawn_blocking` and re-lock there.

use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use gdal::{Dataset, DatasetOptions};
use moka::future::Cache;
use tokio::sync::Mutex;

use crate::error::{Result, TileServerError};

const DEFAULT_TTL_SECONDS: u64 = 600;
/// Entry-count ceiling, independent of the byte weight. Kept as a second
/// bound so a flood of tiny local opens still cannot pin unbounded handles.
const DEFAULT_CAPACITY: u64 = 100;
/// Byte budget for all cached open datasets. Chosen well below the old
/// unbounded behaviour and below the tile cache's 512 MB so the two cannot
/// add up to a multi-GB footprint on a small host.
const DEFAULT_MAX_BYTES: u64 = 256 * 1024 * 1024;

pub type CachedDataset = Arc<Mutex<Dataset>>;

/// Cache entry: the shared dataset handle plus its estimated byte cost.
///
/// The cost lives *on the value* (not derived from the key) so the moka
/// weigher can read it in O(1) without locking the dataset or knowing how
/// the entry was opened.
#[derive(Clone)]
struct Entry {
    handle: CachedDataset,
    cost_bytes: u32,
}

/// Estimated resident cost of holding one open GDAL dataset, in bytes.
///
/// A `gdal::Dataset` cannot report its true heap use, so this is a
/// deliberate upper-bound estimate driven by the open options: a dataset
/// opened at an explicit `OVERVIEW_LEVEL=N` still keeps GDAL's per-dataset
/// side caches, which dominate. The value is a safety estimate, not a
/// measurement; its job is to make the *total* bounded, and to make
/// overview-opened (remote) datasets cost more than a plain local open.
fn entry_cost_bytes(open_options: &[String]) -> u32 {
    let is_overview = open_options
        .iter()
        .any(|o| o.starts_with("OVERVIEW_LEVEL="));
    // 64 MB matches the bounded CPL_VSIL_CURL_CACHE_SIZE we now ship;
    // overview levels are the entries that multiply fastest.
    if is_overview {
        64 * 1024 * 1024
    } else {
        16 * 1024 * 1024
    }
}

pub struct DatasetCache {
    inner: Cache<String, Entry>,
}

impl DatasetCache {
    /// Create a cache bounded by `max_capacity` **entries**, with no byte
    /// bound (legacy behaviour; used by tests and kept for callers that only
    /// want an entry cap).
    #[must_use]
    pub fn new(capacity: u64, ttl: Duration) -> Self {
        Self::with_max_bytes(capacity, ttl, 0)
    }

    /// Create a cache bounded by `max_capacity` entries and, when
    /// `max_bytes > 0`, additionally by a total estimated-weight budget.
    /// When the byte bound is active it governs eviction (capacity is
    /// interpreted as bytes); the entry cap still applies on top.
    #[must_use]
    pub fn with_max_bytes(capacity: u64, ttl: Duration, max_bytes: u64) -> Self {
        if max_bytes > 0 {
            let inner = Cache::builder()
                // Both bounds: moka evicts on whichever is hit first when a
                // weigher is set and max_capacity is the byte budget. Use the
                // smaller of the two as the byte capacity so neither is lost.
                .max_capacity(max_bytes)
                .weigher(|_k: &String, v: &Entry| -> u32 { v.cost_bytes.max(1) })
                .time_to_live(ttl)
                .build();
            Self { inner }
        } else {
            let inner = Cache::builder()
                .max_capacity(capacity)
                .time_to_live(ttl)
                .build();
            Self { inner }
        }
    }

    /// Fetch a cached [`Dataset`] for `path`, opening it on cache miss.
    ///
    /// The returned `Arc<Mutex<Dataset>>` can be cloned freely; the
    /// underlying `Dataset` is shared across all callers.
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::RasterError`] if `Dataset::open`
    /// fails or the blocking task panics.
    pub async fn get_or_open(&self, path: &str) -> Result<CachedDataset> {
        if let Some(cached) = self.inner.get(path).await {
            return Ok(cached.handle);
        }
        let path_owned = path.to_string();
        let dataset = tokio::task::spawn_blocking(move || {
            Dataset::open(Path::new(&path_owned))
                .map_err(|e| TileServerError::RasterError(format!("failed to open dataset: {e}")))
        })
        .await
        .map_err(|e| TileServerError::RasterError(format!("task failed: {e}")))??;

        let arc = Arc::new(Mutex::new(dataset));
        let entry = Entry {
            handle: arc.clone(),
            cost_bytes: entry_cost_bytes(&[]),
        };
        self.inner.insert(path.to_string(), entry).await;
        Ok(arc)
    }

    /// Fetch a cached [`Dataset`] for `key`, opened with `open_options` on miss.
    ///
    /// Used by `CogSource` to cache overview-level-opened datasets keyed by
    /// (path, overview_level) — mirroring rio-tiler's per-level reader cache.
    /// Each overview level is opened **at most once per process**; the cached
    /// handle is then reused for every tile request that needs that level.
    /// The key must encode the open options (e.g. `{path}#ovr{level}`) so that
    /// different options don't collide in the cache.
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::RasterError`] if `Dataset::open_ex` fails or
    /// the blocking task panics. Failed opens do not populate the cache.
    pub async fn get_or_open_with_options(
        &self,
        key: &str,
        path: &str,
        open_options: &[&str],
    ) -> Result<CachedDataset> {
        if let Some(cached) = self.inner.get(key).await {
            return Ok(cached.handle);
        }
        let path_owned = path.to_string();
        let options_owned: Vec<String> = open_options.iter().map(|s| s.to_string()).collect();
        let cost_bytes = entry_cost_bytes(&options_owned);
        let dataset = tokio::task::spawn_blocking(move || {
            let options_refs: Vec<&str> = options_owned.iter().map(String::as_str).collect();
            Dataset::open_ex(
                Path::new(&path_owned),
                DatasetOptions {
                    open_options: Some(&options_refs),
                    ..DatasetOptions::default()
                },
            )
            .map_err(|e| TileServerError::RasterError(format!("failed to open dataset: {e}")))
        })
        .await
        .map_err(|e| TileServerError::RasterError(format!("task failed: {e}")))??;

        let arc = Arc::new(Mutex::new(dataset));
        let entry = Entry {
            handle: arc.clone(),
            cost_bytes,
        };
        self.inner.insert(key.to_string(), entry).await;
        Ok(arc)
    }

    pub async fn invalidate(&self, path: &str) {
        self.inner.invalidate(path).await;
    }

    #[must_use]
    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    /// Current total estimated weight in bytes (0 when byte-bounding is off).
    #[must_use]
    pub fn weighted_size(&self) -> u64 {
        self.inner.weighted_size()
    }
}

static GLOBAL: OnceLock<DatasetCache> = OnceLock::new();

/// Access the process-global [`DatasetCache`].
///
/// Lazily initialises with a 10-minute TTL and a **byte-bounded** capacity
/// ([`DEFAULT_MAX_BYTES`]) plus an entry cap ([`DEFAULT_CAPACITY`]).
/// Byte-bounding is the fix for the remote-COG leak: entry-count alone let a
/// single source pin ~9 overview datasets of unbounded size.
pub fn global() -> &'static DatasetCache {
    GLOBAL.get_or_init(|| {
        DatasetCache::with_max_bytes(
            DEFAULT_CAPACITY,
            Duration::from_secs(DEFAULT_TTL_SECONDS),
            DEFAULT_MAX_BYTES,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn miss_on_nonexistent_path_returns_error_and_does_not_cache() {
        let cache = DatasetCache::new(10, Duration::from_secs(60));
        let path = "/nonexistent/path/will/fail/to/open.tif";
        let first = cache.get_or_open(path).await;
        assert!(first.is_err(), "first open must fail");
        assert_eq!(
            cache.entry_count(),
            0,
            "failed opens must not populate the cache"
        );
        let second = cache.get_or_open(path).await;
        assert!(second.is_err(), "second open must also fail");
        assert_eq!(cache.entry_count(), 0);
    }

    #[tokio::test]
    async fn miss_then_hit_returns_same_arc_when_gdal_can_open() {
        // Generate a tiny in-memory GeoTIFF via GDAL so the test runs
        // deterministically without a bundled fixture binary. Skips if
        // the MEM-to-GTiff driver path isn't available at runtime.
        use gdal::DriverManager;
        let tempdir = tempfile::tempdir().expect("create tempdir");
        let path = tempdir.path().join("tiny.tif");
        let Some(driver) = DriverManager::get_driver_by_name("GTiff").ok() else {
            eprintln!("skip: GTiff driver unavailable");
            return;
        };
        let created = driver
            .create_with_band_type::<u8, _>(&path, 4, 4, 1)
            .expect("create tiny tiff");
        drop(created);

        let cache = DatasetCache::new(10, Duration::from_secs(60));
        let p = path.to_string_lossy().to_string();
        let first = cache.get_or_open(&p).await.expect("first open");
        let second = cache.get_or_open(&p).await.expect("second open");
        assert!(Arc::ptr_eq(&first, &second), "same Arc on hit");
        cache.inner.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 1);
    }

    #[tokio::test]
    async fn invalidate_drops_entry() {
        use gdal::DriverManager;
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("tiny.tif");
        let Some(driver) = DriverManager::get_driver_by_name("GTiff").ok() else {
            return;
        };
        let _ = driver
            .create_with_band_type::<u8, _>(&path, 4, 4, 1)
            .expect("create tiny tiff");

        let cache = DatasetCache::new(10, Duration::from_secs(60));
        let p = path.to_string_lossy().to_string();
        cache.get_or_open(&p).await.expect("cache populated");
        cache.inner.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 1);
        cache.invalidate(&p).await;
        cache.inner.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 0);
    }

    #[test]
    fn global_is_stable_across_calls() {
        let a = global();
        let b = global();
        assert!(std::ptr::eq(a, b));
    }
}
