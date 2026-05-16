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
    toggleXyz: [];
    copyUrl: [url: string];
  }>();

  const xyzUrl = computed(
    () => `${props.baseUrl}/styles/${props.style.id}/{z}/{x}/{y}.png`,
  );
</script>

<template>
  <div class="mt-2 flex items-center gap-3">
    <NuxtLink
      :to="`/styles/${style.id}/?raster`"
      class="flex items-center gap-1.5 rounded-lg bg-muted/50 px-2.5 py-1 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
    >
      <Image class="size-3.5" />
      Raster
    </NuxtLink>
    <NuxtLink
      :to="`/styles/${style.id}/#2/0/0`"
      class="flex items-center gap-1.5 rounded-lg bg-muted/50 px-2.5 py-1 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
    >
      <Grid3x3 class="size-3.5" />
      Vector
    </NuxtLink>
  </div>

  <div class="mt-3 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs">
    <span class="text-muted-foreground">Services:</span>
    <a
      :href="`/styles/${style.id}/style.json`"
      target="_blank"
      class="text-primary hover:underline"
      >GL Style</a
    >
    <span class="text-muted-foreground/30">•</span>
    <a
      :href="`/styles/${style.id}.json`"
      target="_blank"
      class="text-primary hover:underline"
      >TileJSON</a
    >
    <span class="text-muted-foreground/30">•</span>
    <a
      :href="`/styles/${style.id}/wmts.xml`"
      target="_blank"
      class="text-primary hover:underline"
      >WMTS</a
    >
    <span class="text-muted-foreground/30">•</span>
    <button class="text-primary hover:underline" @click="emit('toggleXyz')">
      XYZ URL
    </button>
  </div>

  <div
    v-if="isXyzExpanded"
    class="mt-2 flex items-center gap-2 rounded-lg bg-muted/50 p-2"
  >
    <code class="flex-1 truncate text-xs text-muted-foreground">{{
      xyzUrl
    }}</code>
    <Button
      variant="ghost"
      size="icon"
      class="size-7 shrink-0 rounded-lg"
      @click="emit('copyUrl', xyzUrl)"
    >
      <Check v-if="copiedUrl === xyzUrl" class="size-3.5 text-green-500" />
      <Copy v-else class="size-3.5" />
    </Button>
  </div>
</template>
