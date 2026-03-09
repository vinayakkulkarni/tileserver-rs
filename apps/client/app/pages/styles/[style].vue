<script setup lang="ts">
  import {
    VMap,
    VControlNavigation,
    VControlScale,
    VControlGeolocate,
  } from '@geoql/v-maplibre';
  import { ArrowLeft, Palette, Sparkles } from 'lucide-vue-next';
  import { useEventListener } from '@vueuse/core';

  const route = useRoute('styles-style');
  const styleId = computed(() => String(route.params.style));
  const isRaster = computed(() => 'raster' in route.query);
  const isScreenshot = computed(() => 'screenshot' in route.query);

  const { mapOptions, mapRef, isLoading, navigateBack, onMapLoaded } = useStyleViewer(
    styleId,
    isRaster,
  );

  // File drop overlay
  const {
    dropZoneRef,
    status: dropStatus,
    overlays,
    lastError: dropError,
    lastSuccess: dropSuccess,
    isOverDropZone,
    hasOverlays,
    toggleOverlay,
    removeOverlay,
    removeAllOverlays,
  } = useFileDrop(mapRef);

  const chatOpen = ref(false);

  function toggleChat() {
    chatOpen.value = !chatOpen.value;
  }

  // ⌘K / Ctrl+K to toggle command palette, Esc to close
  useEventListener('keydown', (e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      toggleChat();
    }
    if (e.key === 'Escape' && chatOpen.value) {
      chatOpen.value = false;
    }
  });
</script>

<template>
  <div ref="dropZoneRef" class="relative h-dvh w-full">
    <!-- Back button (hidden for screenshots) -->
    <button
      v-if="!isScreenshot"
      class="absolute top-4 left-4 z-10 flex items-center gap-2 border border-border bg-background px-3 py-2 text-sm font-medium shadow-sm transition-colors hover:bg-accent"
      @click="navigateBack"
    >
      <ArrowLeft class="size-4" />
      <Palette class="size-4" />
      <span>{{ styleId }}</span>
    </button>

    <!-- Bottom dock: AI chat trigger (hidden for screenshots, hidden when chat is open) -->
    <button
      v-if="!isScreenshot && !chatOpen"
      class="absolute bottom-6 left-1/2 z-10 flex -translate-x-1/2 items-center gap-3 border border-border bg-background/95 px-5 py-2.5 shadow-lg backdrop-blur-sm transition-all hover:border-primary/50 hover:shadow-xl"
      @click="toggleChat"
    >
      <Sparkles class="size-4 text-primary" />
      <span class="text-sm text-muted-foreground">Ask about the map…</span>
      <kbd class="border border-border/60 bg-muted/50 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">⌘K</kbd>
    </button>

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
        <VMap :options="mapOptions" :support-pmtiles="false" class="size-full" @loaded="onMapLoaded">
          <VControlScale v-if="!isScreenshot" position="bottom-left" :unit="'metric'" />
          <VControlNavigation v-if="!isScreenshot" position="bottom-right" />
          <VControlGeolocate v-if="!isScreenshot" position="bottom-right" />
        </VMap>
      </ClientOnly>
    </div>

    <!-- File drop overlay + toast notifications -->
    <MapDropOverlay :status="dropStatus" :is-over="isOverDropZone" :error="dropError" :success="dropSuccess" />

    <!-- Overlay layer panel -->
    <MapOverlayPanel
      v-if="hasOverlays"
      :overlays="overlays"
      @toggle="toggleOverlay"
      @remove="removeOverlay"
      @remove-all="removeAllOverlays"
    />

    <!-- LLM Command Palette -->
    <ClientOnly>
      <LlmPalette
        :open="chatOpen"
        :map-ref="mapRef"
        @update:open="chatOpen = $event"
      />
    </ClientOnly>
  </div>
</template>
