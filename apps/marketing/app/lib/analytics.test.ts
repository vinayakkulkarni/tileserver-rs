import { describe, expect, it, vi } from 'vitest';
import {
  createAnalytics,
  parseAnalyticsHosts,
  trackersAllowed,
} from './analytics';

function fakeHandles() {
  return {
    ga: { gtag: vi.fn() },
    plausible: { plausible: vi.fn() },
    openpanel: { track: vi.fn() },
  };
}

describe('createAnalytics', () => {
  it('sends one track() call to GA4, Plausible and OpenPanel', () => {
    const handles = fakeHandles();
    const { track, isEnabled } = createAnalytics(handles);

    track('github_click', { location: 'hero', target: 'repo' });

    const props = { location: 'hero', target: 'repo' };
    expect(isEnabled).toBe(true);
    expect(handles.ga.gtag).toHaveBeenCalledTimes(1);
    expect(handles.ga.gtag).toHaveBeenCalledWith(
      'event',
      'github_click',
      props,
    );
    expect(handles.plausible.plausible).toHaveBeenCalledTimes(1);
    expect(handles.plausible.plausible).toHaveBeenCalledWith('github_click', {
      props,
    });
    expect(handles.openpanel.track).toHaveBeenCalledTimes(1);
    expect(handles.openpanel.track).toHaveBeenCalledWith('github_click', props);
  });

  it('is a no-op when no tracker is loaded', () => {
    const { track, isEnabled } = createAnalytics({});
    expect(isEnabled).toBe(false);
    expect(() =>
      track('install_command_copied', { location: 'hero', method: 'homebrew' }),
    ).not.toThrow();
  });
});

describe('trackersAllowed', () => {
  const hosts = parseAnalyticsHosts(' tileserver.app , Localhost ,');

  it('parses the comma-separated host list', () => {
    expect(hosts).toEqual(['tileserver.app', 'localhost']);
  });

  it('allows a real browser on an allowed host', () => {
    expect(
      trackersAllowed({ webdriver: false, hostname: 'tileserver.app' }, hosts),
    ).toBe(true);
  });

  it('blocks automation browsers on an allowed host', () => {
    expect(
      trackersAllowed({ webdriver: true, hostname: 'tileserver.app' }, hosts),
    ).toBe(false);
  });

  it('blocks hosts outside the allow list', () => {
    expect(
      trackersAllowed(
        {
          webdriver: false,
          hostname: 'preview.tileserver-marketing.pages.dev',
        },
        hosts,
      ),
    ).toBe(false);
  });
});
