//! In-process request statistics for the admin analytics dashboard (#602).
//!
//! The Prometheus pipeline keeps only bounded labels (zoom buckets, no tile
//! coordinates), so it cannot answer "which areas are popular" or "what
//! were the last requests". This store keeps that detail in memory, bounded
//! by fixed limits, and serves it as one JSON snapshot at
//! `GET /__admin/stats`.
//!
//! Collection is off by default (`[telemetry] dashboard = false`). When off,
//! [`record_tile`] returns after one relaxed atomic load. When on, each tile
//! request takes one uncontended mutex for a few counter updates.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::recorder::TileOutcome;

/// Upper bounds (milliseconds) of the latency histogram buckets. A last,
/// open bucket holds everything slower.
pub const LATENCY_BOUNDS_MS: [f64; 15] = [
    0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0,
];
const BUCKETS: usize = LATENCY_BOUNDS_MS.len() + 1;
/// Per-minute history kept for the request-rate chart (24 hours).
const MINUTES_KEPT: usize = 24 * 60;
/// Distinct tiles tracked for the "most requested" list.
const MAX_TRACKED_TILES: usize = 10_000;
/// Distinct heatmap cells tracked.
const MAX_HEAT_CELLS: usize = 20_000;
const TOP_TILES: usize = 20;
const MAX_ZOOM: usize = 30;

/// Dashboard collection settings (from `[telemetry]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardSettings {
    pub enabled: bool,
    /// Requests kept in the "recent requests" list.
    pub recent_requests: usize,
    /// Zoom level that heatmap cells are aggregated to.
    pub heatmap_zoom: u8,
}

impl Default for DashboardSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            recent_requests: 200,
            heatmap_zoom: 10,
        }
    }
}

/// One completed tile request, as the dashboard sees it.
#[derive(Debug, Clone, Copy)]
pub struct TileSample<'a> {
    pub source: &'a str,
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub bytes: usize,
    pub duration: Duration,
    pub outcome: TileOutcome,
}

/// `(source, z, x, y)`.
type TileKey = (String, u8, u32, u32);

#[derive(Debug, Default, Clone)]
struct SourceStats {
    requests: u64,
    hits: u64,
    misses: u64,
    not_found: u64,
    errors: u64,
    bytes: u64,
    latency: [u64; BUCKETS],
}

#[derive(Debug, Clone)]
struct Recent {
    at_unix_ms: u64,
    source: String,
    z: u8,
    x: u32,
    y: u32,
    outcome: TileOutcome,
    duration_ms: f64,
    bytes: usize,
}

#[derive(Debug)]
struct Inner {
    settings: DashboardSettings,
    started: Instant,
    sources: HashMap<String, SourceStats>,
    zooms: [u64; MAX_ZOOM + 1],
    /// `(unix minute, requests, errors)`, oldest first.
    minutes: VecDeque<(u64, u64, u64)>,
    heat: HashMap<(u32, u32), u64>,
    heat_dropped: u64,
    tiles: HashMap<TileKey, u64>,
    recent: VecDeque<Recent>,
}

impl Inner {
    fn new(settings: DashboardSettings) -> Self {
        Self {
            settings,
            started: Instant::now(),
            sources: HashMap::new(),
            zooms: [0; MAX_ZOOM + 1],
            minutes: VecDeque::with_capacity(MINUTES_KEPT),
            heat: HashMap::new(),
            heat_dropped: 0,
            tiles: HashMap::new(),
            recent: VecDeque::with_capacity(settings.recent_requests),
        }
    }

