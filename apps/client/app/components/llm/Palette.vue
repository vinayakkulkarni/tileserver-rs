<script setup lang="ts">
  import {
    Bot,
    GripHorizontal,
    Maximize2,
    Minimize2,
    X,
  } from 'lucide-vue-next';
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

  const isExpanded = computed(() => props.mode === 'expanded');
  const isMinimized = computed(() => props.mode === 'minimized');
</script>

<template>
  <!-- Minimized pill -->
  <div
    v-if="isMinimized"
    ref="panelRef"
    :style="dragStyle"
    class="fixed z-50 flex items-center gap-2 border border-border bg-background/95 px-3 py-2 shadow-lg backdrop-blur-sm transition-shadow hover:shadow-xl"
    :class="{ 'cursor-grabbing': isDragging, 'cursor-grab': !isDragging }"
  >
    <div ref="handleRef" class="flex items-center gap-2">
      <GripHorizontal class="size-3 text-muted-foreground/50" />
      <Bot class="size-3.5 text-primary" />
      <span class="text-xs font-medium text-muted-foreground">Map AI</span>
      <span
        v-if="engineStatus === 'ready'"
        class="size-1.5 rounded-full bg-primary"
      ></span>
    </div>
    <button
      class="flex size-5 items-center justify-center text-muted-foreground transition-colors hover:text-foreground"
      title="Expand"
      @click.stop="expand"
    >
      <Maximize2 class="size-3" />
    </button>
    <button
      class="flex size-5 items-center justify-center text-muted-foreground transition-colors hover:text-destructive"
      title="Close"
      @click.stop="close"
    >
      <X class="size-3" />
    </button>
  </div>

  <!-- Expanded panel -->
  <div
    v-if="isExpanded"
    ref="panelRef"
    :style="dragStyle"
    class="fixed z-50 flex h-[480px] w-[400px] flex-col border border-border bg-background/95 shadow-2xl backdrop-blur-xl"
    :class="{ 'select-none': isDragging }"
  >
    <!-- Drag handle + header -->
    <div
      ref="handleRef"
      class="flex items-center justify-between border-b border-border/50 bg-muted/30 px-3 py-2"
      :class="{ 'cursor-grabbing': isDragging, 'cursor-grab': !isDragging }"
    >
      <div class="flex items-center gap-2">
        <GripHorizontal class="size-3 text-muted-foreground/50" />
        <Bot class="size-3.5 text-muted-foreground" />
        <span class="text-xs font-semibold tracking-tight">Map AI</span>
        <span
          v-if="engineStatus === 'ready'"
          class="inline-flex items-center gap-1 border border-primary/30 bg-primary/10 px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider text-primary"
        >
          <span class="size-1 bg-primary"></span>
          Ready
        </span>
      </div>
      <div class="flex items-center gap-1">
        <LlmModelSelect
          :models="availableModels"
          :selected-id="selectedModel.id"
          :disabled="engineStatus === 'loading'"
          class="w-32"
          @select="selectModel"
        />
        <button
          class="flex size-6 items-center justify-center text-muted-foreground transition-colors hover:text-foreground"
          title="Minimize"
          @click="minimize"
        >
          <Minimize2 class="size-3" />
        </button>
        <button
          class="flex size-6 items-center justify-center text-muted-foreground transition-colors hover:text-destructive"
          title="Close"
          @click="close"
        >
          <X class="size-3" />
        </button>
      </div>
    </div>

    <!-- Engine error -->
    <div
      v-if="engineError"
      class="border-b border-destructive/20 bg-destructive/5 px-4 py-2"
    >
      <p class="text-xs text-destructive">{{ engineError }}</p>
    </div>

    <!-- Loading state -->
    <LlmLoadingState
      v-if="engineStatus === 'loading'"
      :stage-text="loadStageText"
      :progress="loadProgress.progress"
      :model-name="selectedModel.name"
      :model-size="selectedModel.sizeGb"
      class="flex-1"
    />

    <!-- Content: suggestions or messages -->
    <div v-else class="min-h-0 flex-1 overflow-y-auto">
      <div v-if="messages.length === 0" class="flex flex-col gap-3 p-4">
        <p
          class="text-[10px] font-medium uppercase tracking-widest text-muted-foreground/50"
        >
          Try asking
        </p>
        <div class="grid grid-cols-2 gap-2">
          <button
            v-for="s in suggestions"
            :key="s.title"
            class="group flex flex-col gap-1.5 border border-border/50 bg-card p-3 text-left transition-all hover:border-primary/30 hover:bg-primary/5"
            @click="handlePromptSelect(s.prompt)"
          >
            <component
              :is="getIconComponent(s.icon)"
              class="size-3.5 text-muted-foreground transition-colors group-hover:text-primary"
            />
            <span class="text-[11px] font-medium leading-snug">{{
              s.title
            }}</span>
          </button>
        </div>
      </div>
      <LlmMessageList
        v-if="messages.length > 0"
        :messages="messages"
        :is-loading="showLoadingIndicator"
        :is-streaming="isLoading"
      />
      <div ref="scrollAnchor" class="size-0"></div>
    </div>

    <!-- Input -->
    <LlmInput
      v-model="input"
      :is-loading="isLoading"
      :engine-status="engineStatus"
      :has-messages="messages.length > 0"
      @submit="handleSubmit"
      @stop="stop"
    />
  </div>
</template>
