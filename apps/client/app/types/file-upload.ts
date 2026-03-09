import type { GeoJSON } from 'geojson';
import type { LayerSpecification, SourceSpecification } from 'maplibre-gl';

/** Supported geospatial file formats for drag-and-drop */
export type SupportedFormat =
  | 'geojson'
  | 'kml'
  | 'gpx'
  | 'csv'
  | 'shapefile'
  | 'pmtiles'
  | 'mbtiles'
  | 'sqlite'
  | 'cog';

/** Result of parsing a dropped file */
export interface ParsedFile {
  /** Original file name */
  fileName: string;
  /** Detected format */
  format: SupportedFormat;
  /** GeoJSON data (for client-side formats) */
  data?: GeoJSON;
  /** Object URL (for PMTiles) */
  objectUrl?: string;
  /** Number of features parsed */
  featureCount: number;
  /** Geometry types found in the data */
  geometryTypes: GeometryType[];
}

/** Geometry types for auto-styling */
export type GeometryType = 'Point' | 'LineString' | 'Polygon';

/** A single overlay layer added to the map */
export interface OverlayLayer {
  /** Unique layer ID */
  id: string;
  /** Original file name */
  fileName: string;
  /** Detected format */
  format: SupportedFormat;
  /** Number of features */
  featureCount: number;
  /** Assigned color from palette */
  color: string;
  /** Whether the layer is visible */
  visible: boolean;
  /** MapLibre source ID */
  sourceId: string;
  /** MapLibre layer IDs (multiple for multi-geometry files) */
  layerIds: string[];
  /** Server-side upload ID (for cleanup on remove) */
  uploadId?: string;
  }

/** Configuration for adding an overlay to the map */
export interface OverlaySourceConfig {
  sourceId: string;
  source: SourceSpecification;
  layers: LayerSpecification[];
}

/** File drop processing status */
export type FileDropStatus = 'idle' | 'dragging' | 'processing' | 'uploading' | 'error';

/** Error info from file drop processing */
export interface FileDropError {
  fileName: string;
  message: string;
}

/** Success info from file drop processing */
export interface FileDropSuccess {
  fileName: string;
  featureCount: number;
  format: SupportedFormat;
}

/** File extension to format mapping */
export const FORMAT_EXTENSIONS: Record<string, SupportedFormat> = {
  '.geojson': 'geojson',
  '.json': 'geojson',
  '.kml': 'kml',
  '.gpx': 'gpx',
  '.csv': 'csv',
  '.tsv': 'csv',
  '.zip': 'shapefile',
  '.shp': 'shapefile',
  '.pmtiles': 'pmtiles',
  '.mbtiles': 'mbtiles',
  '.sqlite': 'sqlite',
  '.db': 'sqlite',
  '.tif': 'cog',
  '.tiff': 'cog',
};

/** Formats that can be parsed entirely client-side */
export const CLIENT_SIDE_FORMATS: ReadonlySet<SupportedFormat> = new Set([
  'geojson',
  'kml',
  'gpx',
  'csv',
  'shapefile',
  'pmtiles',
]);

/** Formats that require server-side processing */
export const SERVER_SIDE_FORMATS: ReadonlySet<SupportedFormat> = new Set([
  'mbtiles',
  'sqlite',
  'cog',
]);

/** Maximum file size for client-side processing (50 MB) */
export const MAX_FILE_SIZE_BYTES = 50 * 1024 * 1024;

/** Response from server-side file upload */
export interface UploadResponse {
  id: string;
  source_id: string;
  file_name: string;
  format: string;
  tilejson_url: string;
}
