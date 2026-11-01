<script setup lang="ts">
  import { VMap, VControlNavigation } from '@geoql/v-maplibre';
  import type { StatsHeatPoint } from '~/types';
  import { useAnalyticsHeatmap } from '~/composables/admin/use-analytics-heatmap';

  const props = defineProps<{
    points: StatsHeatPoint[];
    zoom: number;
    dropped: number;
    isLoading: boolean;
  }>();
  const { mapOptions, styleReady, onMapLoaded } = useAnalyticsHeatmap(
    toRef(props, 'points'),
  );
</script>

<template>
  <div class="px-page-x py-6 sm:py-8">
    <div class="flex items-baseline justify-between gap-4">
      <p class="font-mono text-11 tracking-180 text-muted-foreground uppercase">
        Request heatmap
      </p>
      <p class="font-mono text-11 text-muted-foreground">
        cells at z{{ zoom
        }}<template v-if="dropped > 0"> · {{ dropped }} not shown</template>
      </p>
    </div>
    <div class="relative mt-4 h-80 border border-border sm:h-96">
      <Skeleton v-if="isLoading || !styleReady" class="absolute inset-0" />
      <ClientOnly v-else>
        <VMap :options="mapOptions" class="size-full" @loaded="onMapLoaded">
          <VControlNavigation position="bottom-right" />
        </VMap>
      </ClientOnly>
    </div>
  </div>
</template>
