/**
 * Initial camera for the map viewers.
 *
 * Order: URL hash (`#zoom/lat/lng/bearing/pitch`), then the style's own
 * `center`/`zoom`/`bearing`/`pitch`, then the world view.
 *
 * MapLibre's `hash: true` rejects a hash whose pitch is above `maxPitch`
 * (default 60) and silently falls back to the `center`/`zoom` options, so
 * terrain links like `#10.5/34.1/77.62/-30/70` opened at 0,0. The viewers
 * therefore raise `maxPitch` to `VIEWER_MAX_PITCH` and pass the resolved
 * view explicitly.
 */

import type { InitialView, StyleCamera } from '~/types/map';

export const VIEWER_MAX_PITCH = 85;

const MIN_ZOOM = -2;
const MAX_ZOOM = 22;

export const DEFAULT_VIEW: InitialView = {
  center: [0, 0],
  zoom: 1,
  bearing: 0,
  pitch: 0,
};

function inRange(value: number, min: number, max: number): boolean {
  return Number.isFinite(value) && value >= min && value <= max;
}

function isLngLat(lng: number, lat: number): boolean {
  return Number.isFinite(lng) && inRange(lat, -90, 90);
}

/** Parse a MapLibre map hash. Returns null when it is absent or invalid. */
export function parseMapHash(hash: string): InitialView | null {
  const raw = hash.startsWith('#') ? hash.slice(1) : hash;
  if (!raw) return null;
  const parts = raw.split('/');
  if (parts.length < 3 || parts.includes('')) return null;
  const [zoom = NaN, lat = NaN, lng = NaN, bearing = 0, pitch = 0] =
    parts.map(Number);
  if (!inRange(zoom, MIN_ZOOM, MAX_ZOOM) || !isLngLat(lng, lat)) return null;
  if (!inRange(bearing, -180, 180) || !inRange(pitch, 0, VIEWER_MAX_PITCH)) {
    return null;
  }
  return { center: [lng, lat], zoom, bearing, pitch };
}

function numberOr(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

/** Read the camera a style declares. Returns null when it sets no center. */
export function styleView(style: StyleCamera | null | undefined) {
  const center = style?.center;
  if (!Array.isArray(center) || center.length < 2) return null;
  const [lng, lat] = center;
  if (typeof lng !== 'number' || typeof lat !== 'number') return null;
  if (!isLngLat(lng, lat)) return null;
  const view: InitialView = {
    center: [lng, lat],
    zoom: numberOr(style?.zoom, DEFAULT_VIEW.zoom),
    bearing: numberOr(style?.bearing, 0),
    pitch: Math.min(numberOr(style?.pitch, 0), VIEWER_MAX_PITCH),
  };
  return view;
}

/** Hash first, then the style's camera, then the world view. */
export function resolveInitialView(
  hash: string,
  style?: StyleCamera | null,
): InitialView {
  return parseMapHash(hash) ?? styleView(style) ?? { ...DEFAULT_VIEW };
}
