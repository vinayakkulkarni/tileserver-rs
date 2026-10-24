import type { CtaLocation, CtaTarget, GithubTarget } from '~/types/analytics';
import { useAnalytics } from './use-analytics';

/** Click handlers for the marketing funnel: CTAs and GitHub outbound links. */
export function useMarketingTracking() {
  const { track } = useAnalytics();

  function trackCta(location: CtaLocation, target: CtaTarget) {
    track('cta_click', { location, target });
  }

  function trackGithub(location: CtaLocation, target: GithubTarget) {
    track('github_click', { location, target });
  }

  return { trackCta, trackGithub };
}
