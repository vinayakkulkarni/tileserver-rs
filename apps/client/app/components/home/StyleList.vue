<script setup lang="ts">
  import { ChevronRight, Palette } from '@lucide/vue';
  import { motion } from 'motion-v';
  import type { Style } from '~/types/style';

  defineProps<{
    styles: Style[];
    isLoading: boolean;
    hasStyles: boolean;
    isOpen: boolean;
    searchQuery: string;
    baseUrl: string;
    expandedXyz: Set<string>;
    copiedUrl: string | null;
  }>();

  const emit = defineEmits<{
    'update:isOpen': [value: boolean];
    toggleXyz: [styleId: string];
    copyUrl: [url: string];
  }>();

  function handleToggleXyz(styleId: string) {
    emit('toggleXyz', styleId);
  }

  function handleCopyUrl(url: string) {
    emit('copyUrl', url);
  }
</script>

<template>
  <motion.div
    :initial="{ opacity: 0, y: 12 }"
    :animate="{ opacity: 1, y: 0 }"
    :transition="{ duration: 0.35, delay: 0.15 }"
  >
    <Collapsible :open="isOpen" @update:open="$emit('update:isOpen', $event)">
      <Card
        class="overflow-hidden border-border/50 bg-card/50 backdrop-blur-sm"
      >
        <CollapsibleTrigger
          class="flex w-full items-center gap-3 p-4 transition-colors hover:bg-muted/30"
        >
          <ChevronRight
            class="size-4 text-muted-foreground transition-transform duration-200"
            :class="{ 'rotate-90': isOpen }"
          />
          <div class="flex size-8 items-center justify-center bg-primary/10">
            <Palette class="size-4 text-primary" />
          </div>
          <span class="font-medium">Styles</span>
          <Badge variant="secondary" class="ml-auto">{{ styles.length }}</Badge>
        </CollapsibleTrigger>
        <CollapsibleContent>
          <HomeStyleListContent
            :styles="styles"
            :is-loading="isLoading"
            :has-styles="hasStyles"
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
