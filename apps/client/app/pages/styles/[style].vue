<script setup lang="ts">
  import {
    VMap,
    VControlNavigation,
    VControlScale,
    VControlGeolocate,
  } from '@geoql/v-maplibre';
  import { ArrowLeft, Palette } from 'lucide-vue-next';

  const route = useRoute('styles-style');
  const styleId = computed(() => String(route.params.style));
  const isRaster = computed(() => 'raster' in route.query);
  const isScreenshot = computed(() => 'screenshot' in route.query);

  const { mapOptions, isLoading } = useStyleViewer(styleId, isRaster);
</script>

<template>
  <div class="relative h-dvh w-full">
    <!-- Floating back button (hidden for screenshots) -->
    <NuxtLink
      v-if="!isScreenshot"
      to="/"
      class="absolute top-4 left-4 z-10 flex items-center gap-2 rounded-lg border border-slate-200 bg-white/90 px-3 py-2 text-sm font-medium text-slate-700 shadow-lg backdrop-blur-sm transition-colors hover:bg-white dark:border-slate-700 dark:bg-slate-800/90 dark:text-slate-200 dark:hover:bg-slate-800"
    >
      <ArrowLeft class="size-4" />
      <Palette class="size-4" />
      <span>{{ styleId }}</span>
    </NuxtLink>

    <!-- Loading -->
    <div
      v-if="isLoading"
      class="flex size-full items-center justify-center bg-slate-100 dark:bg-slate-900"
    >
      <span class="text-slate-500 dark:text-slate-400"> Loading style... </span>
    </div>

    <!-- Map - VMap wrapped in ClientOnly -->
    <div
      v-if="!isLoading && mapOptions"
      class="absolute inset-0 size-full overflow-hidden"
    >
      <ClientOnly>
        <VMap :options="mapOptions" :support-pmtiles="false" class="size-full">
          <VControlScale v-if="!isScreenshot" position="bottom-left" />
          <VControlNavigation v-if="!isScreenshot" position="bottom-right" />
          <VControlGeolocate v-if="!isScreenshot" position="bottom-right" />
        </VMap>
      </ClientOnly>
    </div>
  </div>
</template>
