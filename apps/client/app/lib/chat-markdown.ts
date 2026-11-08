/**
 * Markdown plugins for LLM chat replies.
 *
 * Model output is untrusted: a prompt-injected reply could carry raw HTML
 * with `onerror=` handlers or `javascript:` links. The comark security
 * plugin strips event-handler attributes and unsafe URLs, and the blocked
 * tags below are removed from the tree entirely.
 */
import security from '@comark/nuxt/plugins/security';

const blockedTags = [
  'script',
  'style',
  'iframe',
  'object',
  'embed',
  'form',
  'input',
  'button',
  'link',
  'meta',
  'base',
];

export const chatMarkdownPlugins = [security({ blockedTags })];
