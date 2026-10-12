/**
 * MapLibre Worker URL
 *
 * MapLibre GL v6 computes its worker URL at runtime, so no bundler sees it.
 * `?worker&url` makes Vite bundle the worker of the INSTALLED maplibre-gl
 * (including its `maplibre-gl-shared.mjs` sibling) into one file. The worker
 * and the main thread therefore always run the same maplibre-gl version.
 *
 * @geoql/v-maplibre 3.0.1 also ships a worker, but that worker is frozen at the
 * maplibre-gl version it was built with (6.10.0). With a newer maplibre-gl on
 * the main thread, tiles fail with "can't deserialize unregistered class".
 * v-maplibre keeps a worker URL that is already set, so this one wins.
 *
 * Call before the first map is created (page composables do this).
 */
import { setWorkerUrl } from 'maplibre-gl';
import maplibreWorkerUrl from 'maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url';

export function ensureMaplibreWorker(): void {
  if (import.meta.client) setWorkerUrl(maplibreWorkerUrl);
}
