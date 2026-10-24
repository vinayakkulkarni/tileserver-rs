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

    track('demo_map_interaction', { view: 'style', interaction: 'zoom' });

    const props = { view: 'style', interaction: 'zoom' };
    expect(isEnabled).toBe(true);
    expect(handles.ga.gtag).toHaveBeenCalledTimes(1);
    expect(handles.ga.gtag).toHaveBeenCalledWith(
      'event',
      'demo_map_interaction',
      props,
    );
    expect(handles.plausible.plausible).toHaveBeenCalledTimes(1);
    expect(handles.plausible.plausible).toHaveBeenCalledWith(
      'demo_map_interaction',
      {
        props,
      },
    );
    expect(handles.openpanel.track).toHaveBeenCalledTimes(1);
    expect(handles.openpanel.track).toHaveBeenCalledWith(
      'demo_map_interaction',
      props,
    );
  });

  it('is a no-op when no tracker is loaded', () => {
    const { track, isEnabled } = createAnalytics({});
    expect(isEnabled).toBe(false);
    expect(() =>
      track('demo_file_dropped', { format: 'geojson', featureCount: 3 }),
    ).not.toThrow();
  });
});

describe('trackersAllowed', () => {
  const hosts = parseAnalyticsHosts(' demo.tileserver.app , Localhost ,');

  it('parses the comma-separated host list', () => {
    expect(hosts).toEqual(['demo.tileserver.app', 'localhost']);
  });

  it('allows a real browser on an allowed host', () => {
    expect(
      trackersAllowed(
        { webdriver: false, hostname: 'demo.tileserver.app' },
        hosts,
      ),
    ).toBe(true);
  });

  it('blocks automation browsers on an allowed host', () => {
    expect(
      trackersAllowed(
        { webdriver: true, hostname: 'demo.tileserver.app' },
        hosts,
      ),
    ).toBe(false);
  });

  it('blocks self-hosted instances outside the allow list', () => {
    expect(
      trackersAllowed(
        { webdriver: false, hostname: 'tiles.example.com' },
        hosts,
      ),
    ).toBe(false);
  });
});
