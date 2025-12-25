<script setup lang="ts">
  import {
    VMap,
    VControlNavigation,
    VControlScale,
    VControlGeolocate,
  } from '@geoql/v-maplibre';
  import { ArrowLeft, Layers } from 'lucide-vue-next';

  const route = useRoute('data-data');
  const dataId = computed(() => route.params.data);

  const { mapOptions, onMapLoaded } = useDataInspector(dataId);
</script>

<template>
  <div class="relative h-dvh w-full">
    <!-- Floating back button -->
    <NuxtLink
      to="/"
      class="
        absolute top-4 left-4 z-10 flex items-center gap-2 rounded-lg border
        border-slate-200 bg-white/90 px-3 py-2 text-sm font-medium
        text-slate-700 shadow-lg backdrop-blur-sm transition-colors
        hover:bg-white
        dark:border-slate-700 dark:bg-slate-800/90 dark:text-slate-200
        dark:hover:bg-slate-800
      "
    >
      <ArrowLeft class="size-4" />
      <Layers class="size-4" />
      <span>{{ dataId }}</span>
    </NuxtLink>

    <!-- Full-screen Map -->
    <ClientOnly>
      <VMap
        :options="mapOptions"
        :support-pmtiles="false"
        class="size-full"
        @loaded="onMapLoaded"
      >
        <VControlScale position="bottom-left" />
        <VControlNavigation position="bottom-right" />
        <VControlGeolocate position="bottom-right" />
      </VMap>
    </ClientOnly>
  </div>
</template>
