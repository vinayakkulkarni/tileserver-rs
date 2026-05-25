<script setup lang="ts">
  import { Database } from '@lucide/vue';
  import type { Data } from '~/types/data';

  defineProps<{
    sources: Data[];
    isLoading: boolean;
    hasData: boolean;
    searchQuery: string;
    baseUrl: string;
    expandedXyz: Set<string>;
    copiedUrl: string | null;
  }>();

  const emit = defineEmits<{
    'toggle-xyz': [dataId: string];
    'copy-url': [url: string];
  }>();

  const SKELETON_COUNT = 5;
</script>

<template>
  <Separator class="bg-border/50" />
  <div class="p-4">
    <div v-if="isLoading" class="space-y-3">
      <div
        v-for="i in SKELETON_COUNT"
        :key="i"
        class="border border-border/50 bg-background/50 p-4"
      >
        <div class="flex items-start gap-4">
          <Skeleton class="size-12 shrink-0" />
          <div class="min-w-0 flex-1">
            <Skeleton class="h-4 w-2/3" />
            <Skeleton class="mt-2 h-3 w-1/3" />
            <Skeleton class="mt-3 h-3 w-4/5" />
          </div>
        </div>
      </div>
    </div>
    <div v-else-if="!hasData" class="py-12 text-center">
      <div
        class="mx-auto mb-4 flex size-16 items-center justify-center bg-muted"
      >
        <Database class="size-8 text-muted-foreground" />
      </div>
      <p class="font-medium">No data sources configured</p>
      <p class="mt-1 text-sm text-muted-foreground">
        Add PMTiles or MBTiles to config.toml
      </p>
    </div>
    <div
      v-else-if="sources.length === 0"
      class="py-12 text-center text-muted-foreground"
    >
      No data sources match "{{ searchQuery }}"
    </div>
    <div v-else class="space-y-3">
      <HomeDataCard
        v-for="(source, i) in sources"
        :key="source.id"
        :source="source"
        :index="i"
        :base-url="baseUrl"
        :is-xyz-expanded="expandedXyz.has(source.id)"
        :copied-url="copiedUrl"
        @toggle-xyz="emit('toggle-xyz', $event)"
        @copy-url="emit('copy-url', $event)"
      />
    </div>
  </div>
</template>
