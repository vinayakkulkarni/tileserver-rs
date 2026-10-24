/**
 * Style Viewer Composable
 *
 * Provides map options for viewing styled maps with VMap component.
 */

import type { Map, MapOptions, StyleSpecification } from 'maplibre-gl';
import { ensureMaplibreWorker } from '~/utils/maplibre-worker';
import { useMapInteractionTracking } from './use-map-interaction-tracking';

export function useStyleViewer(styleId: Ref<string>, isRaster: Ref<boolean>) {
  ensureMaplibreWorker();
  const { style, isLoading } = useMapStyle(styleId, isRaster);
  const { watchMap } = useMapInteractionTracking('style');

  // Map instance ref for LLM tool integration
  const mapRef = shallowRef<Map | null>(null);

  // Generate unique container ID for each instance
  const containerId = `map-${Math.random().toString(36).substring(2, 11)}`;

  // IMPORTANT: Use toRaw to unwrap reactive style object for MapLibre
  // Only provide mapOptions when style is loaded
  const mapOptions = computed<MapOptions | null>(() => {
    if (!style.value) return null;

    return {
      container: containerId,
      style: toRaw(style.value) as StyleSpecification,
      center: [0, 0] as [number, number],
      zoom: 1,
      hash: true,
      interactive: true,
    };
  });

  function onMapLoaded(map: Map) {
    mapRef.value = map;
    watchMap(map);
  }

  function navigateBack() {
    navigateTo({ path: '/', hash: '' }, { replace: true });
  }

  return { mapOptions, mapRef, isLoading, navigateBack, onMapLoaded };
}
