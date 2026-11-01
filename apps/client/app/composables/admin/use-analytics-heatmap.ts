import type {
  GeoJSONSource,
  Map,
  MapOptions,
  StyleSpecification,
} from 'maplibre-gl';
import type { StatsHeatPoint } from '~/types';
import { fetchStyles } from '~/utils/api/styles/queries';
import { ensureMaplibreWorker } from '~/utils/maplibre-worker';
import { heatmapGeoJSON } from '~/utils/heatmap-geojson';
import { tokenRgba } from '~/utils/token-color';

const SOURCE_ID = 'tile-requests';
const LAYER_ID = 'tile-requests-heat';

const EMPTY_STYLE: StyleSpecification = { version: 8, sources: {}, layers: [] };

/**
 * MapLibre heatmap of tile requests over the server's first style (or a
 * blank map when the server has no style).
 */
export function useAnalyticsHeatmap(points: Ref<readonly StatsHeatPoint[]>) {
  ensureMaplibreWorker();
  const mapRef = shallowRef<Map | null>(null);
  const styleUrl = ref<string | null>(null);
  const styleReady = ref(false);

  onMounted(async () => {
    const styles = await fetchStyles().catch(() => []);
    const first = styles[0];
    styleUrl.value = first ? apiUrl(`/styles/${first.id}/style.json`) : null;
    styleReady.value = true;
  });

  const mapOptions = computed<MapOptions>(() => ({
    container: 'analytics-heatmap-map',
    style: styleUrl.value ?? EMPTY_STYLE,
    center: [0, 20],
    zoom: 1,
    attributionControl: { compact: true },
  }));

  function maxWeight(): number {
    return Math.max(1, ...points.value.map((p) => p.requests));
  }

  function sync(): void {
    const map = mapRef.value;
    const source = map?.getSource<GeoJSONSource>(SOURCE_ID);
    if (!map || !source) return;
    source.setData(heatmapGeoJSON(points.value));
    map.setPaintProperty(LAYER_ID, 'heatmap-weight', [
      'interpolate',
      ['linear'],
      ['get', 'requests'],
      0,
      0,
      maxWeight(),
      1,
    ]);
  }

  function onMapLoaded(map: Map): void {
    mapRef.value = map;
    map.addSource(SOURCE_ID, {
      type: 'geojson',
      data: heatmapGeoJSON(points.value),
    });
    map.addLayer({
      id: LAYER_ID,
      type: 'heatmap',
      source: SOURCE_ID,
      paint: {
        'heatmap-weight': [
          'interpolate',
          ['linear'],
          ['get', 'requests'],
          0,
          0,
          maxWeight(),
          1,
        ],
        'heatmap-radius': [
          'interpolate',
          ['linear'],
          ['zoom'],
          0,
          24,
          6,
          40,
          12,
          60,
        ],
        'heatmap-intensity': [
          'interpolate',
          ['linear'],
          ['zoom'],
          0,
          1.5,
          12,
          3,
        ],
        'heatmap-opacity': 0.9,
        'heatmap-color': [
          'interpolate',
          ['linear'],
          ['heatmap-density'],
          0,
          tokenRgba('--color-primary', 0),
          0.15,
          tokenRgba('--color-primary', 0.7),
          0.6,
          tokenRgba('--color-success', 0.9),
          1,
          tokenRgba('--color-destructive', 1),
        ],
      },
    });
    fitOnce();
  }

  let fitted = false;

  /** Frame the requested areas once, when the first points arrive. */
  function fitOnce(): void {
    const map = mapRef.value;
    if (!map || fitted || points.value.length === 0) return;
    const lons = points.value.map((p) => p.lon);
    const lats = points.value.map((p) => p.lat);
    map.fitBounds(
      [
        [Math.min(...lons), Math.min(...lats)],
        [Math.max(...lons), Math.max(...lats)],
      ],
      { padding: 60, maxZoom: 9, duration: 0 },
    );
    fitted = true;
  }

  watch(points, () => {
    sync();
    fitOnce();
  });

  return { mapOptions, styleReady, onMapLoaded };
}
