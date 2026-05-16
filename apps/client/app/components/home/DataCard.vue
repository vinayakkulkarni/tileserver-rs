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
    class="group rounded-xl border border-border/50 bg-background/50 p-4 transition-all hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5"
  >
    <div class="flex items-start gap-4">
      <div
        class="flex size-12 shrink-0 items-center justify-center rounded-xl bg-muted ring-1 ring-border/50"
      >
        <Layers class="size-6 text-muted-foreground" />
      </div>

      <div class="min-w-0 flex-1">
        <div class="flex items-start justify-between gap-2">
          <div>
            <h3 class="font-semibold">{{ source.name || source.id }}</h3>
            <p
              class="mt-0.5 flex flex-wrap items-center gap-2 text-sm text-muted-foreground"
            >
              <code
                class="rounded-md bg-muted px-1.5 py-0.5 text-xs font-medium"
                >{{ source.id }}</code
              >
              <Badge variant="outline" class="rounded-md text-[10px]">
                z{{ source.minzoom }}-{{ source.maxzoom }}
              </Badge>
            </p>
          </div>
          <Button
            v-if="source.vector_layers?.length"
            as-child
            variant="secondary"
            size="sm"
            class="rounded-lg"
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
