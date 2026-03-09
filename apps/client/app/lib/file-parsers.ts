import type { FeatureCollection, GeoJSON, Geometry } from 'geojson';
import type { GeometryType, ParsedFile, SupportedFormat } from '~/types/file-upload';
import { FORMAT_EXTENSIONS, CLIENT_SIDE_FORMATS, MAX_FILE_SIZE_BYTES } from '~/types/file-upload';

// ---------------------------------------------------------------------------
// Worker management
// ---------------------------------------------------------------------------

/** Formats offloaded to the web worker (heavy parsing, no DOMParser needed) */
const WORKER_FORMATS = new Set<SupportedFormat>(['geojson', 'csv', 'shapefile']);

let worker: Worker | null = null;

/** Get or create the file parse worker (lazy singleton) */
function getWorker(): Worker {
  if (!worker) {
    worker = new Worker(new URL('./file-parse-worker.ts', import.meta.url), {
      type: 'module',
    });
  }
  return worker;
}

/** Send a parse request to the worker and await the result */
function parseInWorker(
  format: 'geojson' | 'csv' | 'shapefile',
  fileName: string,
  data: string | ArrayBuffer,
): Promise<ParsedFile> {
  return new Promise((resolve, reject) => {
    const id = crypto.randomUUID();
    const w = getWorker();

    function handler(event: MessageEvent) {
      if (event.data.id !== id) return;
      w.removeEventListener('message', handler);

      if (event.data.success) {
        resolve(event.data.result as ParsedFile);
      } else {
        reject(new Error(event.data.error as string));
      }
    }

    w.addEventListener('message', handler);

    // Transfer ArrayBuffer for zero-copy (Shapefile)
    if (data instanceof ArrayBuffer) {
      w.postMessage({ id, format, fileName, data }, [data]);
    } else {
      w.postMessage({ id, format, fileName, data });
    }
  });
}

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
 * Heavy formats (GeoJSON, CSV, Shapefile) are offloaded to a web worker
 * to prevent blocking the main thread and freezing the map UI.
 *
 * KML/GPX stay on main thread — they need DOMParser (unavailable in workers).
 * PMTiles stays on main thread — creates an object URL for MapLibre.
 */
export async function parseFile(file: File, format: SupportedFormat): Promise<ParsedFile> {
  // Offload heavy parsing to web worker
  if (WORKER_FORMATS.has(format)) {
    if (format === 'shapefile') {
      const buffer = await file.arrayBuffer();
      return parseInWorker(format, file.name, buffer);
    }
    const text = await file.text();
    return parseInWorker(format as 'geojson' | 'csv', file.name, text);
  }

  // Main-thread parsing for formats that need browser APIs
  switch (format) {
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

/**
 * Terminate the file parse worker.
 * Call on page unmount to free the worker thread.
 */
export function terminateParseWorker(): void {
  if (worker) {
    worker.terminate();
    worker = null;
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
