/**
 * Home Filters Composable
 *
 * Generates filter chips from loaded source/style metadata.
 * Chips are NOT hardcoded — see CLAUDE.md Rule #20.J.
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

  const activeFilter = ref<FilterCategory>('all');

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
   * Derive source type from tile URL format.
   * Vector tiles: .pbf or .mvt extension
   * Raster tiles: .png, .jpg, .webp, .jpeg extensions
   */
  function inferSourceFormat(data: Data): string {
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

  const dataChips = computed<FilterChip[]>(() => {
    const list = dataSources.value ?? [];
    if (list.length === 0) return [];

    const counts = new Map<string, number>();
    for (const s of list) {
      const format = inferSourceFormat(s as Data);
      counts.set(format, (counts.get(format) ?? 0) + 1);
    }

    const chips: FilterChip[] = [
      { category: 'all', label: 'All', count: list.length },
    ];
    for (const [format, count] of counts) {
      const cat = format as FilterCategory;
      chips.push({ category: cat, label: format, count });
    }
    return chips;
  });

  const allChips = computed<FilterChip[]>(() => {
    const merged = new Map<FilterCategory, FilterChip>();
    for (const chip of [...styleChips.value, ...dataChips.value]) {
      const existing = merged.get(chip.category);
      if (existing) {
        existing.count += chip.count;
      } else {
        merged.set(chip.category, { ...chip });
      }
    }

    const allCount =
      (styles.value?.length ?? 0) + (dataSources.value?.length ?? 0);
    const chips: FilterChip[] = [
      { category: 'all', label: 'All', count: allCount },
    ];
    for (const chip of merged.values()) {
      if (chip.category !== 'all') chips.push(chip);
    }
    return chips;
  });

  const filteredStyles = computed(() => {
    const list = styles.value ?? [];
    if (activeFilter.value === 'all') return list;
    if (activeFilter.value === 'raster') {
      return list.filter(
        (s: Style) =>
          s.id.includes('raster') || s.name.toLowerCase().includes('raster'),
      );
    }
    if (activeFilter.value === 'vector') {
      return list.filter(
        (s: Style) =>
          !s.id.includes('raster') && !s.name.toLowerCase().includes('raster'),
      );
    }
    return list;
  });

  const filteredDataSources = computed(() => {
    const list = dataSources.value ?? [];
    if (activeFilter.value === 'all') return list;
    if (activeFilter.value === 'raster' || activeFilter.value === 'vector') {
      return list;
    }
    return list.filter((s: Data) => s.type === activeFilter.value);
  });

  function setFilter(filter: FilterCategory) {
    activeFilter.value = filter;
  }

  return {
    activeFilter,
    allChips,
    filteredStyles,
    filteredDataSources,
    setFilter,
  };
}