    fn record(&mut self, sample: &TileSample<'_>, now_unix_ms: u64) {
        let is_error = matches!(sample.outcome, TileOutcome::Error);
        let stats = self.sources.entry(sample.source.to_owned()).or_default();
        stats.requests += 1;
        match sample.outcome {
            TileOutcome::Hit => stats.hits += 1,
            TileOutcome::Miss => stats.misses += 1,
            TileOutcome::NotFound => stats.not_found += 1,
            TileOutcome::Error => stats.errors += 1,
        }
        stats.bytes += sample.bytes as u64;
        let ms = sample.duration.as_secs_f64() * 1000.0;
        stats.latency[bucket_index(ms)] += 1;

        self.zooms[usize::from(sample.z).min(MAX_ZOOM)] += 1;

        let minute = now_unix_ms / 60_000;
        match self.minutes.back_mut() {
            Some((m, requests, errors)) if *m == minute => {
                *requests += 1;
                *errors += u64::from(is_error);
            }
            _ => {
                if self.minutes.len() == MINUTES_KEPT {
                    self.minutes.pop_front();
                }
                self.minutes.push_back((minute, 1, u64::from(is_error)));
            }
        }

        let cell = heat_cell(sample.z, sample.x, sample.y, self.settings.heatmap_zoom);
        if let Some(count) = self.heat.get_mut(&cell) {
            *count += 1;
        } else if self.heat.len() < MAX_HEAT_CELLS {
            self.heat.insert(cell, 1);
        } else {
            self.heat_dropped += 1;
        }

        let key = (sample.source.to_owned(), sample.z, sample.x, sample.y);
        if let Some(count) = self.tiles.get_mut(&key) {
            *count += 1;
        } else if self.tiles.len() < MAX_TRACKED_TILES {
            self.tiles.insert(key, 1);
        }

        if self.settings.recent_requests > 0 {
            if self.recent.len() >= self.settings.recent_requests {
                self.recent.pop_front();
            }
            self.recent.push_back(Recent {
                at_unix_ms: now_unix_ms,
                source: sample.source.to_owned(),
                z: sample.z,
                x: sample.x,
                y: sample.y,
                outcome: sample.outcome,
                duration_ms: ms,
                bytes: sample.bytes,
            });
        }
    }
}

static ENABLED: AtomicBool = AtomicBool::new(false);
static STORE: OnceLock<Mutex<Inner>> = OnceLock::new();

fn store() -> MutexGuard<'static, Inner> {
    STORE
        .get_or_init(|| Mutex::new(Inner::new(DashboardSettings::default())))
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

/// Apply settings. Turning collection on, or changing a setting, clears the
/// collected data; calling again with the same settings keeps it, so a
/// config reload that does not touch `[telemetry]` loses nothing.
pub fn configure(settings: DashboardSettings) {
    let mut inner = store();
    apply_settings(&mut inner, settings);
    ENABLED.store(settings.enabled, Ordering::Relaxed);
}

fn apply_settings(inner: &mut Inner, settings: DashboardSettings) {
    if inner.settings != settings {
        *inner = Inner::new(settings);
    }
}

/// Whether collection is on.
#[must_use]
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Record one tile request (no-op when collection is off).
pub fn record_tile(sample: &TileSample<'_>) {
    if !is_enabled() {
        return;
    }
    // The flag is read without the lock; `record_if_enabled` re-checks
    // under it so a request racing a `configure` that turns collection off
    // records nothing.
    record_if_enabled(&mut store(), sample, unix_ms());
}

