<script setup lang="ts">
  import { VMap, VControlNavigation, VControlScale } from '@geoql/v-maplibre';
  import { ArrowLeft, Layers, PanelRightClose, PanelRightOpen } from 'lucide-vue-next';
  import { motion, AnimatePresence } from 'motion-v';

  const route = useRoute('data-data');
  const dataId = computed(() => String(route.params.data));
  const { mapOptions, layerColors, onMapLoaded } = useDataInspector(dataId);

  const panelOpen = ref(true);

  function navigateBack() {
    if (import.meta.client) {
      history.replaceState(null, '', window.location.pathname);
    }
    navigateTo('/');
  }

  function togglePanel() {
    panelOpen.value = !panelOpen.value;
  }
</script>

<template>
  <div class="relative h-dvh w-full">
    <!-- Map (full screen) -->
    <ClientOnly>
      <VMap :options="mapOptions" class="size-full" @loaded="onMapLoaded">
        <VControlNavigation position="bottom-right" />
        <VControlScale position="bottom-left" />
      </VMap>
    </ClientOnly>

    <!-- Back button -->
    <button
      class="absolute top-4 left-4 z-10 flex items-center gap-2 rounded-lg border bg-background/95 px-3 py-2 text-sm font-medium shadow-lg backdrop-blur-sm transition-colors hover:bg-accent"
      @click="navigateBack"
    >
      <ArrowLeft class="size-4" />
      <Layers class="size-4" />
      <span>{{ dataId }}</span>
    </button>

    <!-- Panel toggle button -->
    <button
      class="absolute top-4 right-4 z-10 flex size-9 items-center justify-center rounded-lg border shadow-lg backdrop-blur-sm transition-colors hover:bg-accent"
      :class="panelOpen ? 'bg-background/95' : 'bg-primary text-primary-foreground hover:bg-primary/90'"
      @click="togglePanel"
    >
      <PanelRightClose v-if="panelOpen" class="size-4" />
      <PanelRightOpen v-else class="size-4" />
    </button>

    <!-- Collapsible layers panel -->
    <AnimatePresence>
      <motion.div
        v-if="panelOpen"
        :initial="{ opacity: 0, x: 20, scale: 0.95 }"
        :animate="{ opacity: 1, x: 0, scale: 1 }"
        :exit="{ opacity: 0, x: 20, scale: 0.95 }"
        :transition="{ type: 'spring', stiffness: 300, damping: 25 }"
        class="absolute top-16 right-4 z-10 w-56 rounded-lg border bg-background/95 p-4 shadow-lg backdrop-blur-sm"
      >
        <h3 class="mb-3 text-sm font-semibold">Layers</h3>
        <div class="space-y-1.5">
          <div
            v-for="layer in layerColors"
            :key="layer.id"
            class="flex items-center gap-2 text-sm"
          >
            <div
              class="size-3.5 shrink-0 rounded-sm"
              :style="{ backgroundColor: layer.color }"
            ></div>
            <span class="truncate text-muted-foreground">{{ layer.id }}</span>
          </div>
          <div
            v-if="layerColors.length === 0"
            class="text-sm text-muted-foreground"
          >
            Loading layers...
          </div>
        </div>
      </motion.div>
    </AnimatePresence>
  </div>
</template>
