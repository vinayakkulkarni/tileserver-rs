import { describe, expect, it } from 'vitest';
import {
  NONE,
  formatBytes,
  formatMs,
  formatPercent,
  latencyBars,
  minuteBars,
  zoomBars,
} from './analytics-format';
import { heatmapGeoJSON } from './heatmap-geojson';

describe('formatters', () => {
  it('formats latency', () => {
    expect(formatMs(null)).toBe(NONE);
    expect(formatMs(0.25)).toBe('0.25 ms');
    expect(formatMs(2.5)).toBe('2.5 ms');
    expect(formatMs(42.4)).toBe('42 ms');
    expect(formatMs(2500)).toBe('2.5 s');
  });

  it('formats ratios and sizes', () => {
    expect(formatPercent(null)).toBe(NONE);
    expect(formatPercent(0.9412)).toBe('94.1%');
    expect(formatBytes(null)).toBe(NONE);
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(41_370)).toBe('40.4 KB');
  });
});

describe('bar builders', () => {
  it('scales bars to the largest value', () => {
    const bars = zoomBars([
      { z: 4, requests: 5 },
      { z: 14, requests: 20 },
    ]);
    expect(bars.map((b) => [b.label, b.percent])).toEqual([
      ['z4', 25],
      ['z14', 100],
    ]);
  });

  it('keeps empty series at zero height', () => {
    const bars = minuteBars([{ minute_unix: 1, requests: 0, errors: 0 }]);
    expect(bars[0]?.percent).toBe(0);
  });

  it('labels the open latency bucket', () => {
    const bars = latencyBars([
      { le_ms: 1, count: 3 },
      { le_ms: null, count: 1 },
    ]);
    expect(bars.map((b) => b.label)).toEqual(['≤ 1 ms', '> 5 s']);
  });
});

describe('heatmapGeoJSON', () => {
  it('turns cells into weighted points', () => {
    const fc = heatmapGeoJSON([{ lon: 11.25, lat: 43.77, requests: 7 }]);
    expect(fc.features).toEqual([
      {
        type: 'Feature',
        properties: { requests: 7 },
        geometry: { type: 'Point', coordinates: [11.25, 43.77] },
      },
    ]);
  });
});
