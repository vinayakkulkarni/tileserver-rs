import { StyleSpecification } from 'maplibre-gl';
import { TileJSON } from '~/types/style';

const useMapStyle = async () => {
  const route = useRoute();
  const isRaster = computed(() => 'raster' in route.query);
  const style = ref<StyleSpecification>({
    version: 8,
    metadata: {},
    sources: {},
    glyphs: 'mapbox://fonts/mapbox/{fontstack}/{range}.pbf',
    layers: [
      {
        id: 'land',
        type: 'background',
        layout: {},
        paint: {
          'background-color': 'transparent',
        },
      },
    ],
  });

  if (isRaster.value) {
    const { data } = await useFetch(`/styles/${route.params.style}.json`);
    const tileJSON = data.value as TileJSON;
    style.value = {
      version: 8,
      sources: {
        'raster-tiles': {
          type: 'raster',
          tiles: tileJSON.tiles,
          tileSize: 256,
          attribution: tileJSON.attribution,
        },
      },
      layers: [
        {
          id: 'simple-tiles',
          type: 'raster',
          source: 'raster-tiles',
          minzoom: tileJSON.minzoom,
          maxzoom: tileJSON.maxzoom,
        },
      ],
    } as StyleSpecification;
  } else {
    const { data } = await useFetch(`/styles/${route.params.style}/style.json`);
    style.value = data.value as StyleSpecification;
  }

  return { style };
}

export { useMapStyle };
