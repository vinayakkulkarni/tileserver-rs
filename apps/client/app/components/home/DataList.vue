<script setup lang="ts">
  import { ChevronRight, Database } from '@lucide/vue';
  import { motion } from 'motion-v';
  import type { Data } from '~/types/data';

  defineProps<{
    sources: Data[];
    isLoading: boolean;
    hasData: boolean;
    isOpen: boolean;
    searchQuery: string;
    baseUrl: string;
    expandedXyz: Set<string>;
    copiedUrl: string | null;
  }>();

  const emit = defineEmits<{
    'update:isOpen': [value: boolean];
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
  <motion.div
    :initial="{ opacity: 0, y: 12 }"
    :animate="{ opacity: 1, y: 0 }"
    :transition="{ duration: 0.35, delay: 0.25 }"
  >
    <Collapsible :open="isOpen" @update:open="$emit('update:isOpen', $event)">
      <Card
        class="overflow-hidden rounded-xl border-border/50 bg-card/50 backdrop-blur-sm"
      >
        <CollapsibleTrigger
          class="flex w-full items-center gap-3 p-4 transition-colors hover:bg-muted/30"
        >
          <ChevronRight
            class="size-4 text-muted-foreground transition-transform duration-200"
            :class="{ 'rotate-90': isOpen }"
          />
          <div
            class="flex size-8 items-center justify-center rounded-lg bg-primary/10"
          >
            <Database class="size-4 text-primary" />
          </div>
          <span class="font-medium">Data Sources</span>
          <Badge variant="secondary" class="ml-auto rounded-lg">{{
            sources.length
          }}</Badge>
        </CollapsibleTrigger>
        <CollapsibleContent>
          <HomeDataListContent
            :sources="sources"
            :is-loading="isLoading"
            :has-data="hasData"
            :search-query="searchQuery"
            :base-url="baseUrl"
            :expanded-xyz="expandedXyz"
            :copied-url="copiedUrl"
            @toggle-xyz="handleToggleXyz"
            @copy-url="handleCopyUrl"
          />
        </CollapsibleContent>
      </Card>
    </Collapsible>
  </motion.div>
</template>
