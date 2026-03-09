/**
 * Web worker for offloading heavy file parsing from the main thread.
 *
 * Handles: GeoJSON (via destr), CSV (via papaparse), Shapefile (via shpjs).
 * KML/GPX stay on main thread — they need DOMParser which isn't available in workers.
 */
import { destr } from 'destr';
import type { FeatureCollection, Feature, GeoJSON, Geometry } from 'geojson';
import type { GeometryType } from '~/types/file-upload';

// ---------------------------------------------------------------------------
// Worker message protocol
// ---------------------------------------------------------------------------

interface ParseRequest {
  id: string;
  format: 'geojson' | 'csv' | 'shapefile';
  fileName: string;
  data: string | ArrayBuffer;
}

interface ParseResult {
  fileName: string;
  format: string;
  data: FeatureCollection;
  featureCount: number;
  geometryTypes: GeometryType[];
}

interface ParseSuccess {
  id: string;
  success: true;
  result: ParseResult;
}

interface ParseFailure {
  id: string;
  success: false;
  error: string;
}

// ---------------------------------------------------------------------------
// Message handler
// ---------------------------------------------------------------------------

self.onmessage = async (event: MessageEvent<ParseRequest>) => {
  const { id, format, fileName, data } = event.data;

  try {
    let result: ParseResult;

    switch (format) {
      case 'geojson':
        result = parseGeoJSON(fileName, data as string);
        break;
      case 'csv':
        result = await parseCSV(fileName, data as string);
        break;
      case 'shapefile':
        result = await parseShapefile(fileName, data as ArrayBuffer);
        break;
      default:
        throw new Error(`Unsupported worker format: ${format}`);
    }

    self.postMessage({ id, success: true, result } satisfies ParseSuccess);
  } catch (err) {
    self.postMessage({
      id,
      success: false,
      error: err instanceof Error ? err.message : String(err),
    } satisfies ParseFailure);
  }
};

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

/** Parse GeoJSON using destr (safer + faster than JSON.parse) */
function parseGeoJSON(fileName: string, text: string): ParseResult {
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

/** Parse CSV with lat/lng column detection */
async function parseCSV(fileName: string, text: string): Promise<ParseResult> {
  const Papa = await import('papaparse');

  return new Promise((resolve, reject) => {
    Papa.default.parse<Record<string, string>>(text, {
      header: true,
      skipEmptyLines: true,
      complete(results) {
        if (results.errors.length > 0 && results.data.length === 0) {
          reject(new Error(`CSV parse error: ${results.errors[0]?.message}`));
          return;
        }

        const features = csvToFeatures(results.data, results.meta.fields ?? []);

        if (features.length === 0) {
          reject(
            new Error(
              'No coordinates found. CSV must have columns named lat/latitude/y and lon/longitude/lng/x.',
            ),
          );
          return;
        }

        const data: FeatureCollection = {
          type: 'FeatureCollection',
          features,
        };

        resolve({
          fileName,
          format: 'csv',
          data,
          featureCount: features.length,
          geometryTypes: ['Point'],
        });
      },
      error(err: Error) {
        reject(new Error(`CSV parse error: ${err.message}`));
      },
    });
  });
}

/** Parse Shapefile (.zip) */
async function parseShapefile(fileName: string, buffer: ArrayBuffer): Promise<ParseResult> {
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

/** Normalize any GeoJSON type to a FeatureCollection */
function normalizeToFeatureCollection(data: GeoJSON): FeatureCollection {
  if (data.type === 'FeatureCollection') return data;

  if (data.type === 'Feature') {
    return { type: 'FeatureCollection', features: [data] };
  }

  // Raw geometry → wrap in Feature → wrap in FeatureCollection
  return {
    type: 'FeatureCollection',
    features: [{ type: 'Feature', geometry: data as Geometry, properties: {} }],
  };
}

/** Analyze GeoJSON data to extract feature count and geometry types */
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

/** Map GeoJSON geometry types to our simplified GeometryType */
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

/** Known column name aliases for latitude */
const LAT_ALIASES = new Set(['lat', 'latitude', 'y', 'lat_y', 'point_y']);

/** Known column name aliases for longitude */
const LON_ALIASES = new Set(['lon', 'lng', 'longitude', 'x', 'long', 'lon_x', 'point_x']);

/** Convert CSV rows to GeoJSON Point features */
function csvToFeatures(rows: Record<string, string>[], fields: string[]): Feature[] {
  const latField = fields.find((f) => LAT_ALIASES.has(f.toLowerCase().trim()));
  const lonField = fields.find((f) => LON_ALIASES.has(f.toLowerCase().trim()));

  if (!latField || !lonField) return [];

  const features: Feature[] = [];

  for (const row of rows) {
    const lat = Number.parseFloat(row[latField] ?? '');
    const lon = Number.parseFloat(row[lonField] ?? '');

    if (Number.isNaN(lat) || Number.isNaN(lon)) continue;
    if (lat < -90 || lat > 90 || lon < -180 || lon > 180) continue;

    const properties: Record<string, string> = {};
    for (const field of fields) {
      if (field !== latField && field !== lonField) {
        properties[field] = row[field] ?? '';
      }
    }

    features.push({
      type: 'Feature',
      geometry: {
        type: 'Point',
        coordinates: [lon, lat],
      },
      properties,
    });
  }

  return features;
}
