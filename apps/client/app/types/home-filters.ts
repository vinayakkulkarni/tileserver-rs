/**
 * Filter chip types derived from loaded source/style metadata.
 *
 * Chips are NOT hardcoded — they are computed from actual type counts
 * in use-home-filters.ts. See CLAUDE.md Rule #20.J.
 */
export interface FilterChip {
  category: FilterCategory;
  label: string;
  count: number;
}

export type FilterCategory =
  | 'all'
  | 'raster'
  | 'vector'
  | 'pmtiles'
  | 'mbtiles'
  | 'stac'
  | 'postgis'
  | 'mlt'
  | 'cog'
  | 'geoparquet';
