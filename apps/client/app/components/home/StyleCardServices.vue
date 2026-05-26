<script setup lang="ts">
  import { Check, Copy, Grid3x3, Image } from '@lucide/vue';
  import type { Style } from '~/types/style';

  const props = defineProps<{
    style: Style;
    baseUrl: string;
    isXyzExpanded: boolean;
    copiedUrl: string | null;
  }>();

  const emit = defineEmits<{
    'toggle-xyz': [];
    'copy-url': [url: string];
  }>();

  const xyzUrl = computed(
    () => `${props.baseUrl}/styles/${props.style.id}/{z}/{x}/{y}.png`,
  );
</script>

<template>
  <div class="mt-2.5 flex items-center gap-2">
    <NuxtLink
      :to="`/styles/${style.id}/?raster`"
      class="pill inline-flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium bg-muted text-muted-foreground transition-colors duration-[var(--d-fast,120ms)] hover:bg-muted hover:text-foreground"
    >
      <Image class="size-3.5" />
      Raster
    </NuxtLink>
    <NuxtLink
      :to="`/styles/${style.id}/#2/0/0`"
      class="pill inline-flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-medium bg-muted text-muted-foreground transition-colors duration-[var(--d-fast,120ms)] hover:bg-muted hover:text-foreground"
    >
      <Grid3x3 class="size-3.5" />
      Vector
    </NuxtLink>
  </div>

  <div class="mt-3 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs">
    <span
      class="lbl font-mono text-[10.5px] tracking-[0.10em] uppercase text-muted-foreground font-medium"
      >Services:</span
    >
    <a
      :href="`/styles/${style.id}/style.json`"
      target="_blank"
      class="services-link text-primary font-medium transition-colors duration-[var(--d-fast,120ms)]"
      >GL Style</a
    >
    <span class="sep text-muted-foreground opacity-50">·</span>
    <a
      :href="`/styles/${style.id}.json`"
      target="_blank"
      class="services-link text-primary font-medium transition-colors duration-[var(--d-fast,120ms)]"
      >TileJSON</a
    >
    <span class="sep text-muted-foreground opacity-50">·</span>
    <a
      :href="`/styles/${style.id}/wmts.xml`"
      target="_blank"
      class="services-link text-primary font-medium transition-colors duration-[var(--d-fast,120ms)]"
      >WMTS</a
    >
    <span class="sep text-muted-foreground opacity-50">·</span>
    <button
      type="button"
      class="services-link text-primary font-medium transition-colors duration-[var(--d-fast,120ms)]"
      @click="emit('toggle-xyz')"
    >
      XYZ URL
    </button>
  </div>

  <div v-if="isXyzExpanded" class="mt-2 flex items-center gap-2 bg-muted p-2">
    <code class="flex-1 truncate text-xs text-muted-foreground font-mono">{{
      xyzUrl
    }}</code>
    <Button
      variant="ghost"
      size="icon"
      class="size-7 shrink-0"
      aria-label="Copy XYZ URL"
      @click="emit('copy-url', xyzUrl)"
    >
      <Check v-if="copiedUrl === xyzUrl" class="size-3.5 text-success" />
      <Copy v-else class="size-3.5" />
    </Button>
  </div>
</template>
