<script setup lang="ts">
  import { Layers } from '@lucide/vue';
  import type { Data } from '~/types/data';

  const props = defineProps<{
    source: Data;
    index: number;
    baseUrl: string;
    isXyzExpanded: boolean;
    copiedUrl: string | null;
  }>();

  const emit = defineEmits<{
    'toggle-xyz': [dataId: string];
    'copy-url': [url: string];
  }>();

  function handleToggleXyz() {
    emit('toggle-xyz', props.source.id);
  }

  function handleServiceCopyUrl(url: string) {
    emit('copy-url', url);
  }

  const coverageLeft = computed(() => (props.source.minzoom * 100) / 18);
  const coverageWidth = computed(
    () => ((props.source.maxzoom - props.source.minzoom) * 100) / 18,
  );
</script>

<template>
  <article
    class="card group border border-border/50 bg-background/50 p-4 transition-[border-color,background,box-shadow] duration-[var(--d-fast,120ms)]"
    :style="{
      '--tw-ring-color': 'oklch(from var(--color-primary) l c h / 0.15)',
    }"
  >
    <div class="flex items-start gap-4">
      <div
        class="flex size-12 shrink-0 items-center justify-center bg-muted ring-1 ring-border/50"
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
              <code class="bg-muted px-1.5 py-0.5 text-xs font-medium">{{
                source.id
              }}</code>
              <Badge variant="outline" class="text-[10px]">
                z{{ source.minzoom }}-{{ source.maxzoom }}
              </Badge>
            </p>
          </div>
          <Button
            v-if="source.vector_layers?.length"
            as-child
            variant="secondary"
            size="sm"
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

        <div class="mt-3">
          <div
            class="coverage h-[3px] w-full bg-muted"
            role="img"
            :aria-label="`Zoom range ${source.minzoom} to ${source.maxzoom}`"
          >
            <div
              class="coverage-fill h-full bg-primary"
              :style="{ left: `${coverageLeft}%`, width: `${coverageWidth}%` }"
            ></div>
          </div>
          <div
            class="mt-1 flex justify-between text-[10px] font-mono tracking-widest text-muted-foreground"
            style="letter-spacing: 0.1em"
          >
            <span>z{{ source.minzoom }}</span>
            <span>z{{ source.minzoom }}–{{ source.maxzoom }}</span>
            <span>z18</span>
          </div>
        </div>
      </div>
    </div>
  </article>
</template>

<style scoped>
  .card:hover {
    border-color: var(--color-primary);
    background: oklch(from var(--color-primary) l c h / 0.025);
    box-shadow: inset 0 0 0 1px oklch(from var(--color-primary) l c h / 0.15);
  }

  .card:focus-within {
    border-color: var(--color-primary);
  }
</style>
