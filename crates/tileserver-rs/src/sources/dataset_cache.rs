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
//! - Values are [`Pool`]s: a small set of `Dataset` handles for that key,
//!   not a single handle. A render *borrows* one handle for the duration of
//!   the render and returns it; concurrent renders at the same key each take
//!   a different handle, so they run in parallel. See [`Pool`] for the
//!   acquire/release contract and why a single `Arc<Mutex<Dataset>>` was not
//!   enough.
//! - Moka `future::Cache` handles TTL-based eviction (10 min default) and
//!   LRU eviction, weighted by the explicit **byte cost** estimate per entry
//!   (see [`Pool::weighted_bytes`]).
//! - On miss, `Dataset::open` runs inside `tokio::task::spawn_blocking`
//!   because GDAL I/O is blocking.
//!
//! # Why byte-weighted
//!
//! This cache *retains* up to an estimated **byte budget**, not an entry
//! count. A remote `/vsicurl/` COG is not one cheap handle: `CogSource`
//! opens one dataset per `OVERVIEW_LEVEL` (a COG with 8 overviews yields up
//! to 9 entries for a single source), each pool may hold several handles, and
//! every open dataset retains its own GDAL side caches. An entry *count*
//! bound therefore does not bound memory — a handful of such entries can pin
//! gigabytes. Weighting each entry by an estimated byte cost keeps the
//! *total* retained size bounded regardless of how many overview levels a
//! client visits or how many handles each pool grows to.
//!
//! The budget governs what the cache **retains and reuses**, never whether a
//! tile renders. [`Pool::acquire`] declines to open a new *cached* handle once
//! the process-wide live byte total would exceed the budget (see
//! [`Pool::reserve_budget`]), so the retained total stays bounded. But a
//! renderable tile must always succeed: when the cache cannot hand out a
//! counted handle, `acquire_with_reclaim` falls back to an uncounted one-shot
//! open ([`Pool::open_uncached`]) — the pre-cache baseline — after a short
//! bounded wait, bounded cache-wide by a semaphore. The cache is therefore
//! never observably worse than no cache at all; it only ever speeds things up
//! while the budget allows.
//!
//! # Safety
//!
//! `gdal::Dataset` is `!Send + !Sync` in the general case but we only ever
//! access it behind an async-aware `Mutex` (the pool's `idle` list), so
//! cross-task reuse is sound. The `Arc<Pool>` is cheap to clone; callers pass
//! the clone into `tokio::task::spawn_blocking` and acquire a handle there.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex as StdMutex, OnceLock};
use std::time::Duration;

use gdal::{Dataset, DatasetOptions};
use moka::future::Cache;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::error::{Result, TileServerError};

const DEFAULT_TTL_SECONDS: u64 = 600;
/// Byte budget for all cached open datasets.
///
/// This bounds how much the cache **retains and reuses**, not whether a tile
/// renders: under saturation the cache falls back to uncached per-render opens
/// exactly like the pre-cache baseline, so it is never observably worse than
/// no cache at all — it only ever speeds things up while the budget allows.
///
/// Deliberately below the rendered-tile cache's own budget so the two cannot
/// add up to a multi-GB footprint on a small host. This is the sole *retention*
/// bound on the cache: moka has a single `max_capacity` and it is interpreted
/// as weight when a weigher is installed (see [`DatasetCache::with_max_bytes`]).
const DEFAULT_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// Environment override for [`DEFAULT_MAX_BYTES`], in bytes.
///
/// A dynamic STAC mosaic opens one handle per asset, each costing
/// [`BASE_COST_BYTES`], so an N-asset mosaic needs roughly
/// `N * BASE_COST_BYTES` of budget to hold every asset at once. A 20-asset
/// supercell mosaic therefore saturates the 256 MB default exactly. It must not
/// *fail* past that point: any further asset renders via the uncounted one-shot
/// fallback (see [`Pool::open_uncached`]), so tiles keep rendering — they just
/// re-open, exactly as they would without the cache.
///
/// Deployments with large mosaics can raise this without a rebuild; the
/// default stays conservative so the crate does not assume a big host.
const MAX_BYTES_ENV: &str = "TILESERVER_DATASET_CACHE_MAX_BYTES";

/// Resolve the effective byte budget: env override if set and parseable,
/// otherwise [`DEFAULT_MAX_BYTES`].
///
/// An unparseable or zero value is ignored rather than treated as fatal — a
/// zero budget would make every cache miss fail, which is never the intent of
/// a typo in an env var.
fn resolve_max_bytes() -> u64 {
    match std::env::var(MAX_BYTES_ENV) {
        Ok(raw) => match raw.trim().parse::<u64>() {
            Ok(bytes) if bytes > 0 => bytes,
            _ => {
                tracing::warn!(
                    "{MAX_BYTES_ENV}={raw:?} is not a positive integer; using default {DEFAULT_MAX_BYTES}"
                );
                DEFAULT_MAX_BYTES
            }
        },
        Err(_) => DEFAULT_MAX_BYTES,
    }
}

/// Estimated resident cost of holding one open GDAL dataset, in bytes.
///
/// A `gdal::Dataset` cannot report its true heap use, so this is a deliberate
/// upper-bound estimate. It is a fixed constant per open kind rather than a
/// measurement: its job is to make the *total* bounded and to make
/// overview-opened datasets (which carry their own per-dataset GDAL side
/// caches) cost more than a plain open.
///
/// The overview figure is deliberately close to the observed working set
/// (~8 MB per overview dataset in our RSS measurements) so that a 256 MB
/// budget holds tens of handles rather than a handful — an over-large
/// estimate would make the pool evict and reopen constantly, which is exactly
/// the thrash this cache exists to prevent. Tune it here if the working set
/// needs to grow or shrink.
const OVERVIEW_COST_BYTES: u64 = 8 * 1024 * 1024;
const BASE_COST_BYTES: u64 = 16 * 1024 * 1024;

/// Maximum number of handles a single pool may hold.
///
/// One handle per hardware thread is enough to keep the render pipeline busy
/// (the render is CPU/IO-bound and does not oversubscribe usefully), and it
/// matches the maintainer's proposal. A pool never opens more than this even
/// if the byte budget would allow it.
fn pool_limit() -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
}

/// How long `acquire_with_reclaim` waits for a handle to be *returned* before
/// falling back to an uncounted one-shot open.
///
/// The common case is transient saturation: a render finishes and returns its
/// handle a moment later, so a short wait turns an uncounted open into a cheap
/// cache hit. It must stay short — this is added to every budget-exhausted
/// tile's tail latency — hence the reviewer's suggested 50-100 ms window.
const ONE_SHOT_WAIT_MS: u64 = 75;

/// Maximum idle handles a single pool retains.
///
/// `idle` handles are free, still-counted handles kept only so the next render
/// at this key skips a reopen. Without a cap a single busy source that then
/// stops rendering keeps *all* its handles counted in the cache until the TTL
/// expires, pinning budget that another, actively-rendering source needs. On
/// `PooledHandle::drop`, a handle returned beyond this cap is closed and its
/// bytes refunded instead of parked in the idle list.
///
/// The cap is deliberately small (the reviewer's "1-2"): the win from keeping
/// more than a couple of warm handles is marginal, and each one held is budget
/// another pool cannot use. Only ever costs a slightly slower next acquire.
const IDLE_CAP: usize = 2;

/// Cap on simultaneous **uncounted** one-shot fallback opens, cache-wide.
///
/// One-shot handles are deliberately not counted against the byte budget, so
/// without a cap their number would scale with incoming concurrency and the
/// transient peak could exceed the budget by an unbounded amount. Twice the
/// CPU count keeps the fallback useful under a burst without letting it balloon
/// (the reviewer's suggested `2 * CPU`). Distinct from [`pool_limit`], which
/// bounds *counted* cached handles per key.
fn one_shot_limit() -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
        .saturating_mul(2)
}

/// Estimated byte cost of one handle opened with `open_options`.
fn entry_cost_bytes(open_options: &[String]) -> u64 {
    let is_overview = open_options
        .iter()
        .any(|o| o.starts_with("OVERVIEW_LEVEL="));
    if is_overview {
        OVERVIEW_COST_BYTES
    } else {
        BASE_COST_BYTES
    }
}