fn record_if_enabled(inner: &mut Inner, sample: &TileSample<'_>, now_unix_ms: u64) {
    if inner.settings.enabled {
        inner.record(sample, now_unix_ms);
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn bucket_index(ms: f64) -> usize {
    LATENCY_BOUNDS_MS
        .iter()
        .position(|&bound| ms <= bound)
        .unwrap_or(BUCKETS - 1)
}

/// The heatmap cell (a tile at `zoom`) for tile `z/x/y`: the cell that
/// contains it when `z > zoom`, otherwise the cell at its center (so a
/// low-zoom request lands in the middle of the area it covers, not its
/// north-west corner).
fn heat_cell(z: u8, x: u32, y: u32, zoom: u8) -> (u32, u32) {
    if z > zoom {
        let shift = u32::from(z - zoom);
        (x >> shift, y >> shift)
    } else {
        let shift = u32::from(zoom - z);
        let half = (1u32 << shift) >> 1;
        ((x << shift) + half, (y << shift) + half)
    }
}

/// Center of tile `x/y` at zoom `z` as `[lon, lat]`.
fn tile_center(z: u8, x: u32, y: u32) -> [f64; 2] {
    let n = f64::from(1u32 << z.min(31));
    let lon = (f64::from(x) + 0.5) / n * 360.0 - 180.0;
    let merc = std::f64::consts::PI * (1.0 - 2.0 * (f64::from(y) + 0.5) / n);
    let lat = merc.sinh().atan().to_degrees();
    [lon, lat]
}

/// Latency percentile (`q` in 0..=1) from bucket counts, reported as the
/// bucket's upper bound; `None` with no samples.
fn percentile(buckets: &[u64; BUCKETS], q: f64) -> Option<f64> {
    let total: u64 = buckets.iter().sum();
    if total == 0 {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let rank = ((total as f64) * q).ceil().max(1.0) as u64;
    let mut seen = 0;
    for (i, count) in buckets.iter().enumerate() {
        seen += count;
        if seen >= rank {
            return Some(LATENCY_BOUNDS_MS.get(i).copied().unwrap_or(f64::INFINITY));
        }
    }
    None
}

fn ratio(part: u64, whole: u64) -> Option<f64> {
    #[allow(clippy::cast_precision_loss)]
    (whole > 0).then(|| part as f64 / whole as f64)
}

/// Latency percentiles in milliseconds; `None` when unknown. A value above
/// the last bound is reported as `null` in JSON (infinite).
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct Percentiles {
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p99_ms: Option<f64>,
}

impl Percentiles {
    fn from(buckets: &[u64; BUCKETS]) -> Self {
        let finite = |v: Option<f64>| v.filter(|v| v.is_finite());
        Self {
            p50_ms: finite(percentile(buckets, 0.50)),
            p95_ms: finite(percentile(buckets, 0.95)),
            p99_ms: finite(percentile(buckets, 0.99)),
        }
    }
}

/// Snapshot served at `GET /__admin/stats`.
#[derive(Debug, Clone, Serialize)]
pub struct DashboardSnapshot {
    pub enabled: bool,
    pub collecting_for_secs: u64,
    pub heatmap_zoom: u8,
    pub totals: Totals,
    pub latency: Percentiles,
    /// Bucket upper bounds in ms; the last bucket (`null`) is unbounded.
    pub latency_buckets: Vec<LatencyBucket>,
    /// Requests per minute for the last hour, oldest first, gaps filled.
    pub per_minute: Vec<MinutePoint>,
    pub sources: Vec<SourceSnapshot>,
    pub zooms: Vec<ZoomCount>,
    pub heatmap: Vec<HeatPoint>,
    /// Requests whose heatmap cell was not stored (cell limit reached).
    pub heatmap_dropped: u64,
    pub top_tiles: Vec<TopTile>,
    pub recent: Vec<RecentRequest>,
    pub cache: Option<CacheSnapshot>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct Totals {
    pub requests: u64,
    pub last_hour: u64,
    pub last_24h: u64,
    pub hits: u64,
    pub misses: u64,
    pub not_found: u64,
    pub errors: u64,
    pub bytes: u64,
    /// `hits / (hits + misses)`; `None` before any cacheable request.
    pub cache_hit_rate: Option<f64>,
    pub error_rate: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct LatencyBucket {
    pub le_ms: Option<f64>,
    pub count: u64,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct MinutePoint {
    pub minute_unix: u64,
    pub requests: u64,
    pub errors: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SourceSnapshot {
    pub id: String,
    pub requests: u64,
    pub hits: u64,
    pub misses: u64,
    pub not_found: u64,
    pub errors: u64,
    pub cache_hit_rate: Option<f64>,
    pub avg_bytes: Option<f64>,
    pub latency: Percentiles,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct ZoomCount {
    pub z: u8,
    pub requests: u64,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct HeatPoint {
    pub lon: f64,
    pub lat: f64,
    pub requests: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TopTile {
    pub source: String,
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub requests: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RecentRequest {
    pub at_unix_ms: u64,
    pub source: String,
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub outcome: &'static str,
    pub duration_ms: f64,
    pub bytes: usize,
}

/// Global tile cache state.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct CacheSnapshot {
    pub entries: u64,
    pub size_bytes: u64,
    pub capacity_bytes: u64,
}

/// Build the dashboard snapshot. `cache` is the global tile cache state, if
/// a cache is configured.
#[must_use]
pub fn snapshot(cache: Option<CacheSnapshot>) -> DashboardSnapshot {
    let inner = store();
    build_snapshot(&inner, is_enabled(), unix_ms() / 60_000, cache)
}

fn build_snapshot(
    inner: &Inner,
    enabled: bool,
    now_minute: u64,
    cache: Option<CacheSnapshot>,
) -> DashboardSnapshot {
    let mut all = [0u64; BUCKETS];
    let mut totals = Totals {
        requests: 0,
        last_hour: 0,
        last_24h: 0,
        hits: 0,
        misses: 0,
        not_found: 0,
        errors: 0,
        bytes: 0,
        cache_hit_rate: None,
        error_rate: None,
    };
    let mut sources: Vec<SourceSnapshot> = inner
        .sources
        .iter()
        .map(|(id, s)| {
            totals.requests += s.requests;
            totals.hits += s.hits;
            totals.misses += s.misses;
            totals.not_found += s.not_found;
            totals.errors += s.errors;
            totals.bytes += s.bytes;
            for (sum, n) in all.iter_mut().zip(s.latency) {
                *sum += n;
            }
            let served = s.hits + s.misses;
            SourceSnapshot {
                id: id.clone(),
                requests: s.requests,
                hits: s.hits,
                misses: s.misses,
                not_found: s.not_found,
                errors: s.errors,
                cache_hit_rate: ratio(s.hits, served),
                #[allow(clippy::cast_precision_loss)]
                avg_bytes: (served > 0).then(|| s.bytes as f64 / served as f64),
                latency: Percentiles::from(&s.latency),
            }
        })
        .collect();
    sources.sort_by(|a, b| b.requests.cmp(&a.requests).then_with(|| a.id.cmp(&b.id)));
    totals.cache_hit_rate = ratio(totals.hits, totals.hits + totals.misses);
    totals.error_rate = ratio(totals.errors, totals.requests);

    let by_minute: HashMap<u64, (u64, u64)> =
        inner.minutes.iter().map(|&(m, r, e)| (m, (r, e))).collect();
    for &(m, r, _) in &inner.minutes {
        if now_minute.saturating_sub(m) < 60 {
            totals.last_hour += r;
        }
        if now_minute.saturating_sub(m) < MINUTES_KEPT as u64 {
            totals.last_24h += r;
        }
    }
    let per_minute = (0..60)
        .rev()
        .map(|back| {
            let minute = now_minute.saturating_sub(back);
            let (requests, errors) = by_minute.get(&minute).copied().unwrap_or((0, 0));
            MinutePoint {
                minute_unix: minute,
                requests,
                errors,
            }
        })
        .collect();

    let zooms = inner
        .zooms
        .iter()
        .enumerate()
        .filter(|(_, n)| **n > 0)
        .map(|(z, n)| ZoomCount {
            z: u8::try_from(z).unwrap_or(u8::MAX),
            requests: *n,
        })
        .collect();

    let heatmap = inner
        .heat
        .iter()
        .map(|(&(x, y), &requests)| {
            let [lon, lat] = tile_center(inner.settings.heatmap_zoom, x, y);
            HeatPoint { lon, lat, requests }
        })
        .collect();

    let mut top: Vec<(&TileKey, &u64)> = inner.tiles.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    let top_tiles = top
        .into_iter()
        .take(TOP_TILES)
        .map(|((source, z, x, y), n)| TopTile {
            source: source.clone(),
            z: *z,
            x: *x,
            y: *y,
            requests: *n,
        })
        .collect();

    let recent = inner
        .recent
        .iter()
        .rev()
        .map(|r| RecentRequest {
            at_unix_ms: r.at_unix_ms,
            source: r.source.clone(),
            z: r.z,
            x: r.x,
            y: r.y,
            outcome: outcome_label(r.outcome),
            duration_ms: r.duration_ms,
            bytes: r.bytes,
        })
        .collect();

    let latency_buckets = all
        .iter()
        .enumerate()
        .map(|(i, &count)| LatencyBucket {
            le_ms: LATENCY_BOUNDS_MS.get(i).copied(),
            count,
        })
        .collect();

    DashboardSnapshot {
        enabled,
        collecting_for_secs: inner.started.elapsed().as_secs(),
        heatmap_zoom: inner.settings.heatmap_zoom,
        totals,
        latency: Percentiles::from(&all),
        latency_buckets,
        per_minute,
        sources,
        zooms,
        heatmap,
        heatmap_dropped: inner.heat_dropped,
        top_tiles,
        recent,
        cache,
    }
}

const fn outcome_label(outcome: TileOutcome) -> &'static str {
    match outcome {
        TileOutcome::Hit => "hit",
        TileOutcome::Miss => "miss",
        TileOutcome::NotFound => "not_found",
        TileOutcome::Error => "error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(
        source: &str,
        z: u8,
        x: u32,
        y: u32,
        ms: u64,
        outcome: TileOutcome,
    ) -> TileSample<'_> {
        TileSample {
            source,
            z,
            x,
            y,
            bytes: 1000,
            duration: Duration::from_millis(ms),
            outcome,
        }
    }

    fn settings(recent: usize) -> DashboardSettings {
        DashboardSettings {
            enabled: true,
            recent_requests: recent,
            heatmap_zoom: 4,
        }
    }

    #[test]
    fn buckets_and_percentiles() {
        assert_eq!(bucket_index(0.05), 0);
        assert_eq!(bucket_index(1.0), 3);
        assert_eq!(bucket_index(7.0), 6);
        assert_eq!(bucket_index(99_999.0), BUCKETS - 1);

        let mut b = [0u64; BUCKETS];
        assert_eq!(percentile(&b, 0.5), None);
        b[3] = 90; // <= 1 ms
        b[9] = 9; // <= 100 ms
        b[BUCKETS - 1] = 1; // > 5 s
        assert_eq!(percentile(&b, 0.50), Some(1.0));
        assert_eq!(percentile(&b, 0.95), Some(100.0));
        assert_eq!(percentile(&b, 0.995), Some(f64::INFINITY));
        assert_eq!(Percentiles::from(&b).p99_ms, Some(100.0));
    }

    #[test]
    fn heat_cells_scale_between_zooms() {
        assert_eq!(heat_cell(14, 8192, 5461, 10), (512, 341));
        // z2 tile 1/3 spans z4 cells 4-7 / 12-15; its center cell is 6/14.
        assert_eq!(heat_cell(2, 1, 3, 4), (6, 14));
        assert_eq!(heat_cell(4, 3, 3, 4), (3, 3));
        // z0 lands mid-world, not in the north-west corner.
        let [lon, lat] = tile_center(4, heat_cell(0, 0, 0, 4).0, heat_cell(0, 0, 0, 4).1);
        assert!(lon.abs() < 12.0 && lat.abs() < 25.0, "{lon},{lat}");
    }

    #[test]
    fn tile_centers_are_geographic() {
        let [lon, lat] = tile_center(0, 0, 0);
        assert!(lon.abs() < 1e-9 && lat.abs() < 1e-9);
        let [lon, lat] = tile_center(1, 1, 0);
        assert!((lon - 90.0).abs() < 1e-9);
        assert!(lat > 60.0 && lat < 70.0);
    }

    #[test]
    fn snapshot_aggregates_sources_zooms_and_rates() {
        let mut inner = Inner::new(settings(2));
        let t = 1_000 * 60_000; // minute 1000
        inner.record(&sample("a", 14, 8192, 5461, 1, TileOutcome::Hit), t);
        inner.record(&sample("a", 14, 8192, 5461, 1, TileOutcome::Hit), t + 1);
        inner.record(&sample("a", 12, 2048, 1365, 50, TileOutcome::Miss), t + 2);
        inner.record(&sample("b", 3, 1, 2, 3, TileOutcome::Error), t + 60_000);

        let snap = build_snapshot(&inner, true, 1001, None);
        assert_eq!(snap.totals.requests, 4);
        assert_eq!(snap.totals.hits, 2);
        assert_eq!(snap.totals.errors, 1);
        assert_eq!(snap.totals.cache_hit_rate, Some(2.0 / 3.0));
        assert_eq!(snap.totals.error_rate, Some(0.25));
        assert_eq!(snap.totals.last_hour, 4);

        assert_eq!(snap.sources[0].id, "a");
        assert_eq!(snap.sources[0].requests, 3);
        assert_eq!(snap.sources[0].latency.p50_ms, Some(1.0));
        assert_eq!(snap.sources[0].avg_bytes, Some(1000.0));
        assert_eq!(snap.sources[1].cache_hit_rate, None);

        assert_eq!(
            snap.zooms,
            vec![
                ZoomCount { z: 3, requests: 1 },
                ZoomCount { z: 12, requests: 1 },
                ZoomCount { z: 14, requests: 2 }
            ]
        );

        assert_eq!(snap.per_minute.len(), 60);
        assert_eq!(
            snap.per_minute[58],
            MinutePoint {
                minute_unix: 1000,
                requests: 3,
                errors: 0
            }
        );
        assert_eq!(
            snap.per_minute[59],
            MinutePoint {
                minute_unix: 1001,
                requests: 1,
                errors: 1
            }
        );

        assert_eq!(snap.top_tiles[0].requests, 2);
        assert_eq!((snap.top_tiles[0].z, snap.top_tiles[0].x), (14, 8192));

        // Recent list is capped at 2, newest first.
        assert_eq!(snap.recent.len(), 2);
        assert_eq!(snap.recent[0].source, "b");
        assert_eq!(snap.recent[0].outcome, "error");

        // z14 and z12 near the same place share one z4 cell; z3 maps to another.
        assert_eq!(snap.heatmap.len(), 2);
        assert_eq!(snap.heatmap.iter().map(|h| h.requests).sum::<u64>(), 4);
        assert_eq!(snap.latency_buckets.len(), BUCKETS);
        assert_eq!(snap.latency_buckets.last().map(|b| b.le_ms), Some(None));
    }

    #[test]
    fn minutes_outside_the_window_are_not_counted() {
        let mut inner = Inner::new(settings(0));
        inner.record(&sample("a", 1, 0, 0, 1, TileOutcome::Miss), 10 * 60_000);
        let snap = build_snapshot(&inner, true, 10 + 120, None);
        assert_eq!(snap.totals.last_hour, 0);
        assert_eq!(snap.totals.last_24h, 1);
        assert!(snap.recent.is_empty());
    }

    #[test]
    fn tracked_maps_stay_bounded() {
        let mut inner = Inner::new(settings(1));
        for i in 0..(MAX_TRACKED_TILES as u32 + 10) {
            inner.record(&sample("a", 18, i, i, 1, TileOutcome::Miss), 0);
        }
        assert_eq!(inner.tiles.len(), MAX_TRACKED_TILES);
        assert!(inner.heat.len() <= MAX_HEAT_CELLS);
        assert_eq!(inner.recent.len(), 1);
    }

    #[test]
    fn settings_reset_only_on_change_and_gate_recording() {
        let on = settings(5);
        let mut inner = Inner::new(on);
        record_if_enabled(&mut inner, &sample("a", 1, 0, 0, 1, TileOutcome::Miss), 0);
        assert_eq!(inner.sources["a"].requests, 1);

        apply_settings(&mut inner, on); // same settings: data kept
        assert_eq!(inner.sources["a"].requests, 1);

        apply_settings(
            &mut inner,
            DashboardSettings {
                enabled: false,
                ..on
            },
        );
        assert!(
            inner.sources.is_empty(),
            "a changed setting clears the data"
        );
        record_if_enabled(&mut inner, &sample("a", 1, 0, 0, 1, TileOutcome::Miss), 0);
        assert!(inner.sources.is_empty(), "off: nothing recorded");
    }
}
