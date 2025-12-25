/**
 * Map Styles Collection Composable
 *
 * Provides TanStack DB collection for map styles.
 * Uses the function-based composable pattern for proper Vue reactivity integration.
 *
 * @example
 * ```vue
 * <script setup>
 * const { mapStylesCollection } = useMapStylesCollection()
 * const { data: styles, isLoading } = useLiveQuery(mapStylesCollection)
 * </script>
 * ```
 */

import { createCollection } from '@tanstack/vue-db';
import { queryCollectionOptions } from '@tanstack/query-db-collection';
import { useQueryClient } from '@tanstack/vue-query';
import type { Style } from '~/types';
import { MAP_STYLES_QUERY_KEYS } from '~/utils/query-keys';
import { MAP_STYLES_COLLECTION_KEYS } from '~/utils/collection-keys';
import { fetchStyles } from '~/utils/api/styles';

// ============================================================================
// COLLECTION FACTORY
// ============================================================================

function createMapStylesCollection(
  queryClient: ReturnType<typeof useQueryClient>,
) {
  return createCollection(
    queryCollectionOptions<Style>({
      id: MAP_STYLES_COLLECTION_KEYS.id,
      queryKey: MAP_STYLES_QUERY_KEYS.all,
      queryClient,
      getKey: (item) => item.id,
      queryFn: fetchStyles,
      staleTime: 30 * 1000,
    }),
  );
}

// ============================================================================
// COLLECTION CACHE (Singleton)
// ============================================================================

let cachedMapStylesCollection: ReturnType<typeof createMapStylesCollection> | null = null;

/**
 * Get or create the map styles collection.
 * Caches the collection to ensure single instance.
 */
function getOrCreateCollection(
  queryClient: ReturnType<typeof useQueryClient>,
) {
  if (cachedMapStylesCollection) {
    return cachedMapStylesCollection;
  }

  cachedMapStylesCollection = createMapStylesCollection(queryClient);
  return cachedMapStylesCollection;
}

// ============================================================================
// COMPOSABLE
// ============================================================================

/**
 * Map Styles Collection Composable
 *
 * Creates and returns a TanStack DB collection for map styles.
 * Uses a singleton pattern to ensure all components share the same
 * collection instance (required for useLiveQuery to work correctly).
 *
 * @returns Object containing the map styles collection
 */
export function useMapStylesCollection() {
  const queryClient = useQueryClient();

  // Use cached collection (singleton)
  const mapStylesCollection = getOrCreateCollection(queryClient);

  return {
    mapStylesCollection,
    queryKeys: MAP_STYLES_QUERY_KEYS,
  };
}