/// A handle borrowed from a [`Pool`].
///
/// Dropping a *pooled* handle returns the underlying `Dataset` to the pool's
/// idle list (or closes it if the pool is gone). The handle's byte cost stays
/// counted in the pool's cache while it is borrowed, and is released only when
/// the pool itself is dropped/evicted (`Drop for Pool`). This makes release
/// panic-safe: a render that unwinds still returns its slot, so the pool cannot
/// leak capacity.
///
/// Dropping an *uncounted one-shot* handle (`uncached == true`) instead just
/// closes the `Dataset` and releases its semaphore permit: it was never in the
/// pool and never counted, so there is nothing to return.
pub struct PooledHandle {
    dataset: Option<Dataset>,
    pool: Arc<Pool>,
    /// True for a one-shot fallback handle: closed on drop, never returned to
    /// the pool and never counted against the byte budget.
    uncached: bool,
    /// Held only for the lifetime of a one-shot handle, to bound how many
    /// uncounted opens run at once.
    _permit: Option<OwnedSemaphorePermit>,
}

impl PooledHandle {
    /// Borrow the underlying dataset for the duration of a render.
    #[must_use]
    pub fn dataset(&self) -> &Dataset {
        self.dataset
            .as_ref()
            .expect("PooledHandle dataset taken only on drop")
    }

    /// Whether this handle is an uncounted one-shot fallback (see [`Pool`]).
    #[must_use]
    pub fn is_uncached(&self) -> bool {
        self.uncached
    }
}

impl Drop for PooledHandle {
    fn drop(&mut self) {
        if let Some(dataset) = self.dataset.take() {
            if self.uncached {
                // One-shot fallback: close the dataset and release the permit.
                // Nothing was counted and nothing is returned to the pool.
                drop(dataset);
                self._permit.take();
                return;
            }
            // Return the handle to the idle list and wake one waiter. The
            // handle stays open and stays counted against the byte budget
            // (`live` is NOT decremented here) — it is merely free to be
            // borrowed again. It is only closed, and its cost released, when
            // the pool itself is dropped/evicted.
            //
            // Idle cap: a pool that has stopped rendering must not sit on the
            // whole budget. Once the idle list is full, the returned handle is
            // closed and its bytes refunded instead of parked. That keeps the
            // counter honest (`live` drops with the close) and lets an
            // actively-rendering pool reclaim those bytes on the next pass.
            let mut state = self.pool.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.idle.len() < IDLE_CAP {
                state.idle.push(dataset);
                drop(state);
                self.pool.returned.notify_one();
            } else {
                // Over the cap: close it now and refund its bytes. `live` is
                // decremented here (unlike the parked case), so the byte
                // counter matches the handles that actually remain open.
                state.live -= 1;
                drop(state);
                drop(dataset);
                self.pool
                    .live_bytes
                    .fetch_sub(self.pool.cost_bytes, Ordering::AcqRel);
            }
        }
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        // Refund the bytes of every handle the pool still holds. Reclaim
        // refunds per handle it closes (and decrements `live` with it), so
        // whatever is left in `live` here has never been refunded. Exactly
        // once, whatever the order of reclaim, eviction and the last `Arc`.
        let live = self
            .state
            .get_mut()
            .map_or_else(|e| e.into_inner().live, |s| s.live);
        self.live_bytes
            .fetch_sub(live as u64 * self.cost_bytes, Ordering::AcqRel);
    }
}

/// A small pool of open `Dataset` handles for one cache key.
///
/// # Why a pool and not one handle
///
/// A single `Arc<Mutex<Dataset>>` per key serialises every render at that key:
/// the render holds the lock for its whole duration, so N concurrent tiles at
/// one overview level render one at a time. Measured on a local COG that is
/// ~4x slower than opening per tile at 16-way concurrency. A pool keeps the
/// cached-open win *and* lets N renders proceed in parallel, by handing each
/// concurrent render a different handle.
///
/// # Bounds
///
/// Three independent bounds apply, and all are enforced:
/// - **Concurrency**: `limit` handles at most (`available_parallelism()`).
///   Acquisition beyond that *waits* for a handle to be returned rather than
///   opening more.
/// - **Memory**: a new handle is only opened while the process-wide live byte
///   total stays within the cache's byte budget. When the budget is full the
///   pool reuses existing handles instead of opening new ones.
/// - **Idle retention**: at most [`IDLE_CAP`] free handles are kept idle. A
///   pool that stops rendering closes the excess on return (refunding its
///   bytes) rather than pinning the whole budget; a later acquire reopens.
///
/// # Why a condvar and not a semaphore
///
/// The pool needs exactly one thing: a way to wait until a handle is free
/// without holding the lock that the returning thread needs. A `Condvar`
/// paired with the state mutex does precisely that — `wait` atomically
/// releases the mutex while parked, and `PooledHandle::drop` signals it. A
/// semaphore would add a second primitive, a second accounting path, and an
/// async/blocking boundary to get right, for no additional guarantee: the
/// `live < limit` check under the same lock already bounds concurrency.
pub struct Pool {
    /// Idle handles plus the live count, under one lock. `live` counts every
    /// open handle (idle + borrowed) and is the number the byte budget and the
    /// concurrency limit gate on.
    state: StdMutex<PoolState>,
    /// Signalled by [`PooledHandle::drop`] when a handle is returned.
    returned: Condvar,
    /// Max handles this pool may hold.
    limit: usize,
    /// Byte cost of one handle in this pool (same for all handles in a pool,
    /// since they share `open_options`).
    cost_bytes: u64,
    /// Cache-wide byte budget; a new *cached* handle is refused once the
    /// cache's live total would exceed it. (Renders still proceed via the
    /// uncounted one-shot fallback; see [`DatasetCache::acquire_with_reclaim`].)
    max_bytes: u64,
    /// Shared live-bytes counter owned by the [`DatasetCache`] this pool
    /// belongs to. `reserve_budget` and `Drop` read/write it, so the budget is
    /// a per-cache bound rather than a process-wide one. Each pool holds a
    /// clone of the cache's `Arc`.
    live_bytes: Arc<AtomicU64>,
    /// Cache-wide cap on simultaneous **uncounted** one-shot fallback opens.
    /// A one-shot open acquires a permit before opening and releases it when
    /// the handle drops. Each pool holds a clone of the cache's `Arc`.
    one_shot_permits: Arc<Semaphore>,
    /// Path to open new handles from.
    path: String,
    /// Open options (e.g. `OVERVIEW_LEVEL=N`) to open new handles with.
    open_options: Vec<String>,
}

/// Mutable pool state, guarded by a single mutex.
struct PoolState {
    /// Free handles, ready to be reused.
    idle: Vec<Dataset>,
    /// Handles currently open (idle + borrowed).
    live: usize,
}

impl Pool {
    /// Create an empty pool for `path` opened with `open_options`, sharing the
    /// cache's live-bytes counter `live_bytes` and one-shot `one_shot_permits`.
    fn new(
        path: String,
        open_options: Vec<String>,
        max_bytes: u64,
        live_bytes: Arc<AtomicU64>,
        one_shot_permits: Arc<Semaphore>,
    ) -> Self {
        let limit = pool_limit().max(1);
        let cost_bytes = entry_cost_bytes(&open_options);
        Self {
            state: StdMutex::new(PoolState {
                idle: Vec::new(),
                live: 0,
            }),
            returned: Condvar::new(),
            limit,
            cost_bytes,
            max_bytes,
            live_bytes,
            one_shot_permits,
            path,
            open_options,
        }
    }

    /// Current estimated resident bytes held by this pool.
    #[must_use]
    pub fn weighted_bytes(&self) -> u64 {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.live as u64 * self.cost_bytes
    }

