<script setup lang="ts">
  import { Search } from '@lucide/vue';
  import HomeFilterChip from './FilterChip.vue';
  import type { FilterCategory } from '~/types/home-filters';

  defineProps<{
    searchQuery: string;
    allChips: Array<{
      category: FilterCategory;
      label: string;
      count: number;
    }>;
    activeFilter: FilterCategory;
  }>();

  const emit = defineEmits<{
    'update:searchQuery': [value: string];
    'select-filter': [category: FilterCategory];
  }>();

  function handleSearch(value: string) {
    emit('update:searchQuery', value);
  }

  function handleFilterClick(category: FilterCategory) {
    emit('select-filter', category);
  }
</script>

<template>
  <div
    class="toolbar sticky z-19 bg-background/92 backdrop-blur-md border-b border-border flex flex-col gap-2.5"
    style="top: 56px; padding: 10px clamp(12px, 4vw, 24px)"
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
        class="h-11 w-full border border-border bg-surface px-4 pl-11 text-[15px] text-foreground transition-colors duration-[var(--d-fast,120ms)] placeholder:text-muted-foreground focus:border-primary focus:bg-background focus:outline-none focus:ring-2 focus:ring-primary/20"
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
      class="filters flex gap-1.5 overflow-x-auto pb-0.5"
      style="scrollbar-width: thin"
      role="group"
      aria-label="Filter by type"
    >
      <HomeFilterChip
        v-for="chip in allChips"
        :key="chip.category"
        :label="chip.label"
        :count="chip.count"
        :category="chip.category"
        :active="activeFilter === chip.category"
        @select-filter="handleFilterClick"
      />
    </div>
  </div>
</template>
