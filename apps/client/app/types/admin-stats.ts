/**
 * Admin analytics dashboard types (`GET /__admin/stats`).
 *
 * Mirrors `DashboardSnapshot` in `crates/tileserver-rs/src/metrics/dashboard.rs`.
 */

export interface StatsPercentiles {
  p50_ms: number | null;
  p95_ms: number | null;
  p99_ms: number | null;
}

export interface StatsTotals {
  requests: number;
  last_hour: number;
  last_24h: number;
  hits: number;
  misses: number;
  not_found: number;
  errors: number;
  bytes: number;
  cache_hit_rate: number | null;
  error_rate: number | null;
}

export interface StatsLatencyBucket {
  /** Upper bound in ms; `null` for the open last bucket. */
  le_ms: number | null;
  count: number;
}

export interface StatsMinutePoint {
  minute_unix: number;
  requests: number;
  errors: number;
}

export interface StatsSource {
  id: string;
  requests: number;
  hits: number;
  misses: number;
  not_found: number;
  errors: number;
  cache_hit_rate: number | null;
  avg_bytes: number | null;
  latency: StatsPercentiles;
}

export interface StatsZoomCount {
  z: number;
  requests: number;
}

export interface StatsHeatPoint {
  lon: number;
  lat: number;
  requests: number;
}

export interface StatsTopTile {
  source: string;
  z: number;
  x: number;
  y: number;
  requests: number;
}

export type StatsOutcome = 'hit' | 'miss' | 'not_found' | 'error';

export interface StatsRecentRequest {
  at_unix_ms: number;
  source: string;
  z: number;
  x: number;
  y: number;
  outcome: StatsOutcome;
  duration_ms: number;
  bytes: number;
}

export interface StatsCache {
  entries: number;
  size_bytes: number;
  capacity_bytes: number;
}

export interface AdminStatsPayload {
  enabled: boolean;
  collecting_for_secs: number;
  heatmap_zoom: number;
  totals: StatsTotals;
  latency: StatsPercentiles;
  latency_buckets: StatsLatencyBucket[];
  per_minute: StatsMinutePoint[];
  sources: StatsSource[];
  zooms: StatsZoomCount[];
  heatmap: StatsHeatPoint[];
  heatmap_dropped: number;
  top_tiles: StatsTopTile[];
  recent: StatsRecentRequest[];
  cache: StatsCache | null;
}

/** One labelled figure in the overview strip. */
export interface StatsFigure {
  label: string;
  value: string;
  hint: string;
}

/** One bar in a CSS bar chart, height as a 0-100 percentage. */
export interface StatsBar {
  key: string;
  label: string;
  value: number;
  percent: number;
  title: string;
  /** Precomputed bar height (min 2% for non-zero values). */
  style: { height: string };
}
