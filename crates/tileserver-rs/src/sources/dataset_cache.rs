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
//! - Keyed on a string key: the source `path` for the base dataset, or a
//!   `"{path}#ovr{level}"` key for an `OVERVIEW_LEVEL`-opened dataset (see
//!   [`super::cog::CogSource`]).
//! - Values are `Entry { handle, cost_bytes }`, where `handle` is an
//!   `Arc<Mutex<Dataset>>`; the `Mutex` preserves the existing
//!   thread-safety contract used by [`super::cog::CogSource`].
//! - Moka `future::Cache` handles TTL-based eviction (10 min default)
//!   and LRU eviction, weighted by the explicit **byte cost** estimate
//!   per entry (see `entry_cost_bytes`).
//! - On miss, `Dataset::open` runs inside `tokio::task::spawn_blocking`
//!   because GDAL I/O is blocking.
//!
//! # Why byte-weighted
//!
//! This cache is bounded by an estimated **byte budget**, not by an entry
//! count. A remote `/vsicurl/` COG is not one cheap handle: `CogSource`
//! opens one dataset per `OVERVIEW_LEVEL` (a COG with 8 overviews yields up
//! to 9 entries for a single source), and every open dataset retains its own
//! GDAL side caches. An entry *count* bound therefore does not bound memory —
//! a handful of such entries can pin gigabytes. Weighting each entry by an
//! estimated byte cost keeps the *total* retained size bounded regardless of
//! how many overview levels a client visits.
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
/// Byte budget for all cached open datasets.
///
/// Deliberately below the rendered-tile cache's own budget so the two cannot
/// add up to a multi-GB footprint on a small host. This is the sole bound on
/// the cache: moka has a single `max_capacity` and it is interpreted as
/// weight when a weigher is installed (see [`DatasetCache::with_max_bytes`]).
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
/// deliberate upper-bound estimate. It is a fixed constant per open kind
/// rather than a measurement: its job is to make the *total* bounded and to
/// make overview-opened datasets (which carry their own per-dataset GDAL side
/// caches and multiply fastest) cost more than a plain open.
///
/// The 64 MB figure is intentionally conservative and larger than the
/// per-dataset VSI curl cache; tune it here if the working set needs to grow
/// or shrink.
fn entry_cost_bytes(open_options: &[String]) -> u32 {
    let is_overview = open_options
        .iter()
        .any(|o| o.starts_with("OVERVIEW_LEVEL="));
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
    /// bound. This is the legacy behaviour and is retained only for callers
    /// (and tests) that want a plain entry cap; production uses
    /// [`Self::with_max_bytes`].
    #[must_use]
    pub fn new(capacity: u64, ttl: Duration) -> Self {
        let inner = Cache::builder()
            .max_capacity(capacity)
            .time_to_live(ttl)
            .build();
        Self { inner }
    }

    /// Create a cache bounded by a total estimated-weight budget of
    /// `max_bytes`. When `max_bytes == 0` the cache falls back to an
    /// entry-count bound of `capacity` entries (legacy behaviour).
    ///
    /// Note moka has a **single** `max_capacity`. With a weigher installed it
    /// is interpreted as weight units, so the byte budget is the only active
    /// bound; no separate entry cap is enforced alongside it.
    #[must_use]
    pub fn with_max_bytes(capacity: u64, ttl: Duration, max_bytes: u64) -> Self {
        if max_bytes > 0 {
            let inner = Cache::builder()
                .max_capacity(max_bytes)
                .weigher(|_k: &String, v: &Entry| -> u32 { v.cost_bytes.max(1) })
                .time_to_live(ttl)
                .build();
            Self { inner }
        } else {
            Self::new(capacity, ttl)
        }
    }

    /// Fetch the dataset for `key`, opening `path` (with `open_options` if
    /// non-empty) on a cache miss. This is the single open path: both the
    /// base dataset (`key == path`, no options) and an overview-level dataset
    /// (`key == "{path}#ovr{level}"`, with the `OVERVIEW_LEVEL` option) route
    /// through it.
    ///
    /// The returned `Arc<Mutex<Dataset>>` can be cloned freely; the
    /// underlying `Dataset` is shared across all callers.
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::RasterError`] if the open fails or the
    /// blocking task panics. Failed opens do not populate the cache.
    pub async fn get_or_open_cached(
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
            if options_owned.is_empty() {
                Dataset::open(Path::new(&path_owned))
            } else {
                let options_refs: Vec<&str> = options_owned.iter().map(String::as_str).collect();
                Dataset::open_ex(
                    Path::new(&path_owned),
                    DatasetOptions {
                        open_options: Some(&options_refs),
                        ..DatasetOptions::default()
                    },
                )
            }
            .map_err(|e| TileServerError::RasterError(format!("failed to open dataset: {e}")))
        })
        .await
        .map_err(|e| TileServerError::RasterError(format!("task failed: {e}")))??;

        let arc: CachedDataset = Arc::new(Mutex::new(dataset));
        let entry = Entry {
            handle: arc.clone(),
            cost_bytes,
        };
        self.inner.insert(key.to_string(), entry).await;
        Ok(arc)
    }

    /// Fetch the base dataset for `path`, opening it on a cache miss.
    ///
    /// Convenience wrapper over [`Self::get_or_open_cached`] with no open
    /// options.
    ///
    /// # Errors
    ///
    /// See [`Self::get_or_open_cached`].
    pub async fn get_or_open(&self, path: &str) -> Result<CachedDataset> {
        self.get_or_open_cached(path, path, &[]).await
    }

    /// Fetch a dataset for `key`, opened with `open_options` on a cache miss.
    ///
    /// Used by [`super::cog::CogSource`] to cache overview-level-opened
    /// datasets keyed by `{path}#ovr{level}` — mirroring rio-tiler's per-level
    /// reader cache. Each overview level is opened **at most once per
    /// process**; the cached handle is then reused for every tile request that
    /// needs that level. The key must encode the open options so different
    /// options do not collide.
    ///
    /// # Errors
    ///
    /// See [`Self::get_or_open_cached`].
    pub async fn get_or_open_with_options(
        &self,
        key: &str,
        path: &str,
        open_options: &[&str],
    ) -> Result<CachedDataset> {
        self.get_or_open_cached(key, path, open_options).await
    }

    pub async fn invalidate(&self, path: &str) {
        self.inner.invalidate(path).await;
    }

    #[must_use]
    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    /// Current total estimated weight in bytes (0 when not byte-bounded).
    #[must_use]
    pub fn weighted_size(&self) -> u64 {
        self.inner.weighted_size()
    }
}

