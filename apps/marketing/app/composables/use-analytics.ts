import type {
  GtagHandle,
  OpenPanelHandle,
  PlausibleHandle,
} from '~/types/analytics';
import { createAnalytics } from '~/lib/analytics';

/**
 * One `track(event, props)` that reaches Plausible, OpenPanel and GA4.
 *
 * Each handle is provided by a client plugin only when its tracker loads
 * (see `app/plugins/*.client.ts`), so on the server, in automation browsers
 * and on non-production hosts `track()` is a no-op.
 */
export function useAnalytics() {
  const nuxtApp = useNuxtApp();

  return createAnalytics({
    ga: nuxtApp.$ga as GtagHandle | undefined,
    plausible: nuxtApp.$plausible as PlausibleHandle | undefined,
    openpanel: nuxtApp.$openpanelTracker as OpenPanelHandle | undefined,
  });
}
