<script setup lang="ts">
  import { Search } from '@lucide/vue';
  import type { FilterCategory } from '~/types/home-filters';
  import HomeFilterChip from './FilterChip.vue';

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

  const searchInput = ref<HTMLInputElement | null>(null);

  function handleSearch(value: string) {
    emit('update:searchQuery', value);
  }

  function handleFilterClick(category: FilterCategory) {
    emit('select-filter', category);
  }
</script>

<template>
  <div
    class="sticky top-[3.5rem] z-40 border-b border-border/50 bg-background/95 backdrop-blur-sm"
  >
    <div
      class="mx-auto flex max-w-7xl items-center gap-3 px-4 py-3 sm:px-6 lg:px-8"
    >
      <div class="relative flex-1">
        <Search
          class="pointer-events-none absolute top-1/2 left-4 size-4 -translate-y-1/2 text-muted-foreground transition-colors duration-[var(--d-fast,120ms)]"
          :class="searchQuery ? 'text-primary' : 'text-muted-foreground'"
        />
        <input
          ref="searchInput"
          :value="searchQuery"
          type="text"
          placeholder="Search styles and data sources..."
          class="h-11 w-full border border-border/50 bg-muted/30 pl-11 pr-4 text-sm transition-all duration-[var(--d-fast,120ms)] focus:border-primary focus:bg-background focus:outline-none focus:ring-2 focus:ring-primary/20"
          :aria-label="'Search styles and data sources'"
          @input="handleSearch(($event.target as HTMLInputElement).value)"
        />
      </div>
      <div
        class="flex gap-2 overflow-x-auto pb-0.5 scrollbar-thin"
        style="scrollbar-width: thin"
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
  </div>
</template>