static GLOBAL: OnceLock<DatasetCache> = OnceLock::new();

/// Access the process-global [`DatasetCache`].
///
/// Lazily initialises with a 10-minute TTL and a **byte-bounded** capacity of
/// [`DEFAULT_MAX_BYTES`]. Byte-bounding is what keeps a remote-COG source
/// (which opens one dataset per overview level) from pinning gigabytes.
pub fn global() -> &'static DatasetCache {
    GLOBAL.get_or_init(|| {
        DatasetCache::with_max_bytes(
            0, // byte budget is the only bound; no entry cap
            Duration::from_secs(DEFAULT_TTL_SECONDS),
            DEFAULT_MAX_BYTES,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique-ish path in a fresh tempdir; returns (dir, path). The dir must
    /// stay alive for the path to be openable.
    fn tiny_gtiff(name: &str) -> Option<(tempfile::TempDir, String)> {
        use gdal::DriverManager;
        let dir = tempfile::tempdir().ok()?;
        let path = dir.path().join(name);
        let driver = DriverManager::get_driver_by_name("GTiff").ok()?;
        driver.create_with_band_type::<u8, _>(&path, 4, 4, 1).ok()?;
        Some((dir, path.to_string_lossy().to_string()))
    }

    #[tokio::test]
    async fn miss_on_nonexistent_path_returns_error_and_does_not_cache() {
        let cache = DatasetCache::new(10, Duration::from_secs(60));
        let path = "/nonexistent/path/will/fail/to/open.tif";
        assert!(cache.get_or_open(path).await.is_err());
        cache.inner.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 0, "failed opens must not be cached");
        assert!(cache.get_or_open(path).await.is_err());
        cache.inner.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 0);
    }

    #[tokio::test]
    async fn miss_then_hit_returns_same_arc_when_gdal_can_open() {
        let Some((_dir, p)) = tiny_gtiff("tiny.tif") else {
            eprintln!("skip: GTiff driver unavailable");
            return;
        };
        let cache = DatasetCache::new(10, Duration::from_secs(60));
        let first = cache.get_or_open(&p).await.expect("first open");
        let second = cache.get_or_open(&p).await.expect("second open");
        assert!(Arc::ptr_eq(&first, &second), "same Arc on hit");
        cache.inner.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 1);
    }

    #[tokio::test]
    async fn base_and_overview_keys_are_distinct_entries() {
        // A 4x4 GEOTIFF has no internal overviews, so we cannot open it at a
        // real `OVERVIEW_LEVEL`. What matters here is the *cache* contract:
        // two different keys pointing at the same file are two entries, even
        // when the open options are the same. That is the property `CogSource`
        // relies on (`{path}` vs `{path}#ovr{n}`), and it does not depend on
        // the raster actually having overviews.
        let Some((_dir, p)) = tiny_gtiff("tiny.tif") else {
            return;
        };
        let cache = DatasetCache::new(10, Duration::from_secs(60));
        let base = cache.get_or_open(&p).await.expect("base open");
        let ovr_key = format!("{p}#ovr0");
        let ovr = cache
            .get_or_open_with_options(&ovr_key, &p, &[])
            .await
            .expect("second key open");
        cache.inner.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 2, "distinct keys are distinct entries");
        assert!(!Arc::ptr_eq(&base, &ovr), "distinct handles per key");
    }

    #[tokio::test]
    async fn weighted_size_counts_the_byte_budget() {
        let Some((_dir, p)) = tiny_gtiff("tiny.tif") else {
            return;
        };
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), DEFAULT_MAX_BYTES);
        assert_eq!(cache.weighted_size(), 0, "empty cache weighs nothing");
        let _ = cache.get_or_open(&p).await.expect("open");
        cache.inner.run_pending_tasks().await;
        // A base (non-overview) open costs 16 MB by the estimator.
        assert_eq!(cache.weighted_size(), 16 * 1024 * 1024);
    }

    #[tokio::test]
    async fn evicts_when_byte_budget_is_exceeded() {
        // A budget smaller than a single base entry (16 MB) forces eviction as
        // soon as anything is inserted; the cache must not retain the entry.
        let Some((_d1, p1)) = tiny_gtiff("a.tif") else {
            return;
        };
        let small_budget = 1; // 1 byte: every real entry is over budget
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), small_budget);

        let _ = cache.get_or_open(&p1).await.expect("open");
        cache.inner.run_pending_tasks().await;
        assert_eq!(
            cache.entry_count(),
            0,
            "entry exceeding the byte budget must be evicted"
        );
    }

    #[tokio::test]
    async fn invalidate_drops_entry() {
        let Some((_dir, p)) = tiny_gtiff("tiny.tif") else {
            return;
        };
        let cache = DatasetCache::new(10, Duration::from_secs(60));
        cache.get_or_open(&p).await.expect("cache populated");
        cache.inner.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 1);
        cache.invalidate(&p).await;
        cache.inner.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 0);
    }

    #[test]
    fn global_is_stable_across_calls() {
        assert!(std::ptr::eq(global(), global()));
    }
}
