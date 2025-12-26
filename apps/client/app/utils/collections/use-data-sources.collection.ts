/**
 * Data Sources Collection Composable
 *
 * Provides TanStack DB collection for tile data sources.
 * Uses the function-based composable pattern for proper Vue reactivity integration.
 *
 * @example
 * ```vue
 * <script setup>
 * const { dataSourcesCollection } = useDataSourcesCollection()
 * const { data: sources, isLoading } = useLiveQuery(dataSourcesCollection)
 * </script>
 * ```
 */

import { createCollection } from '@tanstack/vue-db';
import { queryCollectionOptions } from '@tanstack/query-db-collection';
import { useQueryClient } from '@tanstack/vue-query';
import type { Data } from '~/types';
import { DATA_SOURCES_QUERY_KEYS } from '~/utils/query-keys';
import { DATA_SOURCES_COLLECTION_KEYS } from '~/utils/collection-keys';
import { fetchDataSources } from '~/utils/api/data';

// ============================================================================
// COLLECTION FACTORY
// ============================================================================

function createDataSourcesCollection(
  queryClient: ReturnType<typeof useQueryClient>,
) {
  return createCollection(
    queryCollectionOptions<Data>({
      id: DATA_SOURCES_COLLECTION_KEYS.id,
      queryKey: DATA_SOURCES_QUERY_KEYS.all,
      queryClient,
      getKey: (item) => item.id,
      queryFn: fetchDataSources,
      staleTime: 30 * 1000,
    }),
  );
}

// ============================================================================
// COLLECTION CACHE (Singleton)
// ============================================================================

let cachedDataSourcesCollection: ReturnType<
  typeof createDataSourcesCollection
> | null = null;

/**
 * Get or create the data sources collection.
 * Caches the collection to ensure single instance.
 */
function getOrCreateCollection(queryClient: ReturnType<typeof useQueryClient>) {
  if (cachedDataSourcesCollection) {
    return cachedDataSourcesCollection;
  }

  cachedDataSourcesCollection = createDataSourcesCollection(queryClient);
  return cachedDataSourcesCollection;
}

// ============================================================================
// COMPOSABLE
// ============================================================================

/**
 * Data Sources Collection Composable
 *
 * Creates and returns a TanStack DB collection for tile data sources.
 * Uses a singleton pattern to ensure all components share the same
 * collection instance (required for useLiveQuery to work correctly).
 *
 * @returns Object containing the data sources collection
 */
export function useDataSourcesCollection() {
  const queryClient = useQueryClient();

  // Use cached collection (singleton)
  const dataSourcesCollection = getOrCreateCollection(queryClient);

  return {
    dataSourcesCollection,
    queryKeys: DATA_SOURCES_QUERY_KEYS,
  };
}
