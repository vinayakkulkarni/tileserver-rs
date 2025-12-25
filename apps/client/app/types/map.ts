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
