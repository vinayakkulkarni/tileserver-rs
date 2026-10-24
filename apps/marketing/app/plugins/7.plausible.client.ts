import type { PlausibleFn } from '~/types/analytics';
import { parseAnalyticsHosts, trackersAllowed } from '~/lib/analytics';

export default defineNuxtPlugin(() => {
  const config = useRuntimeConfig().public;
  const scriptId = config.plausibleScriptId;
  const apiHost = config.plausibleApiHost;
  if (!scriptId || !apiHost) return;

  const allowed = trackersAllowed(
    { webdriver: navigator.webdriver, hostname: location.hostname },
    parseAnalyticsHosts(config.analyticsHosts),
  );
  if (!allowed) return;

  // The Oct-2025 pa-*.js tracker only boots if `window.plausible.o` is set
  // when it loads. Plausible's official snippet defines the queue stub and
  // calls init() to set `.o` BEFORE the async script arrives. The script
  // carries its own domain and outbound-link setting from the dashboard.
  const w = window as unknown as { plausible?: PlausibleFn };
  const stub: PlausibleFn =
    w.plausible ??
    ((...args: unknown[]) => {
      (stub.q ||= []).push(args);
    });
  stub.init ||= (options?: Record<string, unknown>) => {
    stub.o = options || {};
  };
  w.plausible = stub;
  stub.init({ autoCapturePageviews: true });

  useHead({
    script: [{ src: `${apiHost}/js/pa-${scriptId}.js`, async: true }],
  });

  function plausible(...args: unknown[]) {
    w.plausible?.(...args);
  }

  return {
    provide: {
      plausible: { plausible },
    },
  };
});
