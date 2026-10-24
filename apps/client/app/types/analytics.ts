/**
 * Analytics types for the tileserver-rs web UI (demo.tileserver.app).
 *
 * Every product event is a key of `AnalyticsEventMap`, so `track()` only
 * accepts known event names and the props that belong to each one.
 */
import type { SupportedFormat } from './file-upload';

/** Map pages in the UI. */
export type MapView = 'style' | 'data';

/** User gestures counted as demo engagement (once per gesture per map). */
export type MapInteraction = 'drag' | 'zoom' | 'rotate' | 'pitch';

/** MapLibre events that mark the start of each gesture. */
export type MapGestureEvent =
  | 'dragstart'
  | 'zoomstart'
  | 'rotatestart'
  | 'pitchstart';

export interface AnalyticsEventMap {
  demo_map_interaction: { view: MapView; interaction: MapInteraction };
  demo_file_dropped: { format: SupportedFormat; featureCount: number };
}

export type AnalyticsEvent = keyof AnalyticsEventMap;

/** Shape of the GA4 handle provided as `$ga`. */
export interface GtagHandle {
  gtag: (...args: unknown[]) => void;
}

/** Shape of the Plausible handle provided as `$plausible`. */
export interface PlausibleHandle {
  plausible: (...args: unknown[]) => void;
}

/** Shape of the OpenPanel handle provided as `$openpanelTracker`. */
export interface OpenPanelHandle {
  track: (name: string, properties?: Record<string, unknown>) => void;
}

/** The part of the `@openpanel/web` SDK (`$openpanel`) the gate plugin uses. */
export interface OpenPanelSdk extends OpenPanelHandle {
  ready: () => void;
}

export interface AnalyticsHandles {
  ga?: GtagHandle;
  plausible?: PlausibleHandle;
  openpanel?: OpenPanelHandle;
}

/**
 * The Plausible `pa-*.js` global: callable, plus the loader bookkeeping —
 * `q` (pre-load queue), `o` (init options the loader boots from), `init`.
 */
export interface PlausibleFn {
  (...args: unknown[]): void;
  q?: unknown[];
  o?: Record<string, unknown>;
  init?: (options?: Record<string, unknown>) => void;
}

/** Browser facts the tracker gate decides on. */
export interface TrackerGateInput {
  webdriver: boolean | undefined;
  hostname: string;
}
