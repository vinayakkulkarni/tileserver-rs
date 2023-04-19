<template>
  <v-header />
  <div ref="mapRef" id="map" class="w-full h-full" />
  <v-footer />
</template>

<script lang="ts">
  import type { Map } from 'maplibre-gl';
  import { MaplibreInspect } from 'maplibre-gl-inspect';
  import maplibregl from 'maplibre-gl';

  export default defineComponent({
    name: 'MapData',
    setup() {
      const route = useRoute();
      const mapRef = ref(null);
      let map = markRaw({} as Map);

      onMounted(async () => {
        map = new maplibregl.Map({
          container: mapRef.value || 'map',
          style: {
            version: 8,
            sources: {
              openmaptiles: {
                type: 'vector',
                url: `/data/${route.params.data}.json`,
              },
            },
            layers: [],
          },
          center: [0, 0],
          zoom: 1,
          hash: true,
        });
        addControls();
      });

      /**
       * Add Scale, Geolocate & Navigate controls to the map
       *
       * @returns {void}
       */
      const addControls = (): void => {
        const scale = new maplibregl.ScaleControl({
          maxWidth: 80,
        });
        const nav = new maplibregl.NavigationControl({
          showCompass: true,
          showZoom: true,
          visualizePitch: true,
        });
        const geolocate = new maplibregl.GeolocateControl({
          positionOptions: {
            enableHighAccuracy: true,
          },
          trackUserLocation: true,
        });
        const inspect = new MaplibreInspect({
          showInspectMap: true,
          showInspectButton: false,
          useInspectStyle: true,
        });

        map.addControl(scale, 'bottom-left');
        map.addControl(geolocate, 'bottom-right');
        map.addControl(nav, 'bottom-right');
        map.addControl(inspect, 'top-right');
      };

      return {
        mapRef,
      };
    },
  });
</script>
