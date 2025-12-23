<script setup lang="ts">
import { VMap, VControlScale, VControlNavigation, VControlGeolocate } from '@geoql/v-maplibre'
import { useMapStyle } from '~/composables/use-map-style'

const route = useRoute()
const styleId = computed(() => route.params.style as string)
const isRaster = computed(() => 'raster' in route.query)

const { style, isLoading } = useMapStyle(styleId, isRaster)

const mapOptions = computed(() => ({
  style: style.value,
  center: [0, 0] as [number, number],
  zoom: 1,
  hash: true,
}))
</script>

<template>
  <div class="relative w-full h-screen">
    <!-- Header -->
    <header class="absolute top-0 left-0 right-0 z-10 bg-white/90 dark:bg-slate-900/90 backdrop-blur-sm border-b border-slate-200 dark:border-slate-700">
      <div class="flex items-center justify-between px-4 py-3">
        <div class="flex items-center gap-4">
          <NuxtLink
            to="/"
            class="text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 transition-colors"
          >
            <svg class="size-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
            </svg>
          </NuxtLink>
          <h1 class="text-lg font-semibold text-slate-800 dark:text-slate-100">
            {{ styleId }}
          </h1>
        </div>
        <span class="text-sm text-slate-500 dark:text-slate-400">
          Style Viewer
        </span>
      </div>
    </header>

    <!-- Loading -->
    <div v-if="isLoading" class="w-full h-full flex items-center justify-center">
      <span class="text-slate-500 dark:text-slate-400">Loading style...</span>
    </div>

    <!-- Map -->
    <ClientOnly v-else>
      <VMap class="w-full h-full" :options="mapOptions">
        <VControlScale position="bottom-left" :options="{ maxWidth: 80 }" />
        <VControlNavigation
          position="bottom-right"
          :options="{ showCompass: true, showZoom: true, visualizePitch: true }"
        />
        <VControlGeolocate
          position="bottom-right"
          :options="{ positionOptions: { enableHighAccuracy: true }, trackUserLocation: true }"
        />
      </VMap>
    </ClientOnly>

    <!-- Footer -->
    <footer class="absolute bottom-0 left-0 right-0 z-10 bg-white/90 dark:bg-slate-900/90 backdrop-blur-sm border-t border-slate-200 dark:border-slate-700">
      <div class="flex items-center justify-between px-4 py-2 text-sm text-slate-500 dark:text-slate-400">
        <span>Tileserver RS</span>
        <span>Powered by MapLibre GL JS</span>
      </div>
    </footer>
  </div>
</template>
