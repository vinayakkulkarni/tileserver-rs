<script setup lang="ts">
  import {
    SheetHeader as UiSheetHeader,
    SheetTitle as UiSheetTitle,
    SheetDescription as UiSheetDescription,
  } from '~/components/ui/sheet';
  import { Bot } from '@lucide/vue';
  import type { LlmEngineStatus, LlmModelConfig } from '~/types/llm';

  defineProps<{
    engineStatus: LlmEngineStatus;
    engineError: string | null;
    selectedModel: LlmModelConfig;
    availableModels: LlmModelConfig[];
  }>();

  const emit = defineEmits<{
    selectModel: [modelId: string];
  }>();
</script>

<template>
  <UiSheetHeader
    class="space-y-0 border-b border-border/50 bg-muted/30 px-5 py-4"
  >
    <div class="flex items-center justify-between pr-10">
      <UiSheetTitle
        class="flex items-center gap-2.5 font-display text-base font-semibold tracking-tight"
      >
        <div
          class="flex size-7 items-center justify-center bg-primary text-primary-foreground"
        >
          <Bot class="size-4" />
        </div>
        Map Assistant
      </UiSheetTitle>
      <span
        v-if="engineStatus === 'ready'"
        class="inline-flex items-center gap-1 border border-primary/30 bg-primary/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-primary"
      >
        <span class="size-1.5 bg-primary"></span>
        Ready
      </span>
    </div>
    <UiSheetDescription class="mt-2 text-xs">
      <LlmModelSelect
        :models="availableModels"
        :selected-id="selectedModel.id"
        :disabled="engineStatus === 'loading'"
        @select="emit('selectModel', $event)"
      />
    </UiSheetDescription>
  </UiSheetHeader>

  <div
    v-if="engineError"
    class="border-b border-destructive/20 bg-destructive/5 px-5 py-3"
  >
    <p class="text-xs font-medium text-destructive">{{ engineError }}</p>
  </div>
</template>
