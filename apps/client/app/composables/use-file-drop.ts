import type { Map, LngLatBoundsLike, LayerSpecification } from 'maplibre-gl';
import type { FeatureCollection, GeoJSON } from 'geojson';
import type { ShallowRef } from 'vue';
import type {
  FileDropError,
  FileDropSuccess,
  FileDropStatus,
  OverlayLayer,
} from '~/types/file-upload';
import type { Data } from '~/types/data';
import { SERVER_SIDE_FORMATS } from '~/types/file-upload';
import { validateFile, parseFile } from '~/lib/file-parsers';
import { createOverlayConfig, nextOverlayColor, resetOverlayColors } from '~/lib/auto-style';
import { useUploadFileMutation } from '~/utils/api/upload/use-upload-file.mutation';
import { useDeleteUploadMutation } from '~/utils/api/upload/use-delete-upload.mutation';

/**
 * Composable for drag-and-drop geospatial file visualization.
 *
 * Manages the full lifecycle: drag events → file parsing → map layer creation → layer management.
 * Uses VueUse's useDropZone for drag-and-drop handling.
 *
 * - Client-side formats (GeoJSON, KML, GPX, CSV, Shapefile, PMTiles) are parsed in-browser.
 * - Server-side formats (MBTiles, SQLite, COG) are streamed to the Rust backend via useMutation.
 */
