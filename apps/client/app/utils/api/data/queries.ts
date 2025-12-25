/**
 * Data Sources API Queries
 *
 * Query options and fetch functions for tile data sources.
 * Used by composables and TanStack DB collections.
 */

import type { Data, TileJSON } from '~/types';
import { DATA_SOURCES_QUERY_KEYS } from '~/utils/query-keys';

// ============================================================================
// FETCH FUNCTIONS
// ============================================================================

export async function fetchDataSources(): Promise<Data[]> {
  const result = await $fetch<Data[]>('/data.json');
  return result ?? [];
}

export async function fetchDataSource(id: string): Promise<TileJSON | null> {
  const result = await $fetch<TileJSON>(`/data/${id}.json`);
  return result ?? null;
}

// ============================================================================
// QUERY OPTIONS
// ============================================================================

export function dataSourcesQueryOptions() {
  return {
    queryKey: DATA_SOURCES_QUERY_KEYS.all,
    queryFn: fetchDataSources,
    staleTime: 30 * 1000,
  };
}

export function dataSourceQueryOptions(id: string) {
  return {
    queryKey: DATA_SOURCES_QUERY_KEYS.detail(id),
    queryFn: (): Promise<TileJSON | null> => fetchDataSource(id),
    staleTime: 60 * 1000,
  };
}
