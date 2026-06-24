<script setup lang="ts">
  import { ChevronDown, Search, SlidersHorizontal } from '@lucide/vue';
  import HomeFilterChip from './FilterChip.vue';
  import type { FilterCategory, FilterChip } from '~/types/home-filters';

  defineProps<{
    searchQuery: string;
    styleChips: FilterChip[];
    sourceChips: FilterChip[];
    activeStyleFilter: FilterCategory;
    activeSourceFilter: FilterCategory;
    activeFilterCount: number;
    filtersOpen: boolean;
  }>();

  const emit = defineEmits<{
    'update:search-query': [value: string];
    'select-style-filter': [category: FilterCategory];
    'select-source-filter': [category: FilterCategory];
    'toggle-filters': [];
  }>();

  function handleToggleFilters() {
    emit('toggle-filters');
  }

  function handleSearch(value: string) {
    emit('update:search-query', value);
  }

  function handleStyleFilterClick(category: FilterCategory) {
    emit('select-style-filter', category);
  }

  function handleSourceFilterClick(category: FilterCategory) {
    emit('select-source-filter', category);
  }
</script>

<template>
  <div
    class="toolbar shrink-0 border-b border-border bg-background"
    :class="{ open: filtersOpen }"
  >
    <div
      class="mx-auto flex max-w-screen-2xl flex-col gap-2.5 px-[clamp(12px,4vw,24px)] py-2.5"
    >
      <div class="flex items-center gap-2">
        <div class="search relative flex-1">
          <label class="sr-only" for="search-input"
            >Search styles and data sources</label
          >
          <input
            id="search-input"
            :value="searchQuery"
            type="text"
            placeholder="Search styles and data sources..."
            autocomplete="off"
            class="h-11 w-full border border-border bg-card px-4 pl-11 text-[15px] text-foreground transition-colors duration-[var(--d-fast,120ms)] placeholder:text-muted-foreground focus:border-primary focus:bg-background focus:outline-none focus:ring-2 focus:ring-primary/20"
            :class="searchQuery ? 'text-foreground' : 'text-muted-foreground'"
            @input="handleSearch(($event.target as HTMLInputElement).value)"
          />
          <Search
            class="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground transition-colors duration-[var(--d-fast,120ms)]"
            :class="searchQuery ? 'text-primary' : 'text-muted-foreground'"
            aria-hidden="true"
          />
        </div>
        <button
          type="button"
          class="filters-toggle flex h-11 shrink-0 items-center gap-2 border border-border bg-card px-3.5 text-[13px] font-medium text-foreground transition-colors duration-[var(--d-fast,120ms)] hover:border-primary hover:text-primary lg:hidden"
          :class="
            activeFilterCount > 0 || filtersOpen
              ? 'border-primary text-primary'
              : ''
          "
          :aria-expanded="filtersOpen"
          aria-controls="toolbar-filters"
          @click="handleToggleFilters"
        >
          <SlidersHorizontal class="size-4" aria-hidden="true" />
          <span>Filters</span>
          <span
            v-if="activeFilterCount > 0"
            class="grid size-[18px] place-items-center bg-primary text-[10px] font-semibold text-primary-foreground tabular-nums"
            >{{ activeFilterCount }}</span
          >
          <ChevronDown
            class="size-4 transition-transform duration-[var(--d-base,180ms)] ease-[var(--ease,cubic-bezier(0.16,1,0.3,1))]"
            :class="{ 'rotate-180': filtersOpen }"
            aria-hidden="true"
          />
        </button>
      </div>
      <div id="toolbar-filters" class="toolbar-filters-wrap">
        <div class="toolbar-filters-inner">
          <div
            class="filters flex flex-wrap items-center gap-x-3 gap-y-1.5 overflow-x-auto pb-0.5 pt-2.5 lg:pt-0"
            style="scrollbar-width: thin"
          >
            <div
              v-if="styleChips.length > 0"
              class="flex items-center gap-1.5"
              role="group"
              aria-label="Filter map styles"
            >
              <span
                class="font-mono text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground"
                aria-hidden="true"
                >Styles</span
              >
              <HomeFilterChip
                v-for="chip in styleChips"
                :key="`style-${chip.category}`"
                :label="chip.label"
                :count="chip.count"
                :category="chip.category"
                :active="activeStyleFilter === chip.category"
                @select-filter="handleStyleFilterClick"
              />
            </div>
            <div
              v-if="sourceChips.length > 0"
              class="flex items-center gap-1.5"
              role="group"
              aria-label="Filter data sources"
            >
              <span
                class="font-mono text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground"
                aria-hidden="true"
                >Sources</span
              >
              <HomeFilterChip
                v-for="chip in sourceChips"
                :key="`source-${chip.category}`"
                :label="chip.label"
                :count="chip.count"
                :category="chip.category"
                :active="activeSourceFilter === chip.category"
                @select-filter="handleSourceFilterClick"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
