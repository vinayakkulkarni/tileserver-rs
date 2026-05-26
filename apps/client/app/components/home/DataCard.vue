<script setup lang="ts">
  import { Layers } from '@lucide/vue';
  import type { Data } from '~/types/data';
  import { useCoverageBar } from '~/composables/use-coverage-bar';

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

  const coverage = useCoverageBar(
    () => props.source.minzoom,
    () => props.source.maxzoom,
  );
</script>

<template>
  <article class="card group p-3.5">
    <div class="flex gap-3.5">
      <div
        class="thumb size-14 shrink-0 border border-border bg-muted grid place-items-center"
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
                class="card-id font-mono text-[11px] bg-muted px-1.5 py-0.5 text-muted-foreground tracking-wide"
              >
                {{ source.id }}
              </code>
              <span
                class="badge-outline font-mono text-[10px] tracking-[0.12em] uppercase text-muted-foreground px-1.5 py-0.5 border border-border font-medium"
              >
                {{ coverage.rangeLabel.value }}
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
            :aria-label="coverage.ariaLabel.value"
          >
            <div
              class="coverage-fill"
              :style="{
                left: coverage.left.value,
                width: coverage.width.value,
              }"
            ></div>
          </div>
          <div class="coverage-labels">
            <span>{{ coverage.floorLabel }}</span>
            <span>{{ coverage.ceilingLabel }}</span>
          </div>
        </div>
      </div>
    </div>
  </article>
</template>
