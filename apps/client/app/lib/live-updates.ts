/**
 * Pure helpers for live source updates (`GET /ws`).
 */
import type { LiveServerMessage, LiveStyleSource } from '~/types';

/** `ws(s)://host/<base>/ws` for a page served at `origin`. */
export function liveSocketUrl(origin: string, path: string): string {
  const url = new URL(path, origin);
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  return url.toString();
}

/** Parse a server frame; `null` for anything that is not a known message. */
export function parseLiveMessage(text: string): LiveServerMessage | null {
  try {
    const value: unknown = JSON.parse(text);
    if (typeof value !== 'object' || value === null || !('type' in value)) {
      return null;
    }
    if (
      value.type === 'reload' &&
      'source' in value &&
      typeof value.source === 'string'
    ) {
      return { type: 'reload', source: value.source };
    }
    if (
      value.type === 'hello' &&
      'sources' in value &&
      Array.isArray(value.sources) &&
      value.sources.every((s) => typeof s === 'string')
    ) {
      return { type: 'hello', sources: value.sources };
    }
    return null;
  } catch {
    return null;
  }
}

/** Whether `url` points at the server's data endpoints for `sourceId`. */
export function isDataUrlFor(url: string, sourceId: string): boolean {
  const path = (() => {
    try {
      return new URL(url, 'http://localhost').pathname;
    } catch {
      return url;
    }
  })();
  const marker = `/data/${sourceId}`;
  const at = path.indexOf(marker);
  if (at === -1) return false;
  const next = path.charAt(at + marker.length);
  return next === '' || next === '/' || next === '.';
}

/** Style source ids whose URL or tiles come from server source `sourceId`. */
export function styleSourcesFor(
  sources: Record<string, LiveStyleSource>,
  sourceId: string,
): string[] {
  return Object.entries(sources)
    .filter(([, spec]) =>
      [spec.url, ...(spec.tiles ?? [])].some(
        (u) => typeof u === 'string' && isDataUrlFor(u, sourceId),
      ),
    )
    .map(([id]) => id);
}

/** Delay before reconnect attempt `attempt` (1-based): 1 s doubling to 30 s. */
export function reconnectDelayMs(attempt: number): number {
  return Math.min(30_000, 1000 * 2 ** Math.max(0, attempt - 1));
}
