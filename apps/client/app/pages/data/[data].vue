<script setup lang="ts">
import maplibregl from 'maplibre-gl'
import type { Map } from 'maplibre-gl'

const route = useRoute()
const dataId = computed(() => route.params.data as string)

const mapContainer = ref<HTMLDivElement | null>(null)
let map: Map | null = null

function addControls() {
  if (!map) return

  map.addControl(
    new maplibregl.ScaleControl({ maxWidth: 80 }),
    'bottom-left'
  )

  map.addControl(
    new maplibregl.NavigationControl({
      showCompass: true,
      showZoom: true,
      visualizePitch: true,
    }),
    'bottom-right'
  )

  map.addControl(
    new maplibregl.GeolocateControl({
      positionOptions: { enableHighAccuracy: true },
      trackUserLocation: true,
    }),
    'bottom-right'
  )
}

onMounted(async () => {
  if (!mapContainer.value) return

  // Dynamically import maplibre-gl-inspect to avoid SSR issues
  const { default: MaplibreInspect } = await import('@maplibre/maplibre-gl-inspect')

  map = new maplibregl.Map({
    container: mapContainer.value,
    style: {
      version: 8,
      sources: {
        'vector-source': {
          type: 'vector',
          url: `/data/${dataId.value}.json`,
        },
      },
      layers: [],
    },
    center: [0, 0],
    zoom: 1,
    hash: true,
  })

  addControls()

  // Add inspector control
  map.addControl(
    new MaplibreInspect({
      showInspectMap: true,
      showInspectButton: false,
    }),
    'top-right'
  )
})

onUnmounted(() => {
  map?.remove()
  map = null
})
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
            {{ dataId }}
          </h1>
        </div>
        <span class="text-sm text-slate-500 dark:text-slate-400">
          Data Inspector
        </span>
      </div>
    </header>

    <!-- Map -->
    <ClientOnly>
      <div ref="mapContainer" class="w-full h-full" />
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
