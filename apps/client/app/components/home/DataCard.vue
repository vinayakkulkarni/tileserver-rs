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
    toggleXyz: [dataId: string];
    copyUrl: [url: string];
  }>();

  function handleToggleXyz() {
    emit('toggleXyz', props.source.id);
  }

  function handleServiceCopyUrl(url: string) {
    emit('copyUrl', url);
  }

  const coverageLeft = computed(
    () => `${((props.source.minzoom ?? 0) * 100) / 18}%`,
  );
  const coverageWidth = computed(
    () =>
      `${(((props.source.maxzoom ?? 18) - (props.source.minzoom ?? 0)) * 100) / 18}%`,
  );
</script>

<template>
  <article
    class="card group border border-border bg-background p-3.5 transition-all duration-[var(--d-fast,120ms)] hover:border-primary hover:bg-primary/10 focus-within:border-primary"
    style="
      --tw-shadow: inset 0 0 0 1px oklch(from var(--color-primary) l c h / 0.15);
    "
  >
    <div class="flex gap-3.5">
      <div
        class="thumb size-14 shrink-0 border border-border bg-surface-2 grid place-items-center"
      >
        <Layers class="size-5.5 text-muted-foreground" />
      </div>

      <div class="card-main min-w-0 flex-1">
        <div class="card-top flex items-start justify-between gap-2.5">
          <div class="min-w-0">
            <h3
              class="card-title text-[15px] font-bold tracking-[-0.005em] leading-[1.3]"
            >
              {{ source.name || source.id }}
            </h3>
            <p class="mt-1.5 flex flex-wrap items-center gap-2">
              <code
                class="card-id font-mono text-[11px] bg-surface-2 px-1.5 py-0.5 text-muted-foreground tracking-wide"
              >
                {{ source.id }}
              </code>
              <span
                class="badge-outline font-mono text-[10px] tracking-[0.12em] uppercase text-muted-foreground px-1.5 py-0.5 border border-border font-medium"
              >
                z{{ source.minzoom ?? 0 }}–{{ source.maxzoom ?? 18 }}
              </span>
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
              <Layers class="size-4 mr-1.5" />
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
            class="coverage"
            role="img"
            :aria-label="`Zoom range ${source.minzoom ?? 0} to ${source.maxzoom ?? 18}`"
          >
            <div
              class="coverage-fill"
              :style="{ left: coverageLeft, width: coverageWidth }"
            ></div>
          </div>
          <div class="coverage-labels">
            <span>z{{ source.minzoom ?? 0 }}</span>
            <span>z{{ source.maxzoom ?? 18 }}</span>
          </div>
        </div>
      </div>
    </div>
  </article>
</template>
