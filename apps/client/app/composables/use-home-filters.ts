/**
 * Home Filters Composable
 *
 * Generates filter chips from loaded source/style metadata.
 * Chips are NOT hardcoded — see CLAUDE.md Rule #20.J.
 *
 * Two independent chip namespaces per Rule #20.B:
 * - styleChips: derived from Style.type (raster / vector) — filters Map styles section
 * - sourceChips: derived from Data tile URL extension — filters Data sources section
 *
 * Counts NEVER merge across sections (1 vector style + 2 vector sources renders as
 * "Styles: Vector 1" + "Sources: Vector 2", not "Vector 3").
 */
import type { FilterCategory, FilterChip } from '~/types/home-filters';
import type { Data } from '~/types/data';
import type { Style } from '~/types/style';
import { useDataSourcesCollection } from '~/utils/collections/use-data-sources.collection';
import { useMapStylesCollection } from '~/utils/collections/use-map-styles.collection';
import { useLiveQuery } from '@tanstack/vue-db';

export function useHomeFilters() {
  const { dataSourcesCollection } = useDataSourcesCollection();
  const { mapStylesCollection } = useMapStylesCollection();

  const { data: dataSources } = useLiveQuery(dataSourcesCollection);
  const { data: styles } = useLiveQuery(mapStylesCollection);

  const activeStyleFilter = ref<FilterCategory>('all');
  const activeSourceFilter = ref<FilterCategory>('all');

  const styleChips = computed<FilterChip[]>(() => {
    const list = styles.value ?? [];
    if (list.length === 0) return [];

    const raster = list.filter(
      (s: Style) =>
        s.id.includes('raster') || s.name.toLowerCase().includes('raster'),
    ).length;
    const vector = list.length - raster;

    const chips: FilterChip[] = [
      { category: 'all', label: 'All', count: list.length },
    ];
    if (raster > 0)
      chips.push({ category: 'raster', label: 'Raster', count: raster });
    if (vector > 0)
      chips.push({ category: 'vector', label: 'Vector', count: vector });
    return chips;
  });

  /**
   * Derive source format from tile URL extension.
   * Vector tiles: .pbf or .mvt extension.
   * Raster tiles: .png, .jpg, .webp, .jpeg extensions.
   */
  function inferSourceFormat(data: Data): FilterCategory {
    const tile = data.tiles?.[0] ?? '';
    if (tile.endsWith('.pbf') || tile.endsWith('.mvt')) return 'vector';
    if (
      tile.endsWith('.png') ||
      tile.endsWith('.jpg') ||
      tile.endsWith('.jpeg') ||
      tile.endsWith('.webp')
    )
      return 'raster';
    return 'vector';
  }

  const sourceChips = computed<FilterChip[]>(() => {
    const list = dataSources.value ?? [];
    if (list.length === 0) return [];

    const counts = new Map<FilterCategory, number>();
    for (const s of list) {
      const fmt = inferSourceFormat(s as Data);
      counts.set(fmt, (counts.get(fmt) ?? 0) + 1);
    }

    const chips: FilterChip[] = [
      { category: 'all', label: 'All', count: list.length },
    ];
    for (const [cat, count] of counts) {
      chips.push({
        category: cat,
        label: cat[0]!.toUpperCase() + cat.slice(1),
        count,
      });
    }
    return chips;
  });

  const filteredStyles = computed(() => {
    const list = styles.value ?? [];
    if (activeStyleFilter.value === 'all') return list;
    if (activeStyleFilter.value === 'raster') {
      return list.filter(
        (s: Style) =>
          s.id.includes('raster') || s.name.toLowerCase().includes('raster'),
      );
    }
    if (activeStyleFilter.value === 'vector') {
      return list.filter(
        (s: Style) =>
          !s.id.includes('raster') && !s.name.toLowerCase().includes('raster'),
      );
    }
    return list;
  });

  const filteredDataSources = computed(() => {
    const list = dataSources.value ?? [];
    if (activeSourceFilter.value === 'all') return list;
    return list.filter(
      (s: Data) => inferSourceFormat(s as Data) === activeSourceFilter.value,
    );
  });

  function setStyleFilter(filter: FilterCategory) {
    activeStyleFilter.value = filter;
  }

  function setSourceFilter(filter: FilterCategory) {
    activeSourceFilter.value = filter;
  }

  // Number of filters narrowed off the default 'all' — badges the <lg
  // "Filters" toggle so a collapsed chip row still signals an active filter.
  const activeFilterCount = computed(() => {
    let n = 0;
    if (activeStyleFilter.value !== 'all') n += 1;
    if (activeSourceFilter.value !== 'all') n += 1;
    return n;
  });

  // Drives ONLY the <lg chip disclosure (lg+ shows chips inline); collapsed
  // by default so only the 44px search stays pinned on small viewports.
  const filtersOpen = ref(false);
  function toggleFilters() {
    filtersOpen.value = !filtersOpen.value;
  }

  return {
    activeStyleFilter,
    activeSourceFilter,
    styleChips,
    sourceChips,
    filteredStyles,
    filteredDataSources,
    setStyleFilter,
    setSourceFilter,
    activeFilterCount,
    filtersOpen,
    toggleFilters,
  };
}
