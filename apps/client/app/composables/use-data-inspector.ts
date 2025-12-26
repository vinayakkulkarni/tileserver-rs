/**
 * Data Inspector Composable
 *
 * Provides map options and handlers for the data inspector page.
 * Uses VMap from @geoql/v-maplibre with maplibre-gl-inspect.
 */

import type { Map, MapOptions } from 'maplibre-gl';

import type { Data, LayerColor } from '~/types/data';

export function useDataInspector(dataId: Ref<string>) {
  const layerColors = ref<LayerColor[]>([]);

  // Map options for VMap - container ID is required by VMap
  const mapOptions = computed<MapOptions>(() => ({
    container: 'data-inspector-map',
    hash: true,
    style: {
      version: 8,
      sources: {
        vector_layer_: {
          type: 'vector',
          url: `/data/${dataId.value}.json`,
        },
      },
      layers: [],
    },
  }));

  // Handler for when map is loaded - adds inspect control
  async function onMapLoaded(map: Map) {
    const [maplibregl, { default: MaplibreInspect }] = await Promise.all([
      import('maplibre-gl'),
      import('@maplibre/maplibre-gl-inspect'),
    ]);

    // Fetch TileJSON to get vector_layers
    const tileJson = await $fetch<Data>(`/data/${dataId.value}.json`);
    const vectorLayerIds = tileJson.vector_layers?.map((l) => l.id) || [];

    // Pre-populate sources so inspect knows about the layers
    const sources: Record<string, string[]> = {
      vector_layer_: vectorLayerIds,
    };

    const inspect = new MaplibreInspect({
      showInspectMap: true,
      showInspectButton: false,
      sources,
      popup: new maplibregl.Popup({
        closeButton: false,
        closeOnClick: false,
      }),
    });

    map.addControl(inspect);
    inspect.render();

    // Build layer colors
    layerColors.value = vectorLayerIds.map((layerId) => ({
      id: layerId,
      color: inspect.assignLayerColor(layerId, 1),
    }));
  }

  return {
    mapOptions,
    layerColors,
    onMapLoaded,
  };
}