    /// Number of handles currently open (idle + borrowed).
    #[must_use]
    pub fn live_handles(&self) -> usize {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).live
    }

    /// Close and remove every idle handle, returning how many were taken.
    ///
    /// Used by [`DatasetCache::reclaim_idle`], which then refunds exactly the
    /// returned count with [`Self::release_partial`]. Closing and refunding
    /// per handle keeps the counter equal to the handles that are really
    /// open, whether or not the pool is later evicted.
    fn take_idle_handles(&self) -> usize {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let taken = state.idle.len();
        // `Dataset` closes on drop; move the handles out before releasing the
        // lock so the (possibly slow) close does not serialise other renderers.
        let handles = std::mem::take(&mut state.idle);
        state.live -= taken;
        drop(state);
        drop(handles);
        taken
    }

    /// Refund `count` handles' worth of bytes to the shared counter.
    ///
    /// The caller must have closed `count` handles first (see
    /// [`Self::take_idle_handles`]); handles that are still open stay counted
    /// until they close or the pool drops.
    fn release_partial(&self, count: usize) {
        if count == 0 {
            return;
        }
        self.live_bytes
            .fetch_sub(count as u64 * self.cost_bytes, Ordering::AcqRel);
    }

    /// Acquire a handle for one render, given an `Arc<Pool>` so the returned
    /// [`PooledHandle`] can keep the pool alive and return the handle on drop.
    ///
    /// The fast path (a free handle is already idle) needs no I/O and runs on
    /// the calling async thread. The slow path (opening a new handle or
    /// waiting for one) is blocking work, so it is moved onto
    /// `tokio::task::spawn_blocking` — GDAL's `Dataset::open` and the pool's
    /// condvar `wait` must not run on the async thread or they can stall the
    /// runtime. The returned [`PooledHandle`] returns the handle to the pool
    /// on drop.
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::RasterError`] if a new handle must be opened
    /// and the open fails.
    pub async fn acquire_arc(self: &Arc<Self>) -> Result<PooledHandle> {
        // Fast path: a free handle needs no I/O — take it on the async thread.
        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(dataset) = state.idle.pop() {
                return Ok(PooledHandle {
                    dataset: Some(dataset),
                    pool: Arc::clone(self),
                    uncached: false,
                    _permit: None,
                });
            }
        }

        // Slow path: opening (and any waiting) is blocking work — run it off
        // the async thread so GDAL's open cannot stall the runtime.
        let pool = Arc::clone(self);
        tokio::task::spawn_blocking(move || pool.acquire_blocking())
            .await
            .map_err(|e| TileServerError::RasterError(format!("task failed: {e}")))?
    }

    /// Blocking acquire: open a handle or wait for one.
    ///
    /// MUST run on a blocking thread (GDAL open + condvar wait are both
    /// blocking). It re-checks the idle list inside because a handle may have
    /// been returned between the fast-path check in [`Self::acquire_arc`] and
    /// this blocking task starting.
    ///
    /// This is the *counted* path only: it never opens an uncounted one-shot
    /// handle. When it returns [`TileServerError::BudgetExhausted`] the async
    /// caller ([`DatasetCache::acquire_with_reclaim`]) escalates — reclaim,
    /// short wait, then a one-shot open — so a tile never fails for budget
    /// reasons.
    pub(crate) fn acquire_blocking(self: &Arc<Self>) -> Result<PooledHandle> {
        // Budget-fit precheck. The per-pool `limit` (one handle per hardware
        // thread) is *not* derived from the byte budget, so a single pool can
        // ask for more bytes than the whole cache is allowed to hold. Left
        // unchecked the loop below would spin on a reservation that can never
        // succeed. Signal `BudgetExhausted` *before* taking the state lock so
        // `acquire_with_reclaim` can escalate to the one-shot fallback: this
        // pool can never be *cached*, but its tiles still render.
        if self.max_bytes > 0 && self.cost_bytes > self.max_bytes {
            return Err(TileServerError::BudgetExhausted {
                path: self.path.clone(),
                cost_bytes: self.cost_bytes,
                max_bytes: self.max_bytes,
                live_bytes: self.live_bytes.load(Ordering::Acquire),
            });
        }

        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            // 1. Reuse a free handle.
            if let Some(dataset) = state.idle.pop() {
                return Ok(PooledHandle {
                    dataset: Some(dataset),
                    pool: Arc::clone(self),
                    uncached: false,
                    _permit: None,
                });
            }

            // 2. No free handle: open one if BOTH bounds allow. The byte
            //    budget is reserved before the open so a concurrent opener
            //    cannot slip past it.
            if state.live < self.limit && self.reserve_budget() {
                match self.open_blocking() {
                    Ok(dataset) => {
                        state.live += 1;
                        return Ok(PooledHandle {
                            dataset: Some(dataset),
                            pool: Arc::clone(self),
                            uncached: false,
                            _permit: None,
                        });
                    }
                    Err(e) => {
                        // Open failed: release the reservation we just took.
                        self.live_bytes.fetch_sub(self.cost_bytes, Ordering::AcqRel);
                        return Err(e);
                    }
                }
            }

            // 2b. Nothing idle and the byte budget refused the open.
            //
            //     Two distinct cases, and conflating them is what turned a
            //     per-thread guard into a process-wide outage:
            //
            //     (i) `live > 0`. This pool holds handles that are merely
            //         borrowed. Waiting is correct: a render finishes, drops
            //         its `PooledHandle`, and notifies us.
            //
            //     (ii) `live == 0` and the cache's *global* total is
            //          genuinely occupied by **other** pools. Nothing this
            //          pool can wait for will ever free, so the condvar
            //          would park forever. The old code errored here, which
            //          bricked the server: a mosaic holding the whole budget
            //          made every other source fail permanently. Return the
            //          dedicated `BudgetExhausted` signal so the caller can
            //          reclaim from other pools and retry.
            if state.live == 0 {
                let live_total = self.live_bytes.load(Ordering::Acquire);
                // Drop the state lock first: reclaiming evicts *other* pools,
                // and their `Drop` takes their own state lock. Holding ours
                // across that is a lock-order hazard for no benefit.
                drop(state);
                return Err(TileServerError::BudgetExhausted {
                    path: self.path.clone(),
                    cost_bytes: self.cost_bytes,
                    max_bytes: self.max_bytes,
                    live_bytes: live_total,
                });
            }

            // 3. At a bound: wait for a handle to be returned. `wait` releases
            //    the mutex while parked, so `drop` can take it to push the
            //    returned handle and notify us.
            state = self.returned.wait(state).unwrap_or_else(|e| e.into_inner());
        }
    }

    /// Try to reserve `cost_bytes` against the cache's byte budget.
    ///
    /// Returns `true` and increments the cache's live-bytes counter on
    /// success; returns `false` (leaving the total unchanged) when the budget
    /// is full.
    ///
    /// A `max_bytes == 0` pool has **no byte budget** (the legacy
    /// entry-count-only cache) — in that case the reservation always succeeds
    /// and the open is only gated by the concurrency `limit`.
    fn reserve_budget(&self) -> bool {
        if self.max_bytes == 0 {
            self.live_bytes.fetch_add(self.cost_bytes, Ordering::AcqRel);
            return true;
        }
        self.live_bytes
            .try_update(Ordering::AcqRel, Ordering::Acquire, |cur| {
                if cur + self.cost_bytes <= self.max_bytes {
                    Some(cur + self.cost_bytes)
                } else {
                    None
                }
            })
            .is_ok()
    }

    /// Open one dataset with this pool's path and options.
    fn open_blocking(&self) -> Result<Dataset> {
        let path = Path::new(&self.path);
        if self.open_options.is_empty() {
            Dataset::open(path)
        } else {
            let options_refs: Vec<&str> = self.open_options.iter().map(String::as_str).collect();
            Dataset::open_ex(
                path,
                DatasetOptions {
                    open_options: Some(&options_refs),
                    ..DatasetOptions::default()
                },
            )
        }
        .map_err(|e| {
            TileServerError::RasterError(format!(
                "failed to open {} {:?}: {e}",
                self.path, self.open_options
            ))
        })
    }

    /// Open an **uncounted one-shot** handle: not reserved against the byte
    /// budget, not added to `live`, never returned to the pool.
    ///
    /// This is the terminal fallback for `acquire_with_reclaim` once the
    /// budget is exhausted and nothing is reclaimable. It exists because the
    /// byte budget must never fail a renderable tile: a tile that would succeed
    /// without the cache must succeed *with* it, so under pressure the cache
    /// degrades to exactly the pre-cache behaviour (one uncached open per
    /// render). It is never worse than `main`.
    ///
    /// The open is bounded by the cache-wide `one_shot_permits` semaphore so
    /// uncounted handles cannot scale with incoming concurrency. The permit is
    /// carried by the returned handle and released on drop.
    ///
    /// MUST run on a blocking thread (GDAL open is blocking), like
    /// [`Self::acquire_blocking`].
    pub(crate) async fn open_uncached(self: &Arc<Self>) -> Result<PooledHandle> {
        // Bound concurrent uncounted opens cache-wide before touching GDAL.
        let permit = Arc::clone(&self.one_shot_permits)
            .acquire_owned()
            .await
            .map_err(|e| TileServerError::RasterError(format!("one-shot semaphore closed: {e}")))?;

        let pool = Arc::clone(self);
        let dataset = tokio::task::spawn_blocking(move || pool.open_blocking())
            .await
            .map_err(|e| TileServerError::RasterError(format!("task failed: {e}")))??;

        Ok(PooledHandle {
            dataset: Some(dataset),
            pool: Arc::clone(self),
            uncached: true,
            _permit: Some(permit),
        })
    }
}

pub struct DatasetCache {
    inner: Cache<String, Arc<Pool>>,
    max_bytes: u64,
    /// This cache's live-bytes total (sum of `live * cost_bytes` over every
    /// pool it owns). Each pool holds a clone and reserves/releases against it
    /// via [`Pool::reserve_budget`] and `Drop for Pool`.
    live_bytes: Arc<AtomicU64>,
    /// Cache-wide cap on simultaneous **uncounted** one-shot fallback opens.
    /// Each pool holds a clone; a one-shot open acquires a permit for the life
    /// of the returned handle.
    one_shot_permits: Arc<Semaphore>,
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
            .support_invalidation_closures()
            .build();
        Self {
            inner,
            max_bytes: 0,
            live_bytes: Arc::new(AtomicU64::new(0)),
            one_shot_permits: Arc::new(Semaphore::new(one_shot_limit())),
        }
    }

    /// Create a cache whose *retained* estimated-weight budget is `max_bytes`.
    /// When `max_bytes == 0` the cache falls back to an entry-count bound of
    /// `capacity` entries (legacy behaviour).
    ///
    /// The budget bounds retention, not liveness: once it is full, further
    /// renders fall back to uncounted one-shot opens
    /// ([`Pool::open_uncached`]) rather than failing, so a smaller budget only
    /// costs reopen latency — it never refuses a tile.
    ///
    /// Note moka has a **single** `max_capacity`. With a weigher installed it
    /// is interpreted as weight units, so the byte budget is the only active
    /// bound; no separate entry cap is enforced alongside it. The weigher
    /// reads each pool's *current* live byte total, so it tracks a pool that
    /// grows after insertion rather than freezing at insert time.
    #[must_use]
    pub fn with_max_bytes(capacity: u64, ttl: Duration, max_bytes: u64) -> Self {
        if max_bytes > 0 {
            let inner = Cache::builder()
                .max_capacity(max_bytes)
                .weigher(|_k: &String, pool: &Arc<Pool>| -> u32 {
                    // Weight is the pool's *live* byte total, read at
                    // eviction time. moka re-evaluates the weigher on
                    // maintenance, so a pool that grows is accounted for.
                    //
                    // A freshly created pool weighs 0 here, so moka sees
                    // weight 1 until the first open. That is fine: this
                    // weigher is only an eviction hint, and `reserve_budget`
                    // is the real bound on memory. Do not try to tune moka's
                    // capacity for memory — tune `DEFAULT_MAX_BYTES` instead.
                    u32::try_from(pool.weighted_bytes())
                        .unwrap_or(u32::MAX)
                        .max(1)
                })
                .time_to_live(ttl)
                .support_invalidation_closures()
                .build();
            Self {
                inner,
                max_bytes,
                live_bytes: Arc::new(AtomicU64::new(0)),
                one_shot_permits: Arc::new(Semaphore::new(one_shot_limit())),
            }
        } else {
            Self::new(capacity, ttl)
        }
    }

    /// Fetch the pool for `key`, creating it (empty) on a miss.
    ///
    /// The pool is created lazily and handles are opened on first
    /// [`Pool::acquire_arc`]. Creation itself does not open a dataset, so a
    /// cache hit costs no I/O.
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::RasterError`] if the pool cannot be created
    /// (currently infallible; kept for API symmetry).
    pub async fn get_or_open_cached(
        &self,
        key: &str,
        path: &str,
        open_options: &[&str],
    ) -> Result<Arc<Pool>> {
        let path_owned = path.to_string();
        let options_owned: Vec<String> = open_options.iter().map(|s| s.to_string()).collect();
        let max_bytes = self.max_bytes;
        let live_bytes = Arc::clone(&self.live_bytes);
        let one_shot_permits = Arc::clone(&self.one_shot_permits);

        // `try_get_with` runs the initialiser at most once per key even under
        // concurrent misses, so two cold requests do not both build a pool
        // (and, more importantly, do not both race to open the first handle).
        self.inner
            .try_get_with(key.to_string(), async move {
                Ok::<Arc<Pool>, TileServerError>(Arc::new(Pool::new(
                    path_owned,
                    options_owned,
                    max_bytes,
                    live_bytes,
                    one_shot_permits,
                )))
            })
            .await
            .map_err(|e: Arc<TileServerError>| {
                TileServerError::RasterError(format!("failed to create pool: {e}"))
            })
    }

    /// Fetch the base pool for `path`.
    ///
    /// # Errors
    ///
    /// See [`Self::get_or_open_cached`].
    pub async fn get_or_open(&self, path: &str) -> Result<Arc<Pool>> {
        self.get_or_open_cached(path, path, &[]).await
    }

    /// Acquire a handle from `pool`, reclaiming idle pools if the cache's byte
    /// budget is exhausted, and **never failing a renderable tile** for budget
    /// reasons.
    ///
    /// [`Pool::acquire_blocking`] is synchronous and cannot reclaim on its own
    /// (reclamation awaits moka's maintenance). This wrapper is the async-side
    /// escalator:
    ///
    /// 1. Fast path — try the acquire; on success, done.
    /// 2. On [`TileServerError::BudgetExhausted`], evict the cache's idle
    ///    pools (fully and partially) and try once more. That is the common
    ///    case in the field: one STAC mosaic fills the budget with pools it is
    ///    no longer rendering from.
    /// 3. Still exhausted: wait a short bounded time ([`ONE_SHOT_WAIT_MS`]) for
    ///    a handle to be *returned* (the common transient case) and try once
    ///    more.
    /// 4. Still nothing: open an **uncounted one-shot** handle
    ///    ([`Pool::open_uncached`]), bounded by the cache-wide one-shot
    ///    semaphore. The tile renders; the cache degrades to the pre-cache
    ///    baseline rather than erroring.
    ///
    /// A tile must never fail because of the byte budget: whatever it would do
    /// without the cache, it must do with the cache. The budget governs what is
    /// *retained and reused*, not whether a render succeeds.
    ///
    /// Reclaim runs on the async thread, so the blocking acquire is spawned as
    /// usual and the retry happens between attempts rather than inside one.
    ///
    /// # Errors
    ///
    /// Returns the underlying raster error if the handle cannot be opened even
    /// after reclaiming and the one-shot fallback (e.g. the file is genuinely
    /// unreadable).
    pub async fn acquire_with_reclaim(&self, pool: &Arc<Pool>) -> Result<PooledHandle> {
        // Note: this function cannot call `acquire_arc` and retry it directly,
        // because `PooledHandle` borrows no cache state — only the pool. That
        // is exactly what makes the retry safe: a failed acquire leaves the
        // pool unchanged, so a second attempt is a clean retry.
        match pool.acquire_arc().await {
            Ok(handle) => Ok(handle),
            Err(TileServerError::BudgetExhausted { .. }) => {
                // Step 2: give back idle bytes from other pools and retry.
                // Reclaim can free fewer bytes than this pool needs (an 8 MB
                // overview handle when a 16 MB base handle is wanted), so a
                // second `BudgetExhausted` falls through to steps 3 and 4
                // instead of reaching the HTTP layer.
                if self.reclaim_idle().await > 0 {
                    match pool.acquire_arc().await {
                        Err(TileServerError::BudgetExhausted { .. }) => {}
                        other => return other,
                    }
                }

                // Step 3: nothing idle to reclaim. Wait briefly for a handle
                // to be returned — the common transient case — then retry.
                tokio::time::sleep(Duration::from_millis(ONE_SHOT_WAIT_MS)).await;
                match pool.acquire_arc().await {
                    Ok(handle) => Ok(handle),
                    // Step 4: still saturated. Fall back to an uncounted
                    // one-shot open so the tile renders. Never worse than the
                    // pre-cache baseline; the semaphore bounds the peak.
                    Err(TileServerError::BudgetExhausted { .. }) => pool.open_uncached().await,
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Fetch a pool for `key`, opened with `open_options`.
    ///
    /// Used by [`super::cog::CogSource`] to cache overview-level-opened
    /// datasets keyed by `{path}#ovr{level}` — mirroring rio-tiler's per-level
    /// reader cache. Each overview level keeps a *pool* of handles, so
    /// concurrent tiles at that level render in parallel.
    ///
    /// # Errors
    ///
    /// See [`Self::get_or_open_cached`].
    pub async fn get_or_open_with_options(
        &self,
        key: &str,
        path: &str,
        open_options: &[&str],
    ) -> Result<Arc<Pool>> {
        self.get_or_open_cached(key, path, open_options).await
    }

    /// Invalidate `path` and every overview key derived from it.
    ///
    /// A source has one base key (`path`) and up to one key per overview level
    /// (`{path}#ovr{n}`). Invalidating only the base key would leave every
    /// overview pool live, so this drops all of them.
    pub async fn invalidate(&self, path: &str) {
        let prefix = format!("{path}#ovr");
        let base = path.to_string();
        let _ = self
            .inner
            .invalidate_entries_if(move |k: &String, _v: &Arc<Pool>| {
                k == &base || k.starts_with(&prefix)
            });
    }

    #[must_use]
    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    /// Whether a specific key is currently cached (used by tests to confirm a
    /// `{path}#ovr{n}` pool was created).
    ///
    /// `#[cfg(test)]`: the cache's public surface is quota/observability
    /// (`weighted_size`, `entry_count`, `reclaim_idle`); key-level probing is
    /// an implementation detail only the tests need, so it is compiled out of
    /// the published API entirely. (`pub(crate)` would still trip
    /// `dead_code` in a non-test build, since the only caller is a test.)
    #[cfg(test)]
    #[must_use]
    pub(crate) fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    /// Current total estimated weight in bytes across all pools in this cache.
    ///
    /// This is this cache's live total the byte budget gates on, so it is the
    /// number to assert against in tests.
    #[must_use]
    pub fn weighted_size(&self) -> u64 {
        self.live_bytes.load(Ordering::Acquire)
    }

    /// Close the idle handles of every cached pool, refund their bytes, and
    /// return how many bytes that freed. A pool left with no open handle is
    /// evicted.
    ///
    /// This is the escape hatch for budget saturation: when one source (a
    /// STAC mosaic, typically) fills the retained budget with handles it is
    /// not rendering from, every *other* source would be refused a counted
    /// handle. (Refusal is not terminal even without reclaim —
    /// [`DatasetCache::acquire_with_reclaim`] falls back to an uncounted
    /// one-shot open — but reclaim keeps the cache doing useful work rather
    /// than degrading to uncached opens.)
    ///
    /// Only *idle* handles are closed. A borrowed handle is mid-render and
    /// stays open and counted; it returns to its pool on drop.
    ///
    /// The refund is per handle actually closed, never per pool. So the
    /// counter always equals the handles that are open, also when a render
    /// still holds an `Arc` to a pool that this pass evicts: that pool's
    /// remaining handles are refunded by `Drop for Pool` when its last `Arc`
    /// goes.
    ///
    /// Called from the error path, so it must not block on a render. Each
    /// pool's mutex is only held across a `mem::take` of its idle list; the
    /// handles close after the lock is released.
    pub async fn reclaim_idle(&self) -> u64 {
        // Snapshot first: `iter()` borrows the cache, and the borrow must not
        // be held across the awaits below.
        let candidates: Vec<(String, Arc<Pool>)> =
            self.inner.iter().map(|(k, v)| (k.to_string(), v)).collect();

        let mut reclaimed = 0u64;
        for (key, pool) in candidates {
            let taken = pool.take_idle_handles();
            if taken == 0 {
                continue;
            }
            pool.release_partial(taken);
            reclaimed = reclaimed.saturating_add(taken as u64 * pool.cost_bytes);
            if pool.live_handles() == 0 {
                self.inner.invalidate(&key).await;
            }
        }

        if reclaimed > 0 {
            // Let moka finish its eviction bookkeeping now.
            self.inner.run_pending_tasks().await;
        }
        reclaimed
    }
}

static GLOBAL: OnceLock<DatasetCache> = OnceLock::new();

/// Access the process-global [`DatasetCache`].
///
/// Lazily initialises with a 10-minute TTL and a retained-byte budget of
/// [`DEFAULT_MAX_BYTES`]. The budget bounds what the cache *holds*, not what it
/// serves: once it is full, new renders fall back to uncached opens instead of
/// failing, so the budget keeps a remote-COG source (which opens one dataset
/// per overview level) from pinning gigabytes without ever gating a tile.
pub fn global() -> &'static DatasetCache {
    GLOBAL.get_or_init(|| {
        let max_bytes = resolve_max_bytes();
        DatasetCache::with_max_bytes(
            0, // byte budget is the only bound; no entry cap
            Duration::from_secs(DEFAULT_TTL_SECONDS),
            max_bytes,
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn failed_open_keeps_empty_pool() {
        let cache = DatasetCache::new(10, Duration::from_secs(60));
        let pool = cache.get_or_open("/nonexistent/path.tif").await.unwrap();
        // The pool exists but opening a handle must fail.
        assert!(pool.acquire_arc().await.is_err());
        cache.inner.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 1, "pool is cached on first request");
    }

    #[tokio::test]
    async fn hit_returns_same_pool() {
        let Some((_dir, p)) = tiny_gtiff("tiny.tif") else {
            eprintln!("skip: GTiff driver unavailable");
            return;
        };
        let cache = DatasetCache::new(10, Duration::from_secs(60));
        let first = cache.get_or_open(&p).await.expect("first");
        let second = cache.get_or_open(&p).await.expect("second");
        assert!(Arc::ptr_eq(&first, &second), "same pool on hit");
        cache.inner.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn acquire_release_reuses_handle() {
        let Some((_dir, p)) = tiny_gtiff("tiny.tif") else {
            return;
        };
        let cache = DatasetCache::new(10, Duration::from_secs(60));
        let pool = cache.get_or_open(&p).await.expect("pool");
        {
            let h = pool.acquire_arc().await.expect("first acquire");
            assert_eq!(pool.live_handles(), 1);
            let _ = h.dataset();
        } // handle dropped -> returned to idle
        assert_eq!(pool.live_handles(), 1, "handle stays open in the pool");
        let h2 = pool.acquire_arc().await.expect("second acquire");
        assert_eq!(pool.live_handles(), 1, "reused, not reopened");
        drop(h2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_acquires_get_distinct_handles() {
        // The regression this guards: N concurrent renders at one key must not
        // serialise on a single handle. With a pool, each concurrent borrower
        // holds a distinct handle.
        let Some((_dir, p)) = tiny_gtiff("tiny.tif") else {
            return;
        };
        let cache = DatasetCache::new(10, Duration::from_secs(60));
        let pool = cache.get_or_open(&p).await.expect("pool");

        let n = pool_limit().clamp(2, 4);
        let mut held: Vec<PooledHandle> = Vec::new();
        for _ in 0..n {
            held.push(pool.acquire_arc().await.expect("acquire"));
        }
        assert_eq!(
            pool.live_handles(),
            n,
            "each concurrent borrower holds its own handle"
        );
        // Distinct handles: their raw pointers differ.
        let ptrs: Vec<*const Dataset> = held.iter().map(|h| h.dataset() as *const _).collect();
        for i in 0..ptrs.len() {
            for j in (i + 1)..ptrs.len() {
                assert_ne!(ptrs[i], ptrs[j], "handles must be distinct");
            }
        }
    }

    #[tokio::test]
    async fn base_and_overview_keys_are_distinct_entries() {
        let Some((_dir, p)) = tiny_gtiff("tiny.tif") else {
            return;
        };
        let cache = DatasetCache::new(10, Duration::from_secs(60));
        let base = cache.get_or_open(&p).await.expect("base");
        let ovr_key = format!("{p}#ovr0");
        let ovr = cache
            .get_or_open_with_options(&ovr_key, &p, &[])
            .await
            .expect("ovr");
        cache.inner.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 2, "distinct keys are distinct entries");
        assert!(!Arc::ptr_eq(&base, &ovr), "distinct pools per key");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn weighted_size_counts_live_handles() {
        let Some((_dir, p)) = tiny_gtiff("tiny.tif") else {
            return;
        };
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), DEFAULT_MAX_BYTES);
        let pool = cache.get_or_open(&p).await.expect("pool");
        assert_eq!(pool.weighted_bytes(), 0, "empty pool weighs nothing");
        let h = pool.acquire_arc().await.expect("acquire");
        assert_eq!(
            pool.weighted_bytes(),
            BASE_COST_BYTES,
            "one base handle costs BASE_COST_BYTES"
        );
        assert_eq!(cache.weighted_size(), BASE_COST_BYTES);
        drop(h);
        // Handle is returned to idle, still open, so still counted.
        assert_eq!(pool.weighted_bytes(), BASE_COST_BYTES);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn byte_budget_is_enforced() {
        // The counted path refuses to open past the budget: the bound is
        // enforced, not merely reported. (`acquire_with_reclaim` then escalates
        // to a one-shot fallback — covered separately — so this asserts the
        // *counted* path only.)
        let Some((_dir, p)) = tiny_gtiff("tiny.tif") else {
            return;
        };
        let budget = BASE_COST_BYTES / 2;
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), budget);
        let pool = cache.get_or_open(&p).await.expect("pool");
        assert!(
            matches!(
                pool.acquire_arc().await,
                Err(TileServerError::BudgetExhausted { .. })
            ),
            "opening past the byte budget must be refused with BudgetExhausted"
        );
        assert_eq!(pool.live_handles(), 0, "no handle leaked past the budget");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn budget_too_small_for_one_handle_still_renders_via_fallback() {
        // A pool whose single handle costs more than the whole budget can never
        // be *cached* — `acquire_arc` refuses with `BudgetExhausted` forever.
        // But a tile must never fail for budget reasons, so
        // `acquire_with_reclaim` must still return a usable handle via the
        // uncounted one-shot fallback.
        let Some((_dir, p)) = tiny_gtiff("tiny.tif") else {
            return;
        };
        let budget = BASE_COST_BYTES - 1;
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), budget);
        let pool = cache.get_or_open(&p).await.expect("pool");

        // Counted path refuses.
        assert!(
            matches!(
                pool.acquire_arc().await,
                Err(TileServerError::BudgetExhausted { .. })
            ),
            "an unfittable handle must be refused on the counted path"
        );
        // Reclaiming cannot help — the budget is not held by anyone else.
        assert_eq!(cache.reclaim_idle().await, 0);

        // The escalator still delivers a renderable handle, uncounted.
        let handle = cache
            .acquire_with_reclaim(&pool)
            .await
            .expect("one-shot fallback must return a handle");
        assert!(handle.is_uncached(), "fallback handle is uncounted");
        assert!(handle.dataset().raster_size().0 > 0, "handle is usable");
        assert_eq!(cache.weighted_size(), 0, "fallback handle is not counted");
        assert_eq!(pool.live_handles(), 0, "fallback never enters the pool");
        drop(handle);
        assert_eq!(cache.weighted_size(), 0, "drop releases nothing counted");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn reclaim_idle_frees_budget_for_another_source() {
        // Regression for the production lockout: one source fills the whole
        // byte budget with pools it is no longer rendering from, and every
        // other source is refused forever. `reclaim_idle` must hand the
        // budget back so the second source can open.
        let Some((_dir1, p1)) = tiny_gtiff("first.tif") else {
            return;
        };
        let Some((_dir2, p2)) = tiny_gtiff("second.tif") else {
            return;
        };
        // Budget fits exactly one base handle.
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), BASE_COST_BYTES);

        let pool1 = cache.get_or_open(&p1).await.expect("pool1");
        {
            // Hold and release, so pool1 owns one *idle* handle and its bytes.
            let _h = pool1.acquire_arc().await.expect("acquire pool1");
        }
        assert_eq!(cache.weighted_size(), BASE_COST_BYTES, "budget is full");

        // A second source cannot open: the budget is entirely pool1's.
        let pool2 = cache.get_or_open(&p2).await.expect("pool2");
        assert!(
            matches!(
                pool2.acquire_arc().await,
                Err(TileServerError::BudgetExhausted { .. })
            ),
            "second source must be refused while the budget is full"
        );

        // Reclaiming the idle pool frees the budget...
        let reclaimed = cache.reclaim_idle().await;
        assert_eq!(reclaimed, BASE_COST_BYTES, "idle pool's bytes are freed");
        assert_eq!(cache.weighted_size(), 0, "budget is released");

        // ...and the second source now opens. This is the behaviour the old
        // hard-error path made impossible: the server was stuck on 500s until
        // restarted.
        let handle2 = pool2.acquire_arc().await.expect("acquire after reclaim");
        assert_eq!(cache.weighted_size(), BASE_COST_BYTES);
        drop(handle2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn reclaim_idle_skips_pools_with_borrowed_handles() {
        // Evicting a pool with a borrowed handle would orphan that handle: the
        // pool's `Drop` refunds `live * cost_bytes` (including the borrowed
        // one), and the handle then returns to a pool no longer in the cache.
        // `reclaim_idle` must therefore leave busy pools alone.
        let Some((_dir, p)) = tiny_gtiff("busy.tif") else {
            return;
        };
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), BASE_COST_BYTES);
        let pool = cache.get_or_open(&p).await.expect("pool");

        let held = pool.acquire_arc().await.expect("acquire");
        assert_eq!(cache.reclaim_idle().await, 0, "busy pool is not evicted");
        assert_eq!(cache.weighted_size(), BASE_COST_BYTES, "budget untouched");

        drop(held);
        // Now fully idle, so it becomes reclaimable.
        assert_eq!(cache.reclaim_idle().await, BASE_COST_BYTES);
        assert_eq!(cache.weighted_size(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn reclaim_frees_partially_idle_pool_without_evicting_it() {
        // The field saturation this fixes: a dynamic STAC mosaic holds every
        // pool mid-render, each with one handle borrowed and one idle. Under
        // the old fully-idle-only rule nothing was reclaimable, so
        // `acquire_with_reclaim` re-raised `BudgetExhausted` immediately and
        // the log filled with budget warnings while coverage degraded.
        //
        // A partially idle pool must instead hand back the *free* handles'
        // bytes while keeping the borrowed handles counted and the pool cached.
        let Some((_dir1, p1)) = tiny_gtiff("busy1.tif") else {
            return;
        };
        let Some((_dir2, p2)) = tiny_gtiff("waiting.tif") else {
            return;
        };
        // Budget fits exactly two base handles.
        let budget = BASE_COST_BYTES * 2;
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), budget);

        let pool1 = cache.get_or_open(&p1).await.expect("pool1");
        // Two handles live: one idle, one borrowed.
        let held1 = pool1.acquire_arc().await.expect("first handle");
        let idle1 = pool1.acquire_arc().await.expect("second handle");
        drop(idle1); // return to the idle list, bytes still counted
        assert_eq!(pool1.live_handles(), 2);
        assert_eq!(cache.weighted_size(), budget, "budget is full");

        // A second source is refused while pool1 holds both handles.
        let pool2 = cache.get_or_open(&p2).await.expect("pool2");
        assert!(
            matches!(
                pool2.acquire_arc().await,
                Err(TileServerError::BudgetExhausted { .. })
            ),
            "second source must be refused while the budget is full"
        );

        // Reclaim: pool1 is partially idle (1 of 2 free), so exactly one
        // handle's worth of bytes comes back and the borrowed one stays.
        let reclaimed = cache.reclaim_idle().await;
        assert_eq!(reclaimed, BASE_COST_BYTES, "only the idle handle is freed");
        assert_eq!(
            cache.weighted_size(),
            BASE_COST_BYTES,
            "the borrowed handle is still counted"
        );
        assert_eq!(pool1.live_handles(), 1, "the idle handle was closed");
        assert!(
            cache.contains_key(&p1),
            "a partially idle pool must stay cached, not be evicted"
        );

        // The freed bytes are enough for the blocked source.
        let handle2 = pool2.acquire_arc().await.expect("acquire after reclaim");

        // The render that borrowed pool1's handle still holds a valid dataset.
        assert!(held1.dataset().raster_size().0 > 0, "borrowed handle alive");

        drop(handle2);
        drop(held1);
        // Both pools are now fully idle: pool1 holds one handle, pool2 one.
        // One reclaim pass frees both.
        assert_eq!(cache.reclaim_idle().await, BASE_COST_BYTES * 2);
        assert!(!cache.contains_key(&p1), "fully idle pool is evicted");
        assert!(!cache.contains_key(&p2), "fully idle pool is evicted");
        assert_eq!(cache.weighted_size(), 0);
    }

    #[tokio::test]
    async fn invalidate_drops_base_and_overview_keys() {
        let Some((_dir, p)) = tiny_gtiff("tiny.tif") else {
            return;
        };
        let cache = DatasetCache::new(10, Duration::from_secs(60));
        cache.get_or_open(&p).await.expect("base");
        cache
            .get_or_open_with_options(&format!("{p}#ovr0"), &p, &[])
            .await
            .expect("ovr0");
        cache
            .get_or_open_with_options(&format!("{p}#ovr1"), &p, &[])
            .await
            .expect("ovr1");
        cache.inner.run_pending_tasks().await;
        assert_eq!(cache.entry_count(), 3);
        cache.invalidate(&p).await;
        cache.inner.run_pending_tasks().await;
        assert_eq!(
            cache.entry_count(),
            0,
            "invalidate must drop the base key AND every {{path}}#ovr* key"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn evicted_pool_releases_its_bytes() {
        // Regression: `Pool` had no `Drop`, so an evicted pool (TTL or
        // `invalidate`) closed its idle handles but left `live * cost_bytes`
        // in the cache's live-bytes counter. The budget then filled up and
        // stayed full, and the server stopped opening datasets after a while.
        //
        // This test exercises `Drop for Pool` directly (a pool with an idle
        // handle), with its own standalone live-bytes counter, so that
        // dropping the last `Arc<Pool>` fires `Drop` deterministically — no
        // dependence on moka's *async* eviction timing, which is deliberately
        // out of scope here. The assertions are against this counter, so there
        // is no process-global state to race — no `#[serial]` needed.
        let Some((_dir, p)) = tiny_gtiff("tiny.tif") else {
            return;
        };
        let live_bytes = Arc::new(AtomicU64::new(0));
        let pool = Arc::new(Pool::new(
            p.clone(),
            vec![],
            DEFAULT_MAX_BYTES,
            Arc::clone(&live_bytes),
            Arc::new(Semaphore::new(one_shot_limit())),
        ));
        {
            let _h = pool.acquire_arc().await.expect("acquire");
            assert_eq!(
                live_bytes.load(Ordering::Acquire),
                BASE_COST_BYTES,
                "one live handle is counted"
            );
        } // handle dropped -> returned to idle, still counted
        assert_eq!(
            live_bytes.load(Ordering::Acquire),
            BASE_COST_BYTES,
            "idle handle stays counted while the pool is alive"
        );

        // Drop the only strong `Arc<Pool>`. `Drop for Pool` must release the
        // idle handle's bytes.
        drop(pool);
        assert_eq!(
            live_bytes.load(Ordering::Acquire),
            0,
            "dropping the pool must release the idle handle's bytes"
        );
    }

    #[tokio::test]
    async fn global_is_stable_across_calls() {
        assert!(std::ptr::eq(global(), global()));
    }

    /// Test 1 (reviewer's step 1-4): a busy source fills the budget, goes idle,
    /// and a blocked source still gets a handle — first via reclaim, and after
    /// everything truly drains, `weighted_size()` returns to 0.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn acquire_with_reclaim_returns_zero_after_all_handles_drop() {
        let Some((_dir_a, pa)) = tiny_gtiff("held_by_source.tif") else {
            return;
        };
        let Some((_dir_b, pb)) = tiny_gtiff("waiting_source.tif") else {
            return;
        };
        // Budget fits exactly one base handle.
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), BASE_COST_BYTES);

        // Pool A held in an `Arc` like a source holds it, then rendered and
        // left idle: its handle is back in the idle list, bytes still counted.
        let pool_a = cache.get_or_open(&pa).await.expect("pool_a");
        {
            let _h = cache
                .acquire_with_reclaim(&pool_a)
                .await
                .expect("a renders");
        }
        assert_eq!(cache.weighted_size(), BASE_COST_BYTES, "budget is full");

        // Pool B cannot open while A sits on the budget...
        let pool_b = cache.get_or_open(&pb).await.expect("pool_b");
        // ...but the reclaiming wrapper frees A's idle bytes and B renders.
        let hb = cache
            .acquire_with_reclaim(&pool_b)
            .await
            .expect("B renders after reclaim");
        assert!(!hb.is_uncached(), "reclaim should yield a counted handle");

        drop(hb);
        // Drain A too, then a reclaim pass must free everything.
        cache.reclaim_idle().await;
        cache.reclaim_idle().await;
        assert_eq!(
            cache.weighted_size(),
            0,
            "once every handle has dropped, the counter returns to 0"
        );
        assert!(!cache.contains_key(&pa), "A is evicted once fully idle");
        assert_eq!(
            pool_a.live_handles(),
            0,
            "the held pool has no open handle left behind the counter"
        );
    }

    /// Test 2: when nothing is reclaimable (all handles busy) the fallback must
    /// still deliver a handle, uncounted, so the tile never fails.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn acquire_with_reclaim_falls_back_when_all_handles_busy() {
        let Some((_dir_a, pa)) = tiny_gtiff("busy_source.tif") else {
            return;
        };
        let Some((_dir_b, pb)) = tiny_gtiff("starved_source.tif") else {
            return;
        };
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), BASE_COST_BYTES);

        // Fill the whole budget with a *borrowed* (busy) handle so reclaim
        // finds nothing to free.
        let pool_a = cache.get_or_open(&pa).await.expect("pool_a");
        let _held_a = pool_a.acquire_arc().await.expect("busy handle");
        assert_eq!(cache.weighted_size(), BASE_COST_BYTES, "budget is full");

        let pool_b = cache.get_or_open(&pb).await.expect("pool_b");
        let hb = cache
            .acquire_with_reclaim(&pool_b)
            .await
            .expect("fallback must return a handle, not fail");
        assert!(
            hb.is_uncached(),
            "nothing was reclaimable, so it is one-shot"
        );
        assert!(
            hb.dataset().raster_size().0 > 0,
            "the fallback handle is usable"
        );
        // Uncounted: the budget still reflects only A's busy handle.
        assert_eq!(cache.weighted_size(), BASE_COST_BYTES);
        drop(hb);
        assert_eq!(
            cache.weighted_size(),
            BASE_COST_BYTES,
            "dropping the fallback handle releases nothing counted"
        );
    }

    /// Test 3: a one-shot handle is uncounted, closes on drop, and releases its
    /// semaphore permit (so a later fallback does not block behind it).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn one_shot_handle_is_uncounted_and_releases_its_permit() {
        let Some((_dir, p)) = tiny_gtiff("one_shot.tif") else {
            return;
        };
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), DEFAULT_MAX_BYTES);
        let pool = cache.get_or_open(&p).await.expect("pool");

        let h = pool.open_uncached().await.expect("open one-shot");
        assert!(h.is_uncached());
        assert_eq!(cache.weighted_size(), 0, "one-shot is not counted");
        assert_eq!(pool.live_handles(), 0, "one-shot never enters the pool");
        drop(h);
        assert_eq!(cache.weighted_size(), 0);

        // Permit was released: a second one-shot open succeeds without
        // blocking (capacity is >= 2 on any test host).
        let h2 = pool.open_uncached().await.expect("permit released");
        drop(h2);
    }

    /// Test 5: the one-shot semaphore caps concurrent uncounted opens. Uses a
    /// dedicated cache whose semaphore is shrunk to 1 for the assertion.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn one_shot_semaphore_caps_concurrent_fallback_opens() {
        let Some((_dir, p)) = tiny_gtiff("capped.tif") else {
            return;
        };
        let mut cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), DEFAULT_MAX_BYTES);
        // Shrink the cap to 1 so the test is deterministic and fast.
        cache.one_shot_permits = Arc::new(Semaphore::new(1));
        let cache = Arc::new(cache);

        let pool = cache.get_or_open(&p).await.expect("pool");

        // Take the only permit directly. A subsequent one-shot open must wait
        // (not open a second uncounted handle).
        let permit = Arc::clone(&cache.one_shot_permits)
            .acquire_owned()
            .await
            .expect("permit");

        let cache2 = Arc::clone(&cache);
        let pool2 = Arc::clone(&pool);
        let waiter = tokio::spawn(async move { pool2.open_uncached().await.map(|_| ()) });

        // Give the waiter a chance to run; it must still be blocked.
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!waiter.is_finished(), "one-shot open waits for a permit");

        // Release the permit; the waiter proceeds.
        drop(permit);
        let opened = tokio::time::timeout(Duration::from_secs(5), waiter)
            .await
            .expect("waiter must proceed once a permit frees")
            .expect("join");
        assert!(opened.is_ok(), "open succeeds after the permit frees");
        // `cache2` keeps the cache strong-ref alive across the spawn, matching
        // how a source holds its `Arc<DatasetCache>` while a render is in flight.
        assert!(Arc::strong_count(&cache2) >= 1);
    }

    /// Test 6: a pool that stops rendering must not sit on more than
    /// [`IDLE_CAP`] handles. The excess is closed on drop and its bytes
    /// refunded, so the cache does not hold a whole budget of warm-but-idle
    /// handles while another source needs it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn idle_cap_closes_and_refunds_handles_beyond_the_cap() {
        let Some((_dir, p)) = tiny_gtiff("idle_cap.tif") else {
            return;
        };
        // Budget comfortably fits more than IDLE_CAP handles so the cap, not
        // the budget, is what bounds the idle list.
        let budget = BASE_COST_BYTES * 8;
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), budget);
        let pool = cache.get_or_open(&p).await.expect("pool");

        // Acquire IDLE_CAP + 2 handles, then drop them all. Only IDLE_CAP may
        // remain idle; the rest are closed and refunded.
        let wanted = IDLE_CAP + 2;
        let mut handles = Vec::new();
        for _ in 0..wanted {
            handles.push(pool.acquire_arc().await.expect("acquire"));
        }
        assert_eq!(pool.live_handles(), wanted, "all borrowed, all counted");
        assert_eq!(
            cache.weighted_size(),
            wanted as u64 * BASE_COST_BYTES,
            "full footprint while borrowed"
        );

        drop(handles);

        assert_eq!(
            pool.live_handles(),
            IDLE_CAP,
            "idle list is capped at IDLE_CAP"
        );
        assert_eq!(
            cache.weighted_size(),
            IDLE_CAP as u64 * BASE_COST_BYTES,
            "only the retained handles stay counted; the rest were refunded"
        );
    }

    /// Regression for the reviewer's budget=4 drift: fill a pool to four
    /// handles, drop them all, and confirm the counter returns to the true
    /// retained footprint (closes the §3.4 accounting concern).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn counter_returns_to_zero_after_a_saturated_pool_drains() {
        let Some((_dir, p)) = tiny_gtiff("drift.tif") else {
            return;
        };
        let budget = BASE_COST_BYTES * 4;
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), budget);
        let pool = cache.get_or_open(&p).await.expect("pool");

        let mut handles = Vec::new();
        for _ in 0..4 {
            handles.push(pool.acquire_arc().await.expect("acquire"));
        }
        assert_eq!(cache.weighted_size(), budget, "all four handles counted");
        drop(handles);

        // The pool keeps at most IDLE_CAP warm handles; a reclaim pass evicts
        // the fully-idle pool and refunds the rest, so the counter hits 0.
        assert_eq!(
            cache.reclaim_idle().await,
            IDLE_CAP as u64 * BASE_COST_BYTES
        );
        assert!(!cache.contains_key(&p), "fully idle pool is evicted");
        assert_eq!(
            cache.weighted_size(),
            0,
            "no residual bytes after every handle has dropped"
        );
    }

    /// `resolve_max_bytes` must prefer a valid env override, and must fall
    /// back to the default for anything unparseable or non-positive.
    ///
    /// Run in one test to avoid env races between parallel test threads.
    #[test]
    fn resolve_max_bytes_honours_env_override() {
        // SAFETY: `set_var`/`remove_var` are unsafe in edition 2024 because a
        // concurrent `getenv` in another thread could observe a torn value.
        // This test owns `MAX_BYTES_ENV` exclusively — no other test reads or
        // writes it (it is not used by production code, only by
        // `resolve_max_bytes` at cache init), and we restore the previous
        // value before returning so the process environment is unchanged.
        unsafe {
            let prev = std::env::var(MAX_BYTES_ENV).ok();

            std::env::set_var(MAX_BYTES_ENV, "1073741824");
            assert_eq!(resolve_max_bytes(), 1024 * 1024 * 1024);

            std::env::set_var(MAX_BYTES_ENV, "  268435456  ");
            assert_eq!(resolve_max_bytes(), 256 * 1024 * 1024, "whitespace trimmed");

            std::env::set_var(MAX_BYTES_ENV, "0");
            assert_eq!(resolve_max_bytes(), DEFAULT_MAX_BYTES, "zero is rejected");

            std::env::set_var(MAX_BYTES_ENV, "not-a-number");
            assert_eq!(resolve_max_bytes(), DEFAULT_MAX_BYTES, "garbage rejected");

            std::env::remove_var(MAX_BYTES_ENV);
            assert_eq!(resolve_max_bytes(), DEFAULT_MAX_BYTES, "unset uses default");

            match prev {
                Some(v) => std::env::set_var(MAX_BYTES_ENV, v),
                None => std::env::remove_var(MAX_BYTES_ENV),
            }
        }
    }

    /// A GTiff with one internal overview, so an `OVERVIEW_LEVEL=0` pool can
    /// open it.
    fn gtiff_with_overview(name: &str) -> Option<(tempfile::TempDir, String)> {
        use gdal::DriverManager;
        let dir = tempfile::tempdir().ok()?;
        let path = dir.path().join(name);
        let driver = DriverManager::get_driver_by_name("GTiff").ok()?;
        let mut ds = driver
            .create_with_band_type::<u8, _>(&path, 64, 64, 1)
            .ok()?;
        ds.build_overviews("NEAREST", &[2], &[]).ok()?;
        drop(ds);
        Some((dir, path.to_string_lossy().to_string()))
    }

    /// Sum of `live * cost` over `pools`: the bytes really held open.
    fn open_bytes(pools: &[&Arc<Pool>]) -> u64 {
        pools
            .iter()
            .map(|p| p.live_handles() as u64 * p.cost_bytes)
            .sum()
    }

    /// Reclaim frees an 8 MB overview handle, but the waiting base pool needs
    /// 16 MB. The retry after reclaim must not return `BudgetExhausted`: it
    /// falls through to the wait and the one-shot open, so the tile renders.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn partial_reclaim_too_small_still_falls_back() {
        let Some((_dx, px)) = gtiff_with_overview("busy_base.tif") else {
            return;
        };
        let Some((_do, po)) = gtiff_with_overview("idle_overview.tif") else {
            return;
        };
        let Some((_dy, py)) = gtiff_with_overview("new_base.tif") else {
            return;
        };
        let cache = DatasetCache::with_max_bytes(
            0,
            Duration::from_secs(60),
            BASE_COST_BYTES + OVERVIEW_COST_BYTES,
        );

        let x = cache.get_or_open(&px).await.expect("pool x");
        let _busy = cache.acquire_with_reclaim(&x).await.expect("x renders");
        let key = format!("{po}#ovr0");
        let o = cache
            .get_or_open_with_options(&key, &po, &["OVERVIEW_LEVEL=0"])
            .await
            .expect("pool o");
        drop(cache.acquire_with_reclaim(&o).await.expect("o renders"));
        assert_eq!(
            cache.weighted_size(),
            BASE_COST_BYTES + OVERVIEW_COST_BYTES,
            "budget is full: one busy base handle, one idle overview handle"
        );

        let y = cache.get_or_open(&py).await.expect("pool y");
        let handle = cache
            .acquire_with_reclaim(&y)
            .await
            .expect("a tile must not fail because reclaim freed too little");
        assert!(
            handle.is_uncached(),
            "8 MB freed cannot fit a 16 MB base handle, so this is the one-shot"
        );
        drop(handle);
        assert_eq!(
            cache.weighted_size(),
            open_bytes(&[&x, &o, &y]),
            "the counter matches the handles that are open"
        );
    }

    /// A render can still hold an `Arc` to a pool that reclaim evicts. The
    /// counter must keep matching the handles that are really open, and the
    /// pool's remaining bytes are refunded when its last `Arc` goes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn counter_matches_open_handles_when_an_evicted_pool_is_held() {
        let Some((_da, pa)) = gtiff_with_overview("held.tif") else {
            return;
        };
        let Some((_db, pb)) = gtiff_with_overview("other.tif") else {
            return;
        };
        let cache = DatasetCache::with_max_bytes(0, Duration::from_secs(60), 2 * BASE_COST_BYTES);

        let a = cache.get_or_open(&pa).await.expect("pool a");
        let h1 = cache.acquire_with_reclaim(&a).await.expect("a1");
        let h2 = cache.acquire_with_reclaim(&a).await.expect("a2");
        drop((h1, h2));
        assert_eq!(cache.weighted_size(), 2 * BASE_COST_BYTES, "budget is full");

        // B needs room: reclaim closes A's two idle handles and evicts A,
        // while `a` is still held here.
        let b = cache.get_or_open(&pb).await.expect("pool b");
        let hb = cache.acquire_with_reclaim(&b).await.expect("b renders");
        assert!(!cache.contains_key(&pa), "empty pool a is evicted");
        assert_eq!(a.live_handles(), 0, "reclaim closed a's idle handles");
        assert_eq!(cache.weighted_size(), open_bytes(&[&a, &b]));

        // The held, evicted pool keeps serving; every open is counted.
        for _ in 0..20 {
            drop(cache.acquire_with_reclaim(&a).await.expect("a renders"));
            assert_eq!(cache.weighted_size(), open_bytes(&[&a, &b]));
        }
        drop(hb);
        assert_eq!(cache.weighted_size(), open_bytes(&[&a, &b]));

        // Dropping the last `Arc` of the evicted pool refunds what it holds.
        let b_bytes = open_bytes(&[&b]);
        drop(a);
        assert_eq!(
            cache.weighted_size(),
            b_bytes,
            "a's handles are refunded on drop"
        );
    }

    /// Many concurrent renders over three pools with a one-handle budget:
    /// every acquire succeeds and the counter ends equal to the open handles.
    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn saturated_concurrent_acquires_never_fail() {
        let mut files = Vec::new();
        for i in 0..3 {
            let Some(f) = gtiff_with_overview(&format!("sat{i}.tif")) else {
                return;
            };
            files.push(f);
        }
        let cache = Arc::new(DatasetCache::with_max_bytes(
            0,
            Duration::from_secs(60),
            BASE_COST_BYTES,
        ));
        let mut tasks = Vec::new();
        for i in 0..96 {
            let cache = Arc::clone(&cache);
            let path = files[i % 3].1.clone();
            tasks.push(tokio::spawn(async move {
                let pool = cache.get_or_open(&path).await?;
                let handle = cache.acquire_with_reclaim(&pool).await?;
                tokio::time::sleep(Duration::from_millis(5)).await;
                drop(handle);
                Ok::<(), TileServerError>(())
            }));
        }
        for task in tasks {
            task.await
                .expect("task joins")
                .expect("no acquire fails under saturation");
        }
        let mut pools = Vec::new();
        for (_, path) in &files {
            pools.push(cache.get_or_open(path).await.expect("pool"));
        }
        let refs: Vec<&Arc<Pool>> = pools.iter().collect();
        assert_eq!(cache.weighted_size(), open_bytes(&refs));
    }
}
