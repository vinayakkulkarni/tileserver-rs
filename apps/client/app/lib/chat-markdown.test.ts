import { parseMarkdown } from '@comark/nuxt/parse';
import { describe, expect, it } from 'vitest';

import { chatMarkdownPlugins } from './chat-markdown';

const render = async (markdown: string) =>
  JSON.stringify(
    (await parseMarkdown(markdown, { plugins: chatMarkdownPlugins })).nodes,
  );

describe('chat markdown sanitising', () => {
  it('drops event-handler attributes from raw HTML', async () => {
    const out = await render('hi <img src="x" onerror="alert(1)">');
    expect(out).not.toContain('onerror');
    expect(out).not.toContain('alert(1)');
  });

  it('removes script and iframe tags', async () => {
    const out = await render(
      '<script>alert(1)</script><iframe src="https://evil.example"></iframe>',
    );
    expect(out).not.toContain('script');
    expect(out).not.toContain('iframe');
  });

  it('drops javascript: links but keeps normal markdown', async () => {
    const out = await render(
      '<a href="javascript:alert(1)">x</a> and **bold** [b](https://ok.example)',
    );
    expect(out).not.toContain('javascript:');
    expect(out).toContain('strong');
    expect(out).toContain('https://ok.example');
  });
});
