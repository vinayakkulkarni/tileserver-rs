import type { FeatureCollection, GeoJSON, Geometry, Feature } from 'geojson';
import type { GeometryType, ParsedFile, SupportedFormat } from '~/types/file-upload';
import { FORMAT_EXTENSIONS, CLIENT_SIDE_FORMATS, MAX_FILE_SIZE_BYTES } from '~/types/file-upload';

/**
 * Detect the geospatial format of a file from its extension.
 * Returns undefined for unsupported formats.
 */
export function detectFormat(fileName: string): SupportedFormat | undefined {
  const ext = fileName.slice(fileName.lastIndexOf('.')).toLowerCase();
  return FORMAT_EXTENSIONS[ext];
}

/**
 * Check if a file can be processed client-side.
 */
export function isClientSideFormat(format: SupportedFormat): boolean {
  return CLIENT_SIDE_FORMATS.has(format);
}

/**
 * Validate a file before processing.
 * Throws an error with a user-friendly message if invalid.
 */
export function validateFile(file: File): { format: SupportedFormat } {
  const format = detectFormat(file.name);

  if (!format) {
    const ext = file.name.slice(file.name.lastIndexOf('.'));
    throw new Error(
      `Unsupported format "${ext}". Supported: GeoJSON, KML, GPX, CSV, Shapefile (.zip), PMTiles, MBTiles, COG (.tif).`,
    );
  }

  if (isClientSideFormat(format) && file.size > MAX_FILE_SIZE_BYTES) {
    const sizeMB = Math.round(file.size / 1024 / 1024);
    throw new Error(`File too large (${sizeMB} MB). Maximum for client-side processing is 50 MB.`);
  }

  return { format };
}

/**
 * Parse a dropped file into a ParsedFile result.
 * Only handles client-side formats. Server-side formats (MBTiles, SQLite, COG)
 * should be uploaded to the backend.
 */
export async function parseFile(file: File, format: SupportedFormat): Promise<ParsedFile> {
  switch (format) {
    case 'geojson':
      return parseGeoJSON(file);
    case 'kml':
      return parseKML(file);
    case 'gpx':
      return parseGPX(file);
    case 'csv':
      return parseCSV(file);
    case 'shapefile':
      return parseShapefile(file);
    case 'pmtiles':
      return parsePMTiles(file);
    default:
      throw new Error(`Format "${format}" requires server-side processing.`);
  }
}

/** Parse a GeoJSON file */
async function parseGeoJSON(file: File): Promise<ParsedFile> {
  const text = await file.text();
  const data = JSON.parse(text) as GeoJSON;
  const { featureCount, geometryTypes } = analyzeGeoJSON(data);

  return {
    fileName: file.name,
    format: 'geojson',
    data,
    featureCount,
    geometryTypes,
  };
}

/** Parse a KML file using @tmcw/togeojson */
async function parseKML(file: File): Promise<ParsedFile> {
  const { kml } = await import('@tmcw/togeojson');
  const text = await file.text();
  const dom = new DOMParser().parseFromString(text, 'application/xml');

  const parserError = dom.querySelector('parsererror');
  if (parserError) {
    throw new Error(`Invalid KML file: ${parserError.textContent?.slice(0, 100)}`);
  }

  const data = kml(dom) as FeatureCollection;
  const { featureCount, geometryTypes } = analyzeGeoJSON(data);

  return {
    fileName: file.name,
    format: 'kml',
    data,
    featureCount,
    geometryTypes,
  };
}

/** Parse a GPX file using @tmcw/togeojson */
async function parseGPX(file: File): Promise<ParsedFile> {
  const { gpx } = await import('@tmcw/togeojson');
  const text = await file.text();
  const dom = new DOMParser().parseFromString(text, 'application/xml');

  const parserError = dom.querySelector('parsererror');
  if (parserError) {
    throw new Error(`Invalid GPX file: ${parserError.textContent?.slice(0, 100)}`);
  }

  const data = gpx(dom) as FeatureCollection;
  const { featureCount, geometryTypes } = analyzeGeoJSON(data);

  return {
    fileName: file.name,
    format: 'gpx',
    data,
    featureCount,
    geometryTypes,
  };
}

/** Parse a CSV file using papaparse — expects lat/lon columns */
async function parseCSV(file: File): Promise<ParsedFile> {
  const Papa = await import('papaparse');
  const text = await file.text();

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
          fileName: file.name,
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

/** Parse a Shapefile (.zip) using shpjs */
async function parseShapefile(file: File): Promise<ParsedFile> {
  const shp = await import('shpjs');
  const buffer = await file.arrayBuffer();
  const result = await shp.default(buffer);

  // shpjs can return a single FeatureCollection or an array of them
  const data: FeatureCollection = Array.isArray(result)
    ? {
        type: 'FeatureCollection',
        features: result.flatMap((fc) => fc.features),
      }
    : result;

  const { featureCount, geometryTypes } = analyzeGeoJSON(data);

  return {
    fileName: file.name,
    format: 'shapefile',
    data,
    featureCount,
    geometryTypes,
  };
}

/** Handle PMTiles — create an object URL for MapLibre's pmtiles protocol */
async function parsePMTiles(file: File): Promise<ParsedFile> {
  const objectUrl = URL.createObjectURL(file);

  return {
    fileName: file.name,
    format: 'pmtiles',
    objectUrl,
    featureCount: 0, // Unknown until tiles are loaded
    geometryTypes: [],
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

  // Raw geometry
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
function csvToFeatures(
  rows: Record<string, string>[],
  fields: string[],
): Feature[] {
  const latField = fields.find((f) => LAT_ALIASES.has(f.toLowerCase().trim()));
  const lonField = fields.find((f) => LON_ALIASES.has(f.toLowerCase().trim()));

  if (!latField || !lonField) return [];

  const features: Feature[] = [];

  for (const row of rows) {
    const lat = Number.parseFloat(row[latField] ?? '');
    const lon = Number.parseFloat(row[lonField] ?? '');

    if (Number.isNaN(lat) || Number.isNaN(lon)) continue;
    if (lat < -90 || lat > 90 || lon < -180 || lon > 180) continue;

    // All non-coordinate fields become properties
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