export function useFileDrop(mapRef: ShallowRef<Map | null>) {
  const dropZoneRef = ref<HTMLElement | null>(null);
  const status = ref<FileDropStatus>('idle');
  const overlays = ref<OverlayLayer[]>([]);
  const lastError = ref<FileDropError | null>(null);
  const lastSuccess = ref<FileDropSuccess | null>(null);
  let successTimer: ReturnType<typeof setTimeout> | null = null;

  // TanStack mutations must be called at setup scope (not inside async handlers)
  const uploadMutation = useUploadFileMutation();
  const deleteMutation = useDeleteUploadMutation();

  const { isOverDropZone } = useDropZone(dropZoneRef, {
    onDrop: handleDrop,
    onEnter: () => {
      status.value = 'dragging';
    },
    onLeave: () => {
      if (status.value === 'dragging') {
        status.value = 'idle';
      }
    },
  });

  /** Whether the overlay panel should be visible */
  const hasOverlays = computed(() => overlays.value.length > 0);

  /** Process dropped files */
  async function handleDrop(files: File[] | null) {
    if (!files || files.length === 0) {
      status.value = 'idle';
      return;
    }

    lastError.value = null;

    for (const file of files) {
      try {
        await processFile(file);
      }
      catch (err) {
        const message = err instanceof Error ? err.message : 'Unknown error';
        lastError.value = { fileName: file.name, message };
        console.warn(`[file-drop] Failed to process ${file.name}:`, message);
      }
    }

    status.value = 'idle';
  }

  /** Process a single file: validate → parse/upload → add to map */
  async function processFile(file: File) {
    const { format } = validateFile(file);

    if (SERVER_SIDE_FORMATS.has(format)) {
      await processServerSide(file, format);
    }
    else {
      status.value = 'processing';
      const parsed = await parseFile(file, format);
      const color = nextOverlayColor();
      const config = createOverlayConfig(parsed, color);
      addClientOverlay(config, parsed, color);
      notifySuccess(parsed.fileName, parsed.featureCount, parsed.format);
    }
  }

  /** Upload file to server and add returned tile source to the map */
  async function processServerSide(file: File, format: string) {
    const map = mapRef.value;
    if (!map) {
      throw new Error('Map not ready. Please wait for the map to finish loading.');
    }

    status.value = 'uploading';

    const response = await uploadMutation.mutateAsync(file);

    // Fetch the TileJSON to get bounds, vector_layers, etc.
    const tileJson = await $fetch<Data>(`/data/${response.source_id}.json`);

    const color = nextOverlayColor();
    const sourceId = `overlay-${response.source_id}`;

    // Add tile source to map
    if (format === 'cog') {
      // COG = raster source
      map.addSource(sourceId, {
        type: 'raster',
        url: `/data/${response.source_id}.json`,
        tileSize: 256,
      });

      const layerId = `${sourceId}-raster`;
      map.addLayer({
        id: layerId,
        type: 'raster',
        source: sourceId,
      });

      registerOverlay({
        sourceId,
        layerIds: [layerId],
        fileName: file.name,
        format: format as OverlayLayer['format'],
        featureCount: 0,
        color,
        uploadId: response.id,
        bounds: tileJson?.bounds,
      });
      notifySuccess(file.name, 0, format as FileDropSuccess['format']);
    }
    else {
      // MBTiles / SQLite = vector source
      map.addSource(sourceId, {
        type: 'vector',
        url: `/data/${response.source_id}.json`,
      });

      const layerIds = addVectorLayersFromTileJSON(map, sourceId, tileJson, color);
      const featureCount = tileJson?.vector_layers?.length ?? 0;

      registerOverlay({
        sourceId,
        layerIds,
        fileName: file.name,
        format: format as OverlayLayer['format'],
        featureCount,
        color,
        uploadId: response.id,
        bounds: tileJson?.bounds,
      });
      notifySuccess(file.name, featureCount, format as FileDropSuccess['format']);
    }
  }

  /** Create auto-styled layers from TileJSON vector_layers metadata */
  function addVectorLayersFromTileJSON(
    map: Map,
    sourceId: string,
    tileJson: Data | null,
    color: string,
  ): string[] {
    const layerIds: string[] = [];
    const vectorLayers = tileJson?.vector_layers ?? [];

    if (vectorLayers.length === 0) {
      // No vector_layers metadata — add a generic catch-all layer
      const layerId = `${sourceId}-generic`;
      map.addLayer({
        id: layerId,
        type: 'circle',
        source: sourceId,
        paint: {
          'circle-radius': 5,
          'circle-color': color,
          'circle-opacity': 0.8,
          'circle-stroke-width': 1.5,
          'circle-stroke-color': '#ffffff',
        },
      } as LayerSpecification);
      layerIds.push(layerId);
      return layerIds;
    }

    for (const vl of vectorLayers) {
      // Add fill + outline for each source-layer (safe default for mixed geometry)
      const fillId = `${sourceId}-${vl.id}-fill`;
      map.addLayer({
        id: fillId,
        type: 'fill',
        source: sourceId,
        'source-layer': vl.id,
        paint: {
          'fill-color': color,
          'fill-opacity': 0.2,
        },
      } as LayerSpecification);
      layerIds.push(fillId);

      const lineId = `${sourceId}-${vl.id}-line`;
      map.addLayer({
        id: lineId,
        type: 'line',
        source: sourceId,
        'source-layer': vl.id,
        paint: {
          'line-color': color,
          'line-width': 2,
          'line-opacity': 0.8,
        },
      } as LayerSpecification);
      layerIds.push(lineId);

      const circleId = `${sourceId}-${vl.id}-circle`;
      map.addLayer({
        id: circleId,
        type: 'circle',
        source: sourceId,
        'source-layer': vl.id,
        filter: ['any', ['==', ['geometry-type'], 'Point'], ['==', ['geometry-type'], 'MultiPoint']],
        paint: {
          'circle-radius': 5,
          'circle-color': color,
          'circle-opacity': 0.8,
          'circle-stroke-width': 1.5,
          'circle-stroke-color': '#ffffff',
        },
      } as LayerSpecification);
      layerIds.push(circleId);
    }

    return layerIds;
  }

  /** Register a new overlay and auto-zoom if bounds are available */
  function registerOverlay(opts: {
    sourceId: string;
    layerIds: string[];
    fileName: string;
    format: OverlayLayer['format'];
    featureCount: number;
    color: string;
    uploadId?: string;
    bounds?: number[];
  }) {
    const overlay: OverlayLayer = {
      id: opts.sourceId,
      fileName: opts.fileName,
      format: opts.format,
      featureCount: opts.featureCount,
      color: opts.color,
      visible: true,
      sourceId: opts.sourceId,
      layerIds: opts.layerIds,
      uploadId: opts.uploadId,
    };

    overlays.value = [...overlays.value, overlay];

    // Auto-zoom to bounds from TileJSON
    const map = mapRef.value;
    if (map && opts.bounds && opts.bounds.length >= 4) {
      map.fitBounds(
        [opts.bounds[0]!, opts.bounds[1]!, opts.bounds[2]!, opts.bounds[3]!] as LngLatBoundsLike,
        { padding: 50, maxZoom: 15, duration: 1000 },
      );
    }
  }

  /** Add a parsed client-side overlay to the MapLibre map */
  function addClientOverlay(
    config: ReturnType<typeof createOverlayConfig>,
    parsed: ReturnType<typeof parseFile> extends Promise<infer T> ? T : never,
    color: string,
  ) {
    const map = mapRef.value;
    if (!map) {
      throw new Error('Map not ready. Please wait for the map to finish loading.');
    }

    // Add source
    map.addSource(config.sourceId, config.source);

    // Add layers
    const layerIds: string[] = [];
    for (const layer of config.layers) {
      map.addLayer(layer);
      layerIds.push(layer.id);
    }

    // Register overlay
    const overlay: OverlayLayer = {
      id: config.sourceId,
      fileName: parsed.fileName,
      format: parsed.format,
      featureCount: parsed.featureCount,
      color,
      visible: true,
      sourceId: config.sourceId,
      layerIds,
    };

    overlays.value = [...overlays.value, overlay];

    // Auto-zoom to data extent
    if (parsed.data) {
      zoomToGeoJSON(map, parsed.data);
    }
  }

  /** Toggle visibility of an overlay layer */
  function toggleOverlay(overlayId: string) {
    const map = mapRef.value;
    if (!map) return;

    const overlay = overlays.value.find((o) => o.id === overlayId);
    if (!overlay) return;

    const newVisible = !overlay.visible;
    const visibility = newVisible ? 'visible' : 'none';

    for (const layerId of overlay.layerIds) {
      map.setLayoutProperty(layerId, 'visibility', visibility);
    }

    overlays.value = overlays.value.map((o) =>
      o.id === overlayId ? { ...o, visible: newVisible } : o,
    );
  }

  /** Remove an overlay layer from the map */
  function removeOverlay(overlayId: string) {
    const map = mapRef.value;
    if (!map) return;

    const overlay = overlays.value.find((o) => o.id === overlayId);
    if (!overlay) return;

    // Remove layers first, then source
    for (const layerId of overlay.layerIds) {
      if (map.getLayer(layerId)) {
        map.removeLayer(layerId);
      }
    }

    if (map.getSource(overlay.sourceId)) {
      map.removeSource(overlay.sourceId);
    }

    // Clean up server-side upload
    if (overlay.uploadId) {
      deleteMutation.mutate(overlay.uploadId);
    }

    overlays.value = overlays.value.filter((o) => o.id !== overlayId);

    // Reset color index if all overlays removed
    if (overlays.value.length === 0) {
      resetOverlayColors();
    }
  }

  /** Remove all overlay layers */
  function removeAllOverlays() {
    const ids = overlays.value.map((o) => o.id);
    for (const id of ids) {
      removeOverlay(id);
    }
  }

  /** Zoom the map to fit GeoJSON data bounds */
  function zoomToGeoJSON(map: Map, data: GeoJSON) {
    const bounds = computeBounds(data);
    if (!bounds) return;

    map.fitBounds(bounds as LngLatBoundsLike, {
      padding: 50,
      maxZoom: 15,
      duration: 1000,
    });
  }

  /** Show a success notification that auto-dismisses after 4 seconds */
  function notifySuccess(fileName: string, featureCount: number, format: FileDropSuccess['format']) {
    if (successTimer) clearTimeout(successTimer);
    lastSuccess.value = { fileName, featureCount, format };
    successTimer = setTimeout(() => {
      lastSuccess.value = null;
    }, 4000);
  }


  return {
    dropZoneRef,
    status,
    overlays,
    lastError,
    lastSuccess,
    isOverDropZone,
    hasOverlays,
    toggleOverlay,
    removeOverlay,
    removeAllOverlays,
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Compute bounding box from GeoJSON data */
function computeBounds(data: GeoJSON): [number, number, number, number] | null {
  let minLng = Infinity;
  let minLat = Infinity;
  let maxLng = -Infinity;
  let maxLat = -Infinity;
  let hasCoords = false;

  function processCoord(coord: number[]) {
    const [lng, lat] = coord;
    if (lng === undefined || lat === undefined) return;
    if (Number.isNaN(lng) || Number.isNaN(lat)) return;

    minLng = Math.min(minLng, lng);
    minLat = Math.min(minLat, lat);
    maxLng = Math.max(maxLng, lng);
    maxLat = Math.max(maxLat, lat);
    hasCoords = true;
  }

  function processCoords(coords: number[] | number[][] | number[][][] | number[][][][]) {
    if (typeof coords[0] === 'number') {
      processCoord(coords as number[]);
    }
    else {
      for (const c of coords as (number[] | number[][] | number[][][] )[]) {
        processCoords(c);
      }
    }
  }

  function processGeometry(geom: GeoJSON) {
    if ('coordinates' in geom) {
      processCoords(geom.coordinates as number[]);
    }
    if ('geometries' in geom) {
      for (const g of geom.geometries as GeoJSON[]) {
        processGeometry(g);
      }
    }
    if ('features' in geom) {
      for (const f of (geom as FeatureCollection).features) {
        if (f.geometry) processGeometry(f.geometry);
      }
    }
    if (geom.type === 'Feature' && 'geometry' in geom && geom.geometry) {
      processGeometry(geom.geometry);
    }
  }

  processGeometry(data);

  if (!hasCoords) return null;
  return [minLng, minLat, maxLng, maxLat];
}
