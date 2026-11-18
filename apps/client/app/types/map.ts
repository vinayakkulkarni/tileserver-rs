/**
 * Map Types
 *
 * Types for VMap component options.
 * VMap handles the container internally, so we omit it from options.
 */

import type { MapOptions } from 'maplibre-gl';

/**
 * Options for VMap component.
 * Container is managed by VMap internally.
 */
export type VMapOptions = Omit<MapOptions, 'container'>;

/**
 * Camera the map opens with: from the URL hash, the style, or the default.
 */
export interface InitialView {
  center: [number, number];
  zoom: number;
  bearing: number;
  pitch: number;
}

/** The optional root-level camera fields of a MapLibre style. */
export interface StyleCamera {
  center?: unknown;
  zoom?: unknown;
  bearing?: unknown;
  pitch?: unknown;
}
