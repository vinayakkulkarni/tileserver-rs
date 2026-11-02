/**
 * Live source updates for a map page.
 *
 * When the server has `[live] enabled = true` (`/ping` → `live_enabled`),
 * opens `GET /ws` and, for each `reload` message, re-fetches the tiles of
 * every style source fed by that server source. After a source has been
 * reloaded once, its tile and TileJSON requests skip the browser HTTP
 * cache (tiles are served with a one-day `max-age`).
 */
import { useQuery } from '@tanstack/vue-query';
import type { Map, RequestParameters } from 'maplibre-gl';
import type { LiveStyleSource } from '~/types';
import {
  isDataUrlFor,
  liveSocketUrl,
  parseLiveMessage,
  reconnectDelayMs,
  styleSourcesFor,
} from '~/lib/live-updates';
import { pingQueryOptions } from '~/utils/api/server/queries';

export function useLiveUpdates(mapRef: Ref<Map | null>) {
  const pingQuery = useQuery(pingQueryOptions());
  const enabled = computed(() => pingQuery.data.value?.live_enabled === true);
  /** Server sources reloaded since the page opened. */
  const reloaded = new Set<string>();
  let socket: WebSocket | null = null;
  let attempt = 0;
  let retryTimer: ReturnType<typeof setTimeout> | undefined;
  let stopped = false;

  function transformRequest(url: string): RequestParameters {
    for (const id of reloaded) {
      if (isDataUrlFor(url, id)) return { url, cache: 'no-cache' };
    }
    return { url };
  }

  function reloadSource(map: Map, sourceId: string): void {
    reloaded.add(sourceId);
    const sources = map.getStyle()?.sources ?? {};
    const specs: Record<string, LiveStyleSource> = {};
    for (const [id, spec] of Object.entries(sources)) {
      specs[id] = {
        url:
          'url' in spec && typeof spec.url === 'string' ? spec.url : undefined,
        tiles:
          'tiles' in spec && Array.isArray(spec.tiles) ? spec.tiles : undefined,
      };
    }
    for (const id of styleSourcesFor(specs, sourceId)) map.refreshTiles(id);
  }

  function connect(map: Map): void {
    if (stopped || socket) return;
    socket = new WebSocket(
      liveSocketUrl(window.location.origin, apiUrl('/ws')),
    );
    socket.addEventListener('open', () => {
      attempt = 0;
    });
    socket.addEventListener('message', (event: MessageEvent<unknown>) => {
      if (typeof event.data !== 'string') return;
      const message = parseLiveMessage(event.data);
      if (message?.type === 'reload') reloadSource(map, message.source);
    });
    socket.addEventListener('close', () => {
      socket = null;
      if (stopped) return;
      attempt += 1;
      retryTimer = setTimeout(() => connect(map), reconnectDelayMs(attempt));
    });
  }

  function stop(): void {
    stopped = true;
    clearTimeout(retryTimer);
    socket?.close();
    socket = null;
  }

  watch(
    [mapRef, enabled],
    ([map, on]) => {
      if (!map || !on || !import.meta.client) return;
      map.setTransformRequest(transformRequest);
      connect(map);
    },
    { immediate: true },
  );

  onBeforeUnmount(stop);

  return { liveEnabled: enabled };
}
