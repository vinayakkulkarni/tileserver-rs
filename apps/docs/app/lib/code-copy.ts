import type { CopiedCodeKind } from '~/types/analytics';

/**
 * Classify a copied code block so the docs funnel can count Docker and
 * install copies apart from every other snippet. Only installs of
 * tileserver-rs itself count — `brew install ninja` is just a snippet.
 */
export function classifyCopiedCode(code: string): CopiedCodeKind {
  const text = code.toLowerCase();
  if (/\bdocker(?:-|\s+)compose\b/.test(text)) {
    return { kind: 'docker', command: 'compose' };
  }
  if (/\bdocker\s+pull\b/.test(text))
    return { kind: 'docker', command: 'pull' };
  if (/\bdocker\s+run\b/.test(text)) return { kind: 'docker', command: 'run' };
  if (/\bbrew\s+install\s[^\n]*tileserver-rs/.test(text)) {
    return { kind: 'install', method: 'homebrew' };
  }
  if (/\bcargo\s+install\s[^\n]*tileserver-rs/.test(text)) {
    return { kind: 'install', method: 'cargo' };
  }
  return { kind: 'other' };
}
