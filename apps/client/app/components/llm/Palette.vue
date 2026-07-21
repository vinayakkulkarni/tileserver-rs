<script setup lang="ts">
  import type { Map as MaplibreMap } from 'maplibre-gl';
  import type { OverlayLayer } from '~/types/file-upload';
  import type { LlmPaletteMode } from '~/types/llm';

  const props = defineProps<{
    mode: LlmPaletteMode;
    mapRef: MaplibreMap | null;
    overlays: OverlayLayer[];
  }>();

  const emit = defineEmits<{
    'update:mode': [value: LlmPaletteMode];
  }>();

  const {
    messages,
    isLoading,
    engineStatus,
    loadProgress,
    loadStageText,
    selectedModel,
    availableModels,
    engineError,
    input,
    showLoadingIndicator,
    scrollAnchor,
    stop,
    handleSubmit,
    handlePromptSelect,
    selectModel,
    suggestions,
    getIconComponent,
    panelRef,
    handleRef,
    isDragging,
    dragStyle,
    minimize,
    expand,
    close,
  } = useLlmPalette(
    computed(() => props.mode),
    computed(() => props.mapRef),
    computed(() => props.overlays),
    (mode: LlmPaletteMode) => emit('update:mode', mode),
  );

  function setPanelRef(el: Element | null) {
    panelRef.value = el as HTMLElement | null;
  }

  function setHandleRef(el: Element | null) {
    handleRef.value = el as HTMLElement | null;
  }

  function onAnchorMounted(el: HTMLElement | null) {
    scrollAnchor.value = el;
  }
</script>

<template>
  <div
    v-if="mode === 'minimized'"
    :ref="setPanelRef"
    :style="dragStyle"
    class="fixed z-50 flex items-center gap-2 border border-border bg-background/95 px-3 py-2 shadow-lg backdrop-blur-sm transition-shadow hover:shadow-xl"
    :class="{ 'cursor-grabbing': isDragging, 'cursor-grab': !isDragging }"
  >
    <LlmPaletteMinimized
      :set-handle-ref="setHandleRef"
      :engine-status="engineStatus"
      :is-dragging="isDragging"
      @expand="expand"
      @close="close"
    />
  </div>

  <div
    v-if="mode === 'expanded'"
    :ref="setPanelRef"
    :style="dragStyle"
    class="fixed z-50 flex h-[480px] w-llm-panel flex-col border border-border bg-background/95 shadow-2xl backdrop-blur-xl"
    :class="{ 'select-none': isDragging }"
  >
    <LlmPaletteExpandedHeader
      :set-handle-ref="setHandleRef"
      :is-dragging="isDragging"
      :engine-status="engineStatus"
      :selected-model="selectedModel"
      :available-models="availableModels"
      @minimize="minimize"
      @close="close"
      @select-model="selectModel"
    />
    <LlmPaletteExpandedBody
      :engine-error="engineError"
      :engine-status="engineStatus"
      :load-stage-text="loadStageText"
      :load-progress="loadProgress"
      :selected-model="selectedModel"
      :messages="messages"
      :show-loading-indicator="showLoadingIndicator"
      :is-loading="isLoading"
      :suggestions="suggestions"
      :input="input"
      :get-icon-component="getIconComponent"
      @update:input="input = $event"
      @submit="handleSubmit"
      @stop="stop"
      @prompt-select="handlePromptSelect"
      @anchor-mounted="onAnchorMounted"
    />
  </div>
</template>
