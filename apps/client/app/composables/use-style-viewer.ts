/**
 * Style Viewer Composable
 *
 * Provides map options for viewing styled maps with VMap component.
 */

import type { MapOptions, StyleSpecification } from 'maplibre-gl';

export function useStyleViewer(
  styleId: Ref<string>,
  isRaster: Ref<boolean>,
) {
  const { style, isLoading } = useMapStyle(styleId, isRaster);

  // Generate unique container ID for each instance
  const containerId = `map-style-${Math.random().toString(36).substring(2, 11)}`;

  // VMap requires full MapOptions with container
  const mapOptions = computed<MapOptions>(() => ({
    container: containerId,
    style: style.value as StyleSpecification,
    center: [0, 0],
    zoom: 1,
    hash: true,
  }));

  return { mapOptions, isLoading };
}
