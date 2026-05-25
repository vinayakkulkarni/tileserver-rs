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
    toggleXyz: [styleId: string];
    copyUrl: [url: string];
  }>();

  const imgError = ref(false);

  function handleImgError() {
    imgError.value = true;
  }

  function handleToggleXyz() {
    emit('toggleXyz', props.style.id);
  }

  function handleServiceCopyUrl(url: string) {
    emit('copyUrl', url);
  }

  const coverageLeft = computed(
    () => `${((props.style.minzoom ?? 0) * 100) / 18}%`,
  );
  const coverageWidth = computed(
    () =>
      `${(((props.style.maxzoom ?? 18) - (props.style.minzoom ?? 0)) * 100) / 18}%`,
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
        class="thumb size-14 shrink-0 overflow-hidden border border-border bg-surface-2 grid place-items-center"
      >
        <img
          v-if="!imgError"
          :src="`/styles/${style.id}/static/0,0,1/160x160.png`"
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
              class="card-title text-[15px] font-bold tracking-[-0.005em] leading-[1.3]"
            >
              {{ style.name }}
            </h3>
            <p class="mt-1.5">
              <code
                class="card-id font-mono text-[11px] bg-surface-2 px-1.5 py-0.5 text-muted-foreground tracking-wide"
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
            :aria-label="`Zoom range ${style.minzoom ?? 0} to ${style.maxzoom ?? 18}`"
          >
            <div
              class="coverage-fill"
              :style="{ left: coverageLeft, width: coverageWidth }"
            ></div>
          </div>
          <div class="coverage-labels">
            <span>z{{ style.minzoom ?? 0 }}</span>
            <span>z{{ style.maxzoom ?? 18 }}</span>
          </div>
        </div>
      </div>
    </div>
  </article>
</template>
