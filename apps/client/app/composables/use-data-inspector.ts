/**
 * Data Inspector Composable
 *
 * Provides map options for data inspection with VMap component.
 * Adds the inspect control when the map is loaded.
 */

import type { Map, MapOptions, StyleSpecification } from 'maplibre-gl';

export function useDataInspector(dataId: Ref<string>) {
  // Generate unique container ID for each instance
  const containerId = `map-data-${Math.random().toString(36).substring(2, 11)}`;

  const style = computed<StyleSpecification>(() => ({
    version: 8,
    sources: {
      'vector-source': {
        type: 'vector',
        url: `/data/${dataId.value}.json`,
      },
    },
    layers: [],
  }));

  // VMap requires full MapOptions with container
  const mapOptions = computed<MapOptions>(() => ({
    container: containerId,
    style: style.value,
    center: [0, 0],
    zoom: 1,
    hash: true,
  }));

  async function onMapLoaded(map: Map) {
    // Dynamically import maplibre-gl-inspect to avoid SSR issues
    const { default: MaplibreInspect } = await import('maplibre-gl-inspect');

    map.addControl(
      new MaplibreInspect({
        showInspectMap: true,
        showInspectButton: false,
      }),
      'top-right',
    );
  }

  return { mapOptions, onMapLoaded };
}
