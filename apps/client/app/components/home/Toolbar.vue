<script setup lang="ts">
  import { Search } from '@lucide/vue';
  import HomeFilterChip from './FilterChip.vue';
  import type { FilterCategory, FilterChip } from '~/types/home-filters';

  defineProps<{
    searchQuery: string;
    styleChips: FilterChip[];
    sourceChips: FilterChip[];
    activeStyleFilter: FilterCategory;
    activeSourceFilter: FilterCategory;
  }>();

  const emit = defineEmits<{
    'update:search-query': [value: string];
    'select-style-filter': [category: FilterCategory];
    'select-source-filter': [category: FilterCategory];
  }>();

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
  <div class="toolbar shrink-0 border-b border-border bg-background">
    <div
      class="mx-auto flex max-w-screen-2xl flex-col gap-2.5 px-[clamp(12px,4vw,24px)] py-2.5"
    >
      <div class="search relative">
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
      <div
        class="filters flex flex-wrap items-center gap-x-3 gap-y-1.5 overflow-x-auto pb-0.5"
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
</template>
