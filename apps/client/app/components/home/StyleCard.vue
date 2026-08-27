<script setup lang="ts">
  import { Map } from '@lucide/vue';
  import type { Style } from '~/types/style';
  import { useCoverageBar } from '~/composables/use-coverage-bar';

  const props = defineProps<{
    style: Style;
    index: number;
    baseUrl: string;
    isXyzExpanded: boolean;
    copiedUrl: string | null;
  }>();

  const emit = defineEmits<{
    'toggle-xyz': [styleId: string];
    'copy-url': [url: string];
  }>();

  const imgError = ref(false);

  function handleImgError() {
    imgError.value = true;
  }

  function handleToggleXyz() {
    emit('toggle-xyz', props.style.id);
  }

  function handleServiceCopyUrl(url: string) {
    emit('copy-url', url);
  }

  const coverage = useCoverageBar(
    () => props.style.minzoom,
    () => props.style.maxzoom,
  );
</script>

<template>
  <article class="card group p-3.5">
    <div class="flex gap-3.5">
      <div
        class="thumb size-14 shrink-0 overflow-hidden border border-border bg-muted grid place-items-center"
      >
        <img
          v-if="!imgError"
          :src="apiUrl(`/styles/${style.id}/static/0,0,1/160x160.png`)"
          :alt="style.name"
          class="size-full object-cover"
          loading="lazy"
          @error="handleImgError"
        />
        <Map v-else class="size-5.5 text-muted-foreground" />
      </div>

      <div class="card-main min-w-0 flex-1">
        <div class="card-top flex items-start justify-between gap-2.5">
          <div class="min-w-0">
            <h3
              class="card-title text-15/1-3 font-bold tracking-n5"
            >
              {{ style.name }}
            </h3>
            <p class="mt-1.5">
              <code
                class="card-id font-mono text-11 bg-muted px-1.5 py-0.5 text-muted-foreground tracking-wide"
              >
                {{ style.id }}
              </code>
            </p>
          </div>
          <Button as-child size="sm" class="shrink-0">
            <NuxtLink :to="`/styles/${style.id}/`">
              <Map class="size-4 mr-1.5" />
              Viewer
            </NuxtLink>
          </Button>
        </div>

        <HomeStyleCardServices
          :style="style"
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
