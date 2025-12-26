/**
 * Map Style Composable
 *
 * Fetches individual map style using TanStack Query.
 * Wraps the query options from utils/api/styles.
 */

import type { StyleSpecification } from 'maplibre-gl';
import { useQuery } from '@tanstack/vue-query';
import { fetchRasterStyle, fetchVectorStyle } from '~/utils/api/styles';
import { MAP_STYLES_QUERY_KEYS } from '~/utils/query-keys';

export function useMapStyle(styleId: MaybeRef<string>, isRaster: MaybeRef<boolean> = false) {
  // Use computed to make query options reactive
  const queryOptions = computed(() => {
    const id = toValue(styleId);
    const raster = toValue(isRaster);

    return {
      queryKey: MAP_STYLES_QUERY_KEYS.style(id, raster),
      queryFn: (): Promise<StyleSpecification> =>
        raster ? fetchRasterStyle(id) : fetchVectorStyle(id),
      staleTime: 60 * 1000,
    };
  });

  const { data: style, isLoading, error } = useQuery(queryOptions);

  return {
    style: style as Ref<StyleSpecification | undefined>,
    isLoading,
    error,
  };
}
