/**
 * Parse GeoJSON text using destr (safer + faster than JSON.parse).
 * Runs in a Web Worker via Vite's native `new Worker(new URL(...), { type: 'module' })`.
 */
import { destr } from 'destr';
import type { FeatureCollection, GeoJSON, Geometry } from 'geojson';
import { registerWorkerHandler } from '~/lib/run-worker';
import type { GeometryType, ParseGeoJSONResult } from '~/types/file-upload';

function parseGeoJSON(fileName: string, text: string): ParseGeoJSONResult {
  const data = destr<GeoJSON>(text);

  if (!data || typeof data !== 'object' || !('type' in data)) {
    throw new Error('Invalid GeoJSON: missing "type" property');
  }

  const { featureCount, geometryTypes } = analyzeGeoJSON(data);

  return {
    fileName,
    format: 'geojson',
    data: normalizeToFeatureCollection(data),
    featureCount,
    geometryTypes,
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function normalizeToFeatureCollection(data: GeoJSON): FeatureCollection {
  if (data.type === 'FeatureCollection') return data;

  if (data.type === 'Feature') {
    return { type: 'FeatureCollection', features: [data] };
  }

  return {
    type: 'FeatureCollection',
    features: [{ type: 'Feature', geometry: data as Geometry, properties: {} }],
  };
}

function analyzeGeoJSON(data: GeoJSON): {
  featureCount: number;
  geometryTypes: GeometryType[];
} {
  const types = new Set<GeometryType>();

  if (data.type === 'FeatureCollection') {
    for (const feature of data.features) {
      addGeometryType(feature.geometry, types);
    }
    return { featureCount: data.features.length, geometryTypes: [...types] };
  }

  if (data.type === 'Feature') {
    addGeometryType(data.geometry, types);
    return { featureCount: 1, geometryTypes: [...types] };
  }

  addGeometryType(data as Geometry, types);
  return { featureCount: 1, geometryTypes: [...types] };
}

function addGeometryType(
  geometry: Geometry | null,
  types: Set<GeometryType>,
): void {
  if (!geometry) return;

  switch (geometry.type) {
    case 'Point':
    case 'MultiPoint':
      types.add('Point');
      break;
    case 'LineString':
    case 'MultiLineString':
      types.add('LineString');
      break;
    case 'Polygon':
    case 'MultiPolygon':
      types.add('Polygon');
      break;
    case 'GeometryCollection':
      for (const g of geometry.geometries) {
        addGeometryType(g, types);
      }
      break;
  }
}

registerWorkerHandler<[fileName: string, text: string], ParseGeoJSONResult>(
  parseGeoJSON,
);
