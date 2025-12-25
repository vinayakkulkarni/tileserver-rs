/**
 * Map Style Composable
 *
 * Fetches individual map style using TanStack Query.
 * Wraps the query options from utils/api/styles.
 */

import type { StyleSpecification } from 'maplibre-gl';
import { useQuery } from '@tanstack/vue-query';
import { styleQueryOptions } from '~/utils/api/styles';

export function useMapStyle(styleId: MaybeRef<string>, isRaster: MaybeRef<boolean> = false) {
  const id = toValue(styleId);
  const raster = toValue(isRaster);

  const { data: style, isLoading, error } = useQuery(styleQueryOptions(id, raster));

  return {
    style: style as Ref<StyleSpecification | undefined>,
    isLoading,
    error,
  };
}
