<script setup lang="ts">
  import { GripHorizontal, Bot, Minimize2, X } from '@lucide/vue';
  import type { LlmEngineStatus, LlmModelConfig } from '~/types/llm';

  defineProps<{
    isDragging: boolean;
    engineStatus: LlmEngineStatus;
    selectedModel: LlmModelConfig;
    availableModels: LlmModelConfig[];
    setHandleRef: (el: Element | null) => void;
  }>();

  const emit = defineEmits<{
    minimize: [];
    close: [];
    selectModel: [modelId: string];
  }>();
</script>

<template>
  <div
    :ref="setHandleRef"
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
        @select="emit('selectModel', $event)"
      />
      <button
        class="flex size-6 items-center justify-center text-muted-foreground transition-colors hover:text-foreground"
        title="Minimize"
        @click="emit('minimize')"
      >
        <Minimize2 class="size-3" />
      </button>
      <button
        class="flex size-6 items-center justify-center text-muted-foreground transition-colors hover:text-destructive"
        title="Close"
        @click="emit('close')"
      >
        <X class="size-3" />
      </button>
    </div>
  </div>
</template>
