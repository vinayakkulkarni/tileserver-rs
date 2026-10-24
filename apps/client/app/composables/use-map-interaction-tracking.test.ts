import type { Map } from 'maplibre-gl';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useMapInteractionTracking } from './use-map-interaction-tracking';

const track = vi.fn();
let isEnabled = true;

vi.mock('./use-analytics', () => ({
  useAnalytics: () => ({ track, isEnabled }),
}));

type Handler = (event: { originalEvent?: unknown }) => void;

/** Minimal MapLibre stand-in: records on/off and lets the test fire events. */
function fakeMap() {
  const handlers: Record<string, Handler[]> = {};
  const map = {
    on: (type: string, fn: Handler) => {
      (handlers[type] ||= []).push(fn);
    },
    off: (type: string, fn: Handler) => {
      handlers[type] = (handlers[type] || []).filter((h) => h !== fn);
    },
  };
  function fire(type: string, event: { originalEvent?: unknown }) {
    for (const fn of [...(handlers[type] || [])]) fn(event);
  }
  return { map: map as unknown as Map, fire };
}

describe('useMapInteractionTracking', () => {
  beforeEach(() => {
    track.mockClear();
    isEnabled = true;
  });

  it('sends one event per gesture kind for user gestures', () => {
    const { map, fire } = fakeMap();
    useMapInteractionTracking('style').watchMap(map);

    fire('zoomstart', { originalEvent: {} });
    fire('zoomstart', { originalEvent: {} });
    fire('dragstart', { originalEvent: {} });

    expect(track).toHaveBeenCalledTimes(2);
    expect(track).toHaveBeenNthCalledWith(1, 'demo_map_interaction', {
      view: 'style',
      interaction: 'zoom',
    });
    expect(track).toHaveBeenNthCalledWith(2, 'demo_map_interaction', {
      view: 'style',
      interaction: 'drag',
    });
  });

  it('ignores camera moves the app makes itself', () => {
    const { map, fire } = fakeMap();
    useMapInteractionTracking('data').watchMap(map);

    fire('zoomstart', {});
    fire('rotatestart', {});

    expect(track).not.toHaveBeenCalled();
  });

  it('attaches nothing when no tracker is loaded', () => {
    isEnabled = false;
    const { map, fire } = fakeMap();
    useMapInteractionTracking('style').watchMap(map);

    fire('pitchstart', { originalEvent: {} });

    expect(track).not.toHaveBeenCalled();
  });
});
