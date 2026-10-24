import type {
  AnalyticsEvent,
  AnalyticsEventMap,
  AnalyticsHandles,
  TrackerGateInput,
} from '~/types/analytics';

/**
 * Fan one product event out to every tracker that is loaded.
 *
 * Kept free of Nuxt imports so the unit test can drive it with fake handles.
 */
export function createAnalytics(handles: AnalyticsHandles) {
  const isEnabled = Boolean(
    handles.ga || handles.plausible || handles.openpanel,
  );

  function track<E extends AnalyticsEvent>(
    event: E,
    properties: AnalyticsEventMap[E],
  ) {
    const props: Record<string, unknown> = { ...properties };
    handles.ga?.gtag('event', event, props);
    handles.plausible?.plausible(event, { props });
    handles.openpanel?.track(event, props);
  }

  return { isEnabled, track };
}

/** Split the comma-separated `analyticsHosts` runtime config value. */
export function parseAnalyticsHosts(value: string): string[] {
  return value
    .split(',')
    .map((host) => host.trim().toLowerCase())
    .filter((host) => host.length > 0);
}

/**
 * Decide whether trackers may load in this browser.
 *
 * - Automation browsers (Playwright, Puppeteer, Selenium all set
 *   `navigator.webdriver`) get no trackers: bot traffic pollutes the data and
 *   third-party beacons stretch the network-idle window that agent checkers
 *   wait on.
 * - Only the hostnames in `analyticsHosts` load trackers, so preview deploys
 *   and local builds stay out of the production data unless asked for.
 */
export function trackersAllowed(
  input: TrackerGateInput,
  allowedHosts: string[],
): boolean {
  if (input.webdriver === true) return false;
  return allowedHosts.includes(input.hostname.toLowerCase());
}
