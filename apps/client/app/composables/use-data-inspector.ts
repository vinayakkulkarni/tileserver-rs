/**
 * Data Inspector Composable
 *
 * Provides map options and handlers for the data inspector page.
 * Uses VMap from @geoql/v-maplibre with maplibre-gl-inspect.
 */

import type { Map, MapOptions } from 'maplibre-gl';

import type { LayerColor } from '~/types/data';
import { fetchDataSource } from '~/utils/api/data';

export function useDataInspector(dataId: Ref<string>) {
  const layerColors = ref<LayerColor[]>([]);
  const panelOpen = ref(true);
  const mapRef = shallowRef<Map | null>(null);

  function navigateBack() {
    navigateTo({ path: '/', hash: '' }, { replace: true });
  }

  function togglePanel() {
    panelOpen.value = !panelOpen.value;
  }

  function toggleLayerVisibility(layerId: string) {
    const map = mapRef.value;
    if (!map) return;

    const layer = layerColors.value.find((l) => l.id === layerId);
    if (!layer) return;

    const newVisibility = !layer.visible;
    layer.visible = newVisibility;

    const allLayers = map.getStyle()?.layers || [];
    const layersMatchingSourceLayer = allLayers.filter(
      (l) => 'source-layer' in l && l['source-layer'] === layerId,
    );
    for (const mapLayer of layersMatchingSourceLayer) {
      map.setLayoutProperty(
        mapLayer.id,
        'visibility',
        newVisibility ? 'visible' : 'none',
      );
    }
  }

  const mapOptions = computed<MapOptions>(() => ({
    container: 'data-inspector-map',
    hash: true,
    style: {
      version: 8,
      sources: {
        vector_layer_: {
          type: 'vector',
          url: apiUrl(`/data/${dataId.value}.json`),
        },
      },
      layers: [],
    },
  }));

  async function onMapLoaded(map: Map) {
    mapRef.value = map;

    const [maplibregl, { default: MaplibreInspect }] = await Promise.all([
      import('maplibre-gl'),
      import('maplibre-gl-inspect'),
    ]);

    const tileJson = await fetchDataSource(dataId.value);
    const vectorLayerIds = tileJson?.vector_layers?.map((l) => l.id) ?? [];

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

    layerColors.value = vectorLayerIds.map((layerId) => ({
      id: layerId,
      color: inspect.assignLayerColor(layerId, 1),
      visible: true,
    }));
  }

  return {
    mapOptions,
    layerColors,
    panelOpen,
    navigateBack,
    togglePanel,
    toggleLayerVisibility,
    onMapLoaded,
  };
}
