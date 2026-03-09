import type {
  CircleLayerSpecification,
  FillLayerSpecification,
  LineLayerSpecification,
  LayerSpecification,
  SourceSpecification,
} from 'maplibre-gl';
import type { GeoJSON } from 'geojson';
import type { GeometryType, OverlaySourceConfig, ParsedFile } from '~/types/file-upload';

/**
 * Color palette for overlay layers.
 * Chosen for contrast against typical basemap backgrounds.
 */
export const OVERLAY_COLORS = [
  '#3b82f6', // blue-500
  '#ef4444', // red-500
  '#10b981', // emerald-500
  '#f59e0b', // amber-500
  '#8b5cf6', // violet-500
  '#ec4899', // pink-500
  '#06b6d4', // cyan-500
  '#f97316', // orange-500
] as const;

let colorIndex = 0;

/** Get the next color from the palette (cycles) */
export function nextOverlayColor(): string {
  const color = OVERLAY_COLORS[colorIndex % OVERLAY_COLORS.length]!;
  colorIndex++;
  return color;
}

/** Reset the color index (e.g., when all layers are removed) */
export function resetOverlayColors(): void {
  colorIndex = 0;
}

/**
 * Create MapLibre source + layer configs for a parsed file.
 * Auto-styles by geometry type: fill for polygons, line for lines, circle for points.
 */
export function createOverlayConfig(
  parsed: ParsedFile,
  color: string,
): OverlaySourceConfig {
  const sourceId = `overlay-${crypto.randomUUID().slice(0, 8)}`;

  // PMTiles: use vector source with pmtiles protocol
  if (parsed.format === 'pmtiles' && parsed.objectUrl) {
    return createPMTilesConfig(sourceId, parsed.objectUrl);
  }

  // GeoJSON-based formats
  if (!parsed.data) {
    throw new Error(`No data available for ${parsed.fileName}`);
  }

  const source: SourceSpecification = {
    type: 'geojson',
    data: parsed.data as GeoJSON,
  };

  const layers = createLayersForGeometryTypes(sourceId, parsed.geometryTypes, color);

  return { sourceId, source, layers };
}

/** Create source config for a PMTiles file */
function createPMTilesConfig(sourceId: string, objectUrl: string): OverlaySourceConfig {
  const source: SourceSpecification = {
    type: 'vector',
    url: `pmtiles://${objectUrl}`,
  };

  // PMTiles layers are added dynamically after the source loads
  // For now, create an empty layer array — the composable will handle layer discovery
  return { sourceId, source, layers: [] };
}

/** Create MapLibre layers for each geometry type with auto-styling */
function createLayersForGeometryTypes(
  sourceId: string,
  geometryTypes: GeometryType[],
  color: string,
): LayerSpecification[] {
  const layers: LayerSpecification[] = [];

  // If no geometry types detected (e.g., empty file), add all three
  const types = geometryTypes.length > 0 ? geometryTypes : (['Point', 'LineString', 'Polygon'] as GeometryType[]);

  for (const type of types) {
    switch (type) {
      case 'Polygon':
        layers.push(createFillLayer(sourceId, color));
        layers.push(createOutlineLayer(sourceId, color));
        break;
      case 'LineString':
        layers.push(createLineLayer(sourceId, color));
        break;
      case 'Point':
        layers.push(createCircleLayer(sourceId, color));
        break;
    }
  }

  return layers;
}

/** Create a fill layer for polygon features */
function createFillLayer(sourceId: string, color: string): FillLayerSpecification {
  return {
    id: `${sourceId}-fill`,
    type: 'fill',
    source: sourceId,
    filter: ['any',
      ['==', ['geometry-type'], 'Polygon'],
      ['==', ['geometry-type'], 'MultiPolygon'],
    ],
    paint: {
      'fill-color': color,
      'fill-opacity': 0.2,
    },
  };
}

/** Create an outline layer for polygon features */
function createOutlineLayer(sourceId: string, color: string): LineLayerSpecification {
  return {
    id: `${sourceId}-outline`,
    type: 'line',
    source: sourceId,
    filter: ['any',
      ['==', ['geometry-type'], 'Polygon'],
      ['==', ['geometry-type'], 'MultiPolygon'],
    ],
    paint: {
      'line-color': color,
      'line-width': 2,
      'line-opacity': 0.8,
    },
  };
}

/** Create a line layer for linestring features */
function createLineLayer(sourceId: string, color: string): LineLayerSpecification {
  return {
    id: `${sourceId}-line`,
    type: 'line',
    source: sourceId,
    filter: ['any',
      ['==', ['geometry-type'], 'LineString'],
      ['==', ['geometry-type'], 'MultiLineString'],
    ],
    paint: {
      'line-color': color,
      'line-width': 2.5,
      'line-opacity': 0.9,
    },
  };
}

/** Create a circle layer for point features */
function createCircleLayer(sourceId: string, color: string): CircleLayerSpecification {
  return {
    id: `${sourceId}-circle`,
    type: 'circle',
    source: sourceId,
    filter: ['any',
      ['==', ['geometry-type'], 'Point'],
      ['==', ['geometry-type'], 'MultiPoint'],
    ],
    paint: {
      'circle-radius': 5,
      'circle-color': color,
      'circle-opacity': 0.8,
      'circle-stroke-width': 1.5,
      'circle-stroke-color': '#ffffff',
    },
  };
}
