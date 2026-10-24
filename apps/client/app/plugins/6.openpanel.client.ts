import type { OpenPanelSdk } from '~/types/analytics';
import { parseAnalyticsHosts, trackersAllowed } from '~/lib/analytics';

/**
 * `@openpanel/nuxt` is configured with `disabled: true`, so its SDK queues
 * screen views and clicks in memory without sending them. This plugin calls
 * `ready()` — which enables the SDK and flushes the queue — only when the
 * tracker gate passes, and provides the handle `useAnalytics()` uses.
 */
export default defineNuxtPlugin({
  name: 'openpanel-gate',
  dependsOn: ['openpanel'],
  setup(nuxtApp) {
    const config = useRuntimeConfig().public;
    const allowed = trackersAllowed(
      { webdriver: navigator.webdriver, hostname: location.hostname },
      parseAnalyticsHosts(config.analyticsHosts),
    );
    if (!allowed) return;

    const openpanel: OpenPanelSdk = nuxtApp.$openpanel;
    openpanel.ready();

    return {
      provide: {
        openpanelTracker: openpanel,
      },
    };
  },
});
