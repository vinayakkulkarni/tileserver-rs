/**
 * Tileserver Data Composable
 *
 * Provides access to tileserver data sources and styles
 * using TanStack DB collections with useLiveQuery.
 */

import { useLiveQuery } from '@tanstack/vue-db';
import {
  useDataSourcesCollection,
  useMapStylesCollection,
} from '~/utils/collections';

export function useTileserverData() {
  const { dataSourcesCollection } = useDataSourcesCollection();
  const { mapStylesCollection } = useMapStylesCollection();

  const { data: dataSources, isLoading: isLoadingData } = useLiveQuery(
    dataSourcesCollection,
  );
  const { data: styles, isLoading: isLoadingStyles } = useLiveQuery(
    mapStylesCollection,
  );

  const hasStyles = computed(() => styles.value && styles.value.length > 0);
  const hasData = computed(
    () => dataSources.value && dataSources.value.length > 0,
  );

  return {
    dataSources,
    styles,
    isLoadingData,
    isLoadingStyles,
    hasStyles,
    hasData,
  };
}
