/**
 * Style Viewer Composable
 *
 * Provides map options for viewing styled maps with VMap component.
 */

import type { Map, MapOptions, StyleSpecification } from 'maplibre-gl';
import { resolveInitialView, VIEWER_MAX_PITCH } from '~/lib/initial-view';
import { ensureMaplibreWorker } from '~/utils/maplibre-worker';
import { useLiveUpdates } from './use-live-updates';
import { useMapInteractionTracking } from './use-map-interaction-tracking';

export function useStyleViewer(styleId: Ref<string>, isRaster: Ref<boolean>) {
  ensureMaplibreWorker();
  const { style, isLoading } = useMapStyle(styleId, isRaster);
  const { watchMap } = useMapInteractionTracking('style');

  // Map instance ref for LLM tool integration
  const mapRef = shallowRef<Map | null>(null);
  useLiveUpdates(mapRef);

  // Generate unique container ID for each instance
  const containerId = `map-${Math.random().toString(36).substring(2, 11)}`;

  // IMPORTANT: Use toRaw to unwrap reactive style object for MapLibre
  // Only provide mapOptions when style is loaded
  const mapOptions = computed<MapOptions | null>(() => {
    if (!style.value) return null;

    const rawStyle = toRaw(style.value) as StyleSpecification;
    // URL hash, then the style's own camera, then the world view.
    const view = resolveInitialView(window.location.hash, rawStyle);

    return {
      container: containerId,
      style: rawStyle,
      ...view,
      maxPitch: VIEWER_MAX_PITCH,
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
