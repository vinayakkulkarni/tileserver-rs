import type { MaybeRefOrGetter } from 'vue';

const MAX_ZOOM = 22;

export function useCoverageBar(
  minzoom: MaybeRefOrGetter<number | undefined>,
  maxzoom: MaybeRefOrGetter<number | undefined>,
) {
  const min = computed(() => toValue(minzoom) ?? 0);
  const max = computed(() => toValue(maxzoom) ?? MAX_ZOOM);

  const left = computed(() => `${(min.value * 100) / MAX_ZOOM}%`);
  const width = computed(
    () => `${((max.value - min.value) * 100) / MAX_ZOOM}%`,
  );
  const ariaLabel = computed(() => `Zoom range ${min.value} to ${max.value}`);
  const rangeLabel = computed(() => `z${min.value}–${max.value}`);
  const ceilingLabel = `z${MAX_ZOOM}`;
  const floorLabel = 'z0';

  return {
    min,
    max,
    left,
    width,
    ariaLabel,
    rangeLabel,
    ceilingLabel,
    floorLabel,
  };
}
