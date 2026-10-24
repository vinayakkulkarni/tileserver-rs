import type { Map, MapMovementEvent } from 'maplibre-gl';
import type {
  MapGestureEvent,
  MapInteraction,
  MapView,
} from '~/types/analytics';
import { useAnalytics } from './use-analytics';

const GESTURES: ReadonlyArray<readonly [MapInteraction, MapGestureEvent]> = [
  ['drag', 'dragstart'],
  ['zoom', 'zoomstart'],
  ['rotate', 'rotatestart'],
  ['pitch', 'pitchstart'],
];

/**
 * Count demo engagement: the first user gesture of each kind on a map sends
 * one `demo_map_interaction` event. Camera moves the app makes itself (hash
 * restore, fitBounds) carry no `originalEvent` and are ignored.
 */
export function useMapInteractionTracking(view: MapView) {
  const { track, isEnabled } = useAnalytics();

  function watchMap(map: Map) {
    if (!isEnabled) return;

    for (const [interaction, type] of GESTURES) {
      const onGesture = (event: MapMovementEvent) => {
        if (!event.originalEvent) return;
        map.off(type, onGesture);
        track('demo_map_interaction', { view, interaction });
      };
      map.on(type, onGesture);
    }
  }

  return { watchMap };
}
