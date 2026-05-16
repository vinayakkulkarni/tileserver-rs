<script setup lang="ts">
  import { VMap, VControlNavigation, VControlScale } from '@geoql/v-maplibre';
  import {
    ArrowLeft,
    Layers,
    PanelRightClose,
    PanelRightOpen,
  } from '@lucide/vue';

  const route = useRoute('data-data');
  const dataId = computed(() => String(route.params.data));
  const {
    mapOptions,
    layerColors,
    panelOpen,
    navigateBack,
    togglePanel,
    toggleLayerVisibility,
    onMapLoaded,
  } = useDataInspector(dataId);
</script>

<template>
  <div class="relative h-dvh w-full">
    <ClientOnly>
      <VMap :options="mapOptions" class="size-full" @loaded="onMapLoaded">
        <VControlNavigation position="bottom-right" />
        <VControlScale position="bottom-left" />
      </VMap>
    </ClientOnly>

    <button
      class="absolute top-4 left-4 z-10 flex items-center gap-2 border border-border bg-background px-3 py-2 text-sm font-medium shadow-sm transition-colors hover:bg-accent"
      @click="navigateBack"
    >
      <ArrowLeft class="size-4" />
      <Layers class="size-4" />
      <span>{{ dataId }}</span>
    </button>

    <button
      class="absolute top-4 right-4 z-10 flex size-9 items-center justify-center border border-border shadow-sm transition-colors hover:bg-accent"
      :class="
        panelOpen
          ? 'bg-background/95'
          : 'bg-primary text-primary-foreground hover:bg-primary/90'
      "
      @click="togglePanel"
    >
      <PanelRightClose v-if="panelOpen" class="size-4" />
      <PanelRightOpen v-else class="size-4" />
    </button>

    <DataLayersPanel
      :panel-open="panelOpen"
      :layer-colors="layerColors"
      @toggle-layer-visibility="toggleLayerVisibility"
    />
  </div>
</template>
