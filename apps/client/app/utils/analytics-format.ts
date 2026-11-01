/** Formatting and bar-scaling helpers for the admin analytics page. */
import type {
  StatsBar,
  StatsLatencyBucket,
  StatsMinutePoint,
  StatsZoomCount,
} from '~/types';

/** Shown where a value is unknown (no samples yet). */
export const NONE = 'n/a';

const NUMBER = new Intl.NumberFormat('en-US');

export function formatCount(n: number): string {
  return NUMBER.format(n);
}

export function formatPercent(ratio: number | null): string {
  return ratio === null ? NONE : `${(ratio * 100).toFixed(1)}%`;
}

export function formatMs(ms: number | null): string {
  if (ms === null) return NONE;
  if (ms < 1) return `${ms.toFixed(2)} ms`;
  if (ms < 10) return `${ms.toFixed(1)} ms`;
  if (ms < 1000) return `${Math.round(ms)} ms`;
  return `${(ms / 1000).toFixed(1)} s`;
}

export function formatBytes(bytes: number | null): string {
  if (bytes === null) return NONE;
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return unit === 0 ? `${bytes} B` : `${value.toFixed(1)} ${units[unit]}`;
}

export function formatClock(unixMs: number): string {
  return new Date(unixMs).toLocaleTimeString('en-GB', { hour12: false });
}

function toBars<T>(
  items: readonly T[],
  value: (item: T) => number,
  key: (item: T) => string,
  label: (item: T) => string,
  title: (item: T) => string,
): StatsBar[] {
  const max = Math.max(1, ...items.map(value));
  return items.map((item) => ({
    key: key(item),
    label: label(item),
    value: value(item),
    percent: (value(item) / max) * 100,
    title: title(item),
    style: {
      height: `${value(item) > 0 ? Math.max((value(item) / max) * 100, 2) : 0}%`,
    },
  }));
}

export function minuteBars(points: readonly StatsMinutePoint[]): StatsBar[] {
  return toBars(
    points,
    (p) => p.requests,
    (p) => String(p.minute_unix),
    (p) => formatClock(p.minute_unix * 60_000).slice(0, 5),
    (p) =>
      `${formatClock(p.minute_unix * 60_000).slice(0, 5)} · ${formatCount(p.requests)} requests, ${formatCount(p.errors)} errors`,
  );
}

export function zoomBars(zooms: readonly StatsZoomCount[]): StatsBar[] {
  return toBars(
    zooms,
    (z) => z.requests,
    (z) => String(z.z),
    (z) => `z${z.z}`,
    (z) => `z${z.z} · ${formatCount(z.requests)} requests`,
  );
}

/** Latency bucket bound as written in the config: `0.25 ms`, `1 ms`, `2.5 s`. */
function boundLabel(ms: number | null): string {
  if (ms === null) return '> 5 s';
  return ms >= 1000 ? `≤ ${ms / 1000} s` : `≤ ${ms} ms`;
}

export function latencyBars(
  buckets: readonly StatsLatencyBucket[],
): StatsBar[] {
  return toBars(
    buckets,
    (b) => b.count,
    (b) => String(b.le_ms ?? 'inf'),
    (b) => boundLabel(b.le_ms),
    (b) => `${boundLabel(b.le_ms)} · ${formatCount(b.count)}`,
  );
}
