<script setup lang="ts">
  import { Layers } from '@lucide/vue';
  import { motion } from 'motion-v';
  import type { Data } from '~/types/data';

  const props = defineProps<{
    source: Data;
    index: number;
    baseUrl: string;
    isXyzExpanded: boolean;
    copiedUrl: string | null;
  }>();

  const emit = defineEmits<{
    toggleXyz: [dataId: string];
    copyUrl: [url: string];
  }>();

  function handleToggleXyz() {
    emit('toggleXyz', props.source.id);
  }

  function handleServiceCopyUrl(url: string) {
    emit('copyUrl', url);
  }
</script>

<template>
  <motion.div
    :initial="{ opacity: 0, y: 12 }"
    :animate="{ opacity: 1, y: 0 }"
    :transition="{ duration: 0.3, delay: 0.05 * index }"
    class="group border border-border bg-background p-3.5 transition-all duration-[var(--d-fast,120ms)] hover:border-primary hover:bg-primary/2.5 focus-within:border-primary"
  >
    <div class="flex gap-3.5">
      <!-- 56x56 icon box per A2 -->
      <div
        class="flex size-14 shrink-0 items-center justify-center border border-border bg-surface-2"
      >
        <Layers class="size-5.5 text-muted-foreground" />
      </div>

      <!-- Card content -->
      <div class="min-w-0 flex-1">
        <div class="flex items-start justify-between gap-2.5">
          <div class="min-w-0">
            <h3 class="text-[15px] font-bold leading-snug tracking-[-0.005em]">
              {{ source.name || source.id }}
            </h3>
            <p class="mt-0.5 flex flex-wrap items-center gap-2">
              <code
                class="inline-block bg-surface-2 px-1.5 py-0.5 font-mono text-[11px] font-medium text-muted-foreground"
              >
                {{ source.id }}
              </code>
              <Badge
                variant="outline"
                class="font-mono text-[10px] tracking-[0.06em]"
              >
                z{{ source.minzoom }}–{{ source.maxzoom }}
              </Badge>
            </p>
          </div>
          <Button
            v-if="source.vector_layers?.length"
            as-child
            variant="secondary"
            size="sm"
            class="shrink-0"
          >
            <NuxtLink :to="`/data/${source.id}/`">
              <Layers class="mr-1.5 size-4" />
              Inspect
            </NuxtLink>
          </Button>
        </div>

        <HomeDataCardServices
          :source="source"
          :base-url="baseUrl"
          :is-xyz-expanded="isXyzExpanded"
          :copied-url="copiedUrl"
          @toggle-xyz="handleToggleXyz"
          @copy-url="handleServiceCopyUrl"
        />
      </div>
    </div>
  </motion.div>
</template>
