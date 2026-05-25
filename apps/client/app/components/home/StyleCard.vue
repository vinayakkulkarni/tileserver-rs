<script setup lang="ts">
  import { Map } from '@lucide/vue';
  import type { Style } from '~/types/style';

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

  const coverageLeft = computed(() => (props.style.minzoom * 100) / 18);
  const coverageWidth = computed(
    () => ((props.style.maxzoom - props.style.minzoom) * 100) / 18,
  );
</script>

<template>
  <article
    class="card group border border-border/50 bg-background/50 p-4 transition-[border-color,background,box-shadow] duration-[var(--d-fast,120ms)]"
    :style="{
      '--tw-ring-color': 'oklch(from var(--color-primary) l c h / 0.15)',
    }"
  >
    <div class="flex gap-4">
      <div
        class="flex size-20 shrink-0 items-center justify-center overflow-hidden bg-muted ring-1 ring-border/50"
      >
        <img
          v-if="!imgError"
          :src="`/styles/${style.id}/static/0,0,1/160x160.png`"
          :alt="style.name"
          class="size-full object-cover"
          loading="lazy"
          @error="handleImgError"
        />
        <Map v-else class="size-8 text-muted-foreground" />
      </div>

      <div class="min-w-0 flex-1">
        <div class="flex items-start justify-between gap-2">
          <div>
            <h3 class="font-semibold">{{ style.name }}</h3>
            <p class="mt-0.5 text-sm text-muted-foreground">
              <code class="bg-muted px-1.5 py-0.5 text-xs font-medium">{{
                style.id
              }}</code>
            </p>
          </div>
          <Button as-child size="sm">
            <NuxtLink :to="`/styles/${style.id}/`">
              <Map class="mr-1.5 size-4" />
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
            class="coverage h-[3px] w-full bg-muted"
            role="img"
            :aria-label="`Zoom range ${style.minzoom} to ${style.maxzoom}`"
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
            <span>z{{ style.minzoom }}</span>
            <span>z{{ style.minzoom }}–{{ style.maxzoom }}</span>
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
