<script setup lang="ts">
  import { VMap, VControlNavigation, VControlScale } from '@geoql/v-maplibre';
  import { ArrowLeft, Layers } from 'lucide-vue-next';

  const route = useRoute('data-data');
  const dataId = computed(() => route.params.data);
  const { mapOptions, layerColors, onMapLoaded } = useDataInspector(dataId);
</script>

<template>
  <div class="relative flex h-dvh w-full">
    <!-- Map container -->
    <div class="relative flex-1">
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

      <!-- Map -->
      <ClientOnly>
        <VMap :options="mapOptions" class="size-full" @loaded="onMapLoaded">
          <VControlNavigation position="bottom-right" />
          <VControlScale position="bottom-left" />
        </VMap>
      </ClientOnly>
    </div>

    <!-- Layer list sidebar -->
    <div
      class="
        w-60 shrink-0 overflow-auto border-l border-slate-200 bg-white p-4
        dark:border-slate-700 dark:bg-slate-900
      "
    >
      <h2
        class="
          mb-3 text-sm font-semibold text-slate-900
          dark:text-slate-100
        "
      >
        Layers
      </h2>
      <div class="space-y-1.5">
        <div
          v-for="layer in layerColors"
          :key="layer.id"
          class="
            flex items-center gap-2 text-sm text-slate-700
            dark:text-slate-300
          "
        >
          <div
            class="size-3.5 shrink-0 rounded-sm"
            :style="{ backgroundColor: layer.color }"
          ></div>
          <span class="truncate">{{ layer.id }}</span>
        </div>
        <div
          v-if="layerColors.length === 0"
          class="
            text-sm text-slate-500
            dark:text-slate-400
          "
        >
          Loading layers...
        </div>
      </div>
    </div>
  </div>
</template>
