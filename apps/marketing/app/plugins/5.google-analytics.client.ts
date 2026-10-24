import { parseAnalyticsHosts, trackersAllowed } from '~/lib/analytics';

export default defineNuxtPlugin(() => {
  const config = useRuntimeConfig().public;
  const gtagId = config.gtagId;
  if (!gtagId) return;

  const allowed = trackersAllowed(
    { webdriver: navigator.webdriver, hostname: location.hostname },
    parseAnalyticsHosts(config.analyticsHosts),
  );
  if (!allowed) return;

  const { proxy } = useScriptGoogleAnalytics({ id: gtagId });

  function gtag(...args: unknown[]) {
    (proxy.gtag as (...a: unknown[]) => void)(...args);
  }

  return {
    provide: {
      ga: { gtag },
    },
  };
});
