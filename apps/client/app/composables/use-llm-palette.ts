import { useDraggable, useStorage, useWindowSize } from '@vueuse/core';
import type { Map as MaplibreMap } from 'maplibre-gl';
import type { OverlayLayer } from '~/types/file-upload';
import type { LlmPaletteMode, LlmPalettePosition } from '~/types/llm';

const PALETTE_WIDTH = 400;
const PALETTE_HEIGHT_EXPANDED = 480;
const PILL_WIDTH = 160;
const PILL_HEIGHT = 40;

function getDefaultPosition(
  windowWidth: number,
  windowHeight: number,
): LlmPalettePosition {
  return {
    x: windowWidth - PALETTE_WIDTH - 24,
    y: windowHeight - PALETTE_HEIGHT_EXPANDED - 24,
  };
}

export function useLlmPalette(
  modeRef: Ref<LlmPaletteMode>,
  mapRef: Ref<MaplibreMap | null>,
  overlaysRef: Ref<readonly OverlayLayer[]>,
  emitMode: (mode: LlmPaletteMode) => void,
) {
  const panelData = useLlmPanel(mapRef, overlaysRef);
  const { scrollAnchor } = panelData;

  const panelRef = ref<HTMLElement | null>(null);
  const handleRef = ref<HTMLElement | null>(null);
  const { width: winWidth, height: winHeight } = useWindowSize();

  const savedPosition = useStorage<LlmPalettePosition>(
    'tileserver-llm-position',
    getDefaultPosition(winWidth.value, winHeight.value),
  );

  const {
    x,
    y,
    isDragging,
    style: dragStyle,
  } = useDraggable(panelRef, {
    initialValue: { x: savedPosition.value.x, y: savedPosition.value.y },
    handle: handleRef,
    onEnd: (pos) => {
      savedPosition.value = { x: pos.x, y: pos.y };
    },
  });

  function clampPosition() {
    const w = modeRef.value === 'expanded' ? PALETTE_WIDTH : PILL_WIDTH;
    const h =
      modeRef.value === 'expanded' ? PALETTE_HEIGHT_EXPANDED : PILL_HEIGHT;
    x.value = Math.max(0, Math.min(x.value, winWidth.value - w));
    y.value = Math.max(0, Math.min(y.value, winHeight.value - h));
  }

  watch([winWidth, winHeight], clampPosition);

  function minimize() {
    emitMode('minimized');
  }

  function expand() {
    emitMode('expanded');
    nextTick(() => {
      clampPosition();
      panelRef.value?.querySelector<HTMLInputElement>('input')?.focus();
      nextTick(() => scrollAnchor.value?.scrollIntoView({ block: 'end' }));
    });
  }

  function close() {
    emitMode('closed');
  }

  function resetPosition() {
    const pos = getDefaultPosition(winWidth.value, winHeight.value);
    x.value = pos.x;
    y.value = pos.y;
    savedPosition.value = pos;
  }

  watch(modeRef, (mode) => {
    if (mode === 'expanded') {
      nextTick(() => {
        clampPosition();
        panelRef.value?.querySelector<HTMLInputElement>('input')?.focus();
        nextTick(() => scrollAnchor.value?.scrollIntoView({ block: 'end' }));
      });
    }
  });

  return {
    ...panelData,
    panelRef,
    handleRef,
    isDragging,
    dragStyle,
    x,
    y,
    minimize,
    expand,
    close,
    resetPosition,
  };
}
