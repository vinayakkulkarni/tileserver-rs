/**
 * Parse CSV text with lat/lng column detection.
 * Runs in a web worker via nuxt-workers — auto-imported, zero config.
 */
import type { FeatureCollection, Feature } from 'geojson';
import type { GeometryType } from '~/types/file-upload';

interface ParseResult {
  fileName: string;
  format: 'csv';
  data: FeatureCollection;
  featureCount: number;
  geometryTypes: GeometryType[];
}

export async function parseCSV(fileName: string, text: string): Promise<ParseResult> {
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const LAT_ALIASES = new Set(['lat', 'latitude', 'y', 'lat_y', 'point_y']);
const LON_ALIASES = new Set(['lon', 'lng', 'longitude', 'x', 'long', 'lon_x', 'point_x']);

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
