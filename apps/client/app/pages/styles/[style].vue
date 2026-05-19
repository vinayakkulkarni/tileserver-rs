<script setup lang="ts">
  import {
    VMap,
    VControlNavigation,
    VControlScale,
    VControlGeolocate,
  } from '@geoql/v-maplibre';

  const route = useRoute('styles-style');
  const styleId = computed(() => String(route.params.style));
  const isRaster = computed(() => 'raster' in route.query);
  const isScreenshot = computed(() => 'screenshot' in route.query);

  const { mapOptions, mapRef, isLoading, navigateBack, onMapLoaded } =
    useStyleViewer(styleId, isRaster);

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

  const { chatMode, isChatVisible, toggleChat } = useStyleViewerChat();
</script>

<template>
  <div ref="dropZoneRef" class="relative h-dvh w-full">
    <StylesViewerToolbar
      :style-id="styleId"
      :is-screenshot="isScreenshot"
      :is-chat-visible="isChatVisible"
      @navigate-back="navigateBack"
      @toggle-chat="toggleChat"
    />

    <div
      v-if="isLoading"
      class="flex size-full items-center justify-center bg-muted"
    >
      <span class="text-muted-foreground"> Loading style... </span>
    </div>

    <div
      v-if="!isLoading && mapOptions"
      class="absolute inset-0 size-full overflow-hidden"
    >
      <ClientOnly>
        <VMap
          :options="mapOptions"
          :support-pmtiles="false"
          class="size-full"
          @loaded="onMapLoaded"
        >
          <VControlScale
            v-if="!isScreenshot"
            position="bottom-left"
            :unit="'metric'"
          />
          <VControlNavigation v-if="!isScreenshot" position="bottom-right" />
          <VControlGeolocate v-if="!isScreenshot" position="bottom-right" />
        </VMap>
      </ClientOnly>
    </div>

    <MapDropOverlay
      :status="dropStatus"
      :is-over="isOverDropZone"
      :error="dropError"
      :success="dropSuccess"
    />

    <MapOverlayPanel
      v-if="hasOverlays"
      :overlays="overlays"
      @toggle="toggleOverlay"
      @remove="removeOverlay"
      @remove-all="removeAllOverlays"
    />

    <ClientOnly>
      <LlmPalette
        v-if="isChatVisible"
        :mode="chatMode"
        :map-ref="mapRef"
        :overlays="overlays"
        @update:mode="chatMode = $event"
      />
    </ClientOnly>
  </div>
</template>
