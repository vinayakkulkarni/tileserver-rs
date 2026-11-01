import type { StatsHeatPoint } from '~/types';

/** GeoJSON points weighted by request count. */
export function heatmapGeoJSON(points: readonly StatsHeatPoint[]) {
  return {
    type: 'FeatureCollection' as const,
    features: points.map((p) => ({
      type: 'Feature' as const,
      properties: { requests: p.requests },
      geometry: { type: 'Point' as const, coordinates: [p.lon, p.lat] },
    })),
  };
}
