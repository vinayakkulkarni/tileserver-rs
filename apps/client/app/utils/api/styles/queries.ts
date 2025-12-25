/**
 * Map Styles API Queries
 *
 * Query options and fetch functions for map styles.
 * Used by composables and TanStack DB collections.
 */

import type { StyleSpecification } from 'maplibre-gl';
import type { Style, TileJSON } from '~/types';
import { MAP_STYLES_QUERY_KEYS } from '~/utils/query-keys';

// ============================================================================
// DEFAULT STYLE
// ============================================================================

export const defaultStyle: StyleSpecification = {
  version: 8,
  metadata: {},
  sources: {},
  glyphs: 'https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf',
  layers: [
    {
      id: 'background',
      type: 'background',
      paint: {
        'background-color': '#f0f0f0',
      },
    },
  ],
};

// ============================================================================
// FETCH FUNCTIONS
// ============================================================================

export async function fetchStyles(): Promise<Style[]> {
  const result = await $fetch<Style[]>('/styles.json');
  return result ?? [];
}

export async function fetchVectorStyle(id: string): Promise<StyleSpecification> {
  const styleSpec = await $fetch<StyleSpecification>(`/styles/${id}/style.json`);
  return styleSpec ?? defaultStyle;
}

export async function fetchRasterStyle(id: string): Promise<StyleSpecification> {
  const tileJSON = await $fetch<TileJSON>(`/styles/${id}.json`);

  if (!tileJSON) {
    return defaultStyle;
  }

  return {
    version: 8,
    sources: {
      'raster-tiles': {
        type: 'raster',
        tiles: tileJSON.tiles,
        tileSize: 256,
        attribution: tileJSON.attribution,
      },
    },
    layers: [
      {
        id: 'raster-layer',
        type: 'raster',
        source: 'raster-tiles',
        minzoom: tileJSON.minzoom,
        maxzoom: tileJSON.maxzoom,
      },
    ],
  };
}

// ============================================================================
// QUERY OPTIONS
// ============================================================================

export function stylesQueryOptions() {
  return {
    queryKey: MAP_STYLES_QUERY_KEYS.all,
    queryFn: fetchStyles,
    staleTime: 30 * 1000,
  };
}

export function styleQueryOptions(id: string, isRaster: boolean = false) {
  return {
    queryKey: MAP_STYLES_QUERY_KEYS.style(id),
    queryFn: (): Promise<StyleSpecification> =>
      isRaster ? fetchRasterStyle(id) : fetchVectorStyle(id),
    staleTime: 60 * 1000,
  };
}
