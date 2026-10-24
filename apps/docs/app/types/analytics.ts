/**
 * Analytics types for docs.tileserver.app.
 *
 * Every product event is a key of `AnalyticsEventMap`, so `track()` only
 * accepts known event names and the props that belong to each one.
 */

/** Where on the docs site a link sits. */
export type DocsLinkLocation = 'header' | 'hero' | 'home_footer';

/** GitHub destinations (repo home, license file). */
export type GithubTarget = 'repo' | 'license';

/** Install channels documented on the site. */
export type InstallMethod = 'homebrew' | 'cargo';

/** Docker commands a copied code block can contain. */
export type DockerCommand = 'pull' | 'run' | 'compose';

export interface AnalyticsEventMap {
  /** A code block was copied that is neither an install nor a Docker command. */
  code_copied: { page: string; language: string };
  docker_command_copied: { page: string; command: DockerCommand };
  install_command_copied: { page: string; method: InstallMethod };
  github_click: { location: DocsLinkLocation; target: GithubTarget };
  cta_click: { location: DocsLinkLocation; target: 'marketing' };
}

export type AnalyticsEvent = keyof AnalyticsEventMap;

/** What a copied code block was, used to pick the funnel event. */
export type CopiedCodeKind =
  | { kind: 'docker'; command: DockerCommand }
  | { kind: 'install'; method: InstallMethod }
  | { kind: 'other' };

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
