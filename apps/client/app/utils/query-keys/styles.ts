/**
 * Styles Query Keys
 * Used by TanStack Query and TanStack DB collections
 */
export const MAP_STYLES_QUERY_KEYS = {
  all: ['map-styles'] as const,
  lists: () => [...MAP_STYLES_QUERY_KEYS.all, 'list'] as const,
  list: (params?: { visibility?: string }) =>
    [...MAP_STYLES_QUERY_KEYS.lists(), params ?? {}] as const,
  details: () => [...MAP_STYLES_QUERY_KEYS.all, 'detail'] as const,
  detail: (styleId: string) =>
    [...MAP_STYLES_QUERY_KEYS.details(), styleId] as const,
  style: (styleId: string, isRaster: boolean = false) =>
    [
      ...MAP_STYLES_QUERY_KEYS.all,
      'style',
      styleId,
      isRaster ? 'raster' : 'vector',
    ] as const,
} as const;
