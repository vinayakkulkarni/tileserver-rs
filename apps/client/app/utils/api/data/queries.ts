/**
 * Data Sources API Queries
 *
 * Query options and fetch functions for tile data sources.
 * Used by composables and TanStack DB collections.
 */

import type { Data } from '~/types';

// ============================================================================
// FETCH FUNCTIONS
// ============================================================================

export async function fetchDataSources(): Promise<Data[]> {
  const result = await $fetch<Data[]>(apiUrl('/data.json'));
  return result ?? [];
}

export async function fetchDataSource(id: string): Promise<Data | null> {
  const result = await $fetch<Data>(apiUrl(`/data/${id}.json`));
  return result ?? null;
}
