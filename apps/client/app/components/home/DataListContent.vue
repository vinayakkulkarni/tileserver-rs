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
    toggleXyz: [dataId: string];
    copyUrl: [url: string];
  }>();

  function handleToggleXyz(dataId: string) {
    emit('toggleXyz', dataId);
  }

  function handleCopyUrl(url: string) {
    emit('copyUrl', url);
  }
</script>

<template>
  <Separator class="bg-border/50" />
  <div class="p-4">
    <div v-if="isLoading" class="flex justify-center py-12">
      <div
        class="size-8 animate-spin rounded-full border-2 border-muted border-t-primary"
      ></div>
    </div>
    <div v-else-if="!hasData" class="py-12 text-center">
      <div
        class="mx-auto mb-4 flex size-16 items-center justify-center bg-muted/50"
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
        @toggle-xyz="handleToggleXyz"
        @copy-url="handleCopyUrl"
      />
    </div>
  </div>
</template>
