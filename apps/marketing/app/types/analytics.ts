/**
 * Analytics types for tileserver.app.
 *
 * Every product event is a key of `AnalyticsEventMap`, so `track()` only
 * accepts known event names and the props that belong to each one.
 */

/** Where on the page a call to action sits. */
export type CtaLocation =
  | 'nav'
  | 'mobile_nav'
  | 'hero'
  | 'ai_section'
  | 'api_section'
  | 'performance_section'
  | 'cta_section'
  | 'footer';

/** Non-GitHub destinations a call to action points at. */
export type CtaTarget =
  | 'docs'
  | 'quickstart'
  | 'demo'
  | 'openapi'
  | 'benchmarks'
  | 'ai_docs'
  | 'author';

/** GitHub destinations (repo home, releases page, license file). */
export type GithubTarget = 'repo' | 'releases' | 'license';

/** Install channels the site advertises. */
export type InstallMethod = 'homebrew';

export interface AnalyticsEventMap {
  cta_click: { location: CtaLocation; target: CtaTarget };
  github_click: { location: CtaLocation; target: GithubTarget };
  install_command_copied: { location: CtaLocation; method: InstallMethod };
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
