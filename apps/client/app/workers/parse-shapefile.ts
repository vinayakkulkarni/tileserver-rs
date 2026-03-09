/**
 * Parse Shapefile (.zip) using shpjs.
 * Runs in a web worker via nuxt-workers — auto-imported, zero config.
 */
import type { FeatureCollection, GeoJSON, Geometry } from 'geojson';
import type { GeometryType } from '~/types/file-upload';

interface ParseResult {
  fileName: string;
  format: 'shapefile';
  data: FeatureCollection;
  featureCount: number;
  geometryTypes: GeometryType[];
}

export async function parseShapefile(fileName: string, buffer: ArrayBuffer): Promise<ParseResult> {
  const shp = await import('shpjs');
  const result = await shp.default(buffer);

  const data: FeatureCollection = Array.isArray(result)
    ? {
        type: 'FeatureCollection',
        features: result.flatMap((fc) => fc.features),
      }
    : result;

  const { featureCount, geometryTypes } = analyzeGeoJSON(data);

  return {
    fileName,
    format: 'shapefile',
    data,
    featureCount,
    geometryTypes,
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

function addGeometryType(geometry: Geometry | null, types: Set<GeometryType>): void {
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
