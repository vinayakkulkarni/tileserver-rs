<template>
  <v-header />
  <div ref="mapRef" id="map" class="w-full h-full relative" />
  <v-footer />
</template>

<script lang="ts">
  import maplibregl from 'maplibre-gl';
  import type { Map, StyleSpecification } from 'maplibre-gl';

  export default defineComponent({
    name: 'MapStyle',
    setup() {
      const mapRef = ref(null);
      let map = markRaw({} as Map);

      onMounted(async () => {
        const { style } = await useMapStyle();
        map = new maplibregl.Map({
          container: mapRef.value || 'map',
          // @ts-ignore
          style: style.value as StyleSpecification,
          center: [0, 0],
          zoom: 1,
          hash: true,
        });

        map.on('load', () => {
          addControls();
        });
      });

      /**
       * Add Scale, Geolocate & Navigate controls to the map
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
        map.addControl(scale, 'bottom-left');
        map.addControl(geolocate, 'bottom-right');
        map.addControl(nav, 'bottom-right');
      };

      return {
        mapRef,
      };
    },
  });
</script>
