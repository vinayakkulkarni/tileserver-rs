<script setup lang="ts">
  import {
    Sheet as UiSheet,
    SheetContent as UiSheetContent,
  } from '~/components/ui/sheet';
  import { ScrollArea as UiScrollArea } from '~/components/ui/scroll-area';
  import type { Map as MaplibreMap } from 'maplibre-gl';

  const props = defineProps<{
    open: boolean;
    mapRef: MaplibreMap | null;
  }>();

  const emit = defineEmits<{
    'update:open': [value: boolean];
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
    suggestions,
    showLoadingIndicator,
    scrollAnchor,
    stop,
    handleSubmit,
    handlePromptSelect,
    selectModel,
    getIconComponent,
  } = useLlmPanel(computed(() => props.mapRef));
</script>

<template>
  <UiSheet :open="open" @update:open="emit('update:open', $event)">
    <UiSheetContent
      side="right"
      class="flex w-llm-panel flex-col gap-0 border-l border-border/50 p-0 shadow-2xl sm:max-w-llm-panel"
    >
      <LlmPanelHeader
        :engine-status="engineStatus"
        :engine-error="engineError"
        :selected-model="selectedModel"
        :available-models="availableModels"
        @select-model="selectModel"
      />

      <LlmLoadingState
        v-if="engineStatus === 'loading'"
        :stage-text="loadStageText"
        :progress="loadProgress.progress"
        :model-name="selectedModel.name"
        :model-size="selectedModel.sizeGb"
        class="flex-1"
      />

      <UiScrollArea v-else class="flex-1">
        <LlmPanelSuggestions
          v-if="messages.length === 0"
          :suggestions="suggestions"
          :get-icon-component="getIconComponent"
          @prompt-select="handlePromptSelect"
        />
        <LlmMessageList
          v-if="messages.length > 0"
          :messages="messages"
          :is-loading="showLoadingIndicator"
          :is-streaming="isLoading"
        />
        <div ref="scrollAnchor" class="h-0 w-full"></div>
      </UiScrollArea>

      <LlmInput
        v-model="input"
        :is-loading="isLoading"
        :engine-status="engineStatus"
        :has-messages="messages.length > 0"
        @submit="handleSubmit"
        @stop="stop"
      />
    </UiSheetContent>
  </UiSheet>
</template>
