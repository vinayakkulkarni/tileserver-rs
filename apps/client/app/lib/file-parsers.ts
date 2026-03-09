import type { FeatureCollection, GeoJSON, Geometry } from 'geojson';
import type { GeometryType, ParsedFile, SupportedFormat } from '~/types/file-upload';
import { FORMAT_EXTENSIONS, CLIENT_SIDE_FORMATS, MAX_FILE_SIZE_BYTES } from '~/types/file-upload';


// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

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
 *
 * Heavy formats (GeoJSON, CSV, Shapefile) are offloaded to web workers
 * via nuxt-workers — auto-imported, zero config, SSR-safe.
 *
 * KML/GPX stay on main thread — they need DOMParser (unavailable in workers).
 * PMTiles stays on main thread — creates an object URL for MapLibre.
 */
export async function parseFile(file: File, format: SupportedFormat): Promise<ParsedFile> {
  switch (format) {
    // Worker-offloaded formats (nuxt-workers auto-imports)
    case 'geojson': {
      const text = await file.text();
      return parseGeoJSON(file.name, text);
    }
    case 'csv': {
      const text = await file.text();
      return parseCSV(file.name, text);
    }
    case 'shapefile': {
      const buffer = await file.arrayBuffer();
      return parseShapefile(file.name, buffer);
    }
    // Main-thread parsing (require browser APIs not available in workers)
    case 'kml':
      return parseKML(file);
    case 'gpx':
      return parseGPX(file);
    case 'pmtiles':
      return parsePMTiles(file);
    default:
      throw new Error(`Format "${format}" requires server-side processing.`);
  }
}

// ---------------------------------------------------------------------------
// Main-thread parsers (require browser APIs not available in workers)
// ---------------------------------------------------------------------------

/** Parse a KML file using @tmcw/togeojson (requires DOMParser) */
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

/** Parse a GPX file using @tmcw/togeojson (requires DOMParser) */
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
// Helpers (needed on main thread for KML/GPX analysis)
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
