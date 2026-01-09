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
  const { isDark } = useThemeToggle();

  const basemapStyle = computed(() => (isDark.value ? 'dark_all' : 'light_all'));

  const mapOptions = computed<MapOptions>(() => ({
    container: 'data-inspector-map',
    hash: true,
    style: {
      version: 8,
      sources: {
        basemap: {
          type: 'raster',
          tiles: [
            `https://a.basemaps.cartocdn.com/${basemapStyle.value}/{z}/{x}/{y}@2x.png`,
            `https://b.basemaps.cartocdn.com/${basemapStyle.value}/{z}/{x}/{y}@2x.png`,
            `https://c.basemaps.cartocdn.com/${basemapStyle.value}/{z}/{x}/{y}@2x.png`,
          ],
          tileSize: 256,
          attribution:
            '&copy; <a href="https://carto.com/">CARTO</a> &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>',
        },
        vector_layer_: {
          type: 'vector',
          url: `/data/${dataId.value}.json`,
        },
      },
      layers: [
        {
          id: 'basemap',
          type: 'raster',
          source: 'basemap',
          minzoom: 0,
          maxzoom: 22,
        },
      ],
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
