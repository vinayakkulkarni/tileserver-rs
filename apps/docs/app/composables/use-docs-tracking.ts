import type { DocsLinkLocation, GithubTarget } from '~/types/analytics';
import { classifyCopiedCode } from '~/lib/code-copy';
import { useAnalytics } from './use-analytics';

/** Funnel events for the docs site: code copies and outbound links. */
export function useDocsTracking() {
  const { track } = useAnalytics();
  const route = useRoute();

  function trackCodeCopied(code: string, language: string | undefined) {
    const page = route.path;
    const copied = classifyCopiedCode(code);
    if (copied.kind === 'docker') {
      track('docker_command_copied', { page, command: copied.command });
    } else if (copied.kind === 'install') {
      track('install_command_copied', { page, method: copied.method });
    } else {
      track('code_copied', { page, language: language ?? 'text' });
    }
  }

  function trackGithub(location: DocsLinkLocation, target: GithubTarget) {
    track('github_click', { location, target });
  }

  function trackMarketing(location: DocsLinkLocation) {
    track('cta_click', { location, target: 'marketing' });
  }

  return { trackCodeCopied, trackGithub, trackMarketing };
}
