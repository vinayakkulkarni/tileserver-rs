<script setup lang="ts">
  import type {
    LlmEngineStatus,
    LlmModelConfig,
    LlmLoadProgress,
    SuggestedPrompt,
    ReadonlyUIMessage,
  } from '~/types/llm';

  const props = defineProps<{
    engineError: string | null;
    engineStatus: LlmEngineStatus;
    loadStageText: string;
    loadProgress: LlmLoadProgress;
    selectedModel: LlmModelConfig;
    messages: readonly ReadonlyUIMessage[];
    showLoadingIndicator: boolean;
    isLoading: boolean;
    suggestions: SuggestedPrompt[];
    input: string;
    getIconComponent: (icon: string) => unknown;
  }>();

  const emit = defineEmits<{
    'update:input': [value: string];
    submit: [];
    stop: [];
    promptSelect: [prompt: string];
    anchorMounted: [el: HTMLElement | null];
  }>();

  const anchorEl = ref<HTMLElement | null>(null);

  watch(anchorEl, (el) => emit('anchorMounted', el), { immediate: true });
</script>

<template>
  <div
    v-if="engineError"
    class="border-b border-destructive/20 bg-destructive/5 px-4 py-2"
  >
    <p class="text-xs text-destructive">{{ engineError }}</p>
  </div>

  <LlmLoadingState
    v-if="engineStatus === 'loading'"
    :stage-text="loadStageText"
    :progress="loadProgress.progress"
    :model-name="selectedModel.name"
    :model-size="selectedModel.sizeGb"
    class="flex-1"
  />

  <div v-else class="min-h-0 flex-1 overflow-y-auto">
    <div v-if="messages.length === 0" class="flex flex-col gap-3 p-4">
      <p
        class="text-10 font-medium uppercase tracking-widest text-muted-foreground/50"
      >
        Try asking
      </p>
      <div class="grid grid-cols-2 gap-2">
        <button
          v-for="s in suggestions"
          :key="s.title"
          class="group flex flex-col gap-1.5 border border-border/50 bg-card p-3 text-left transition-all hover:border-primary/30 hover:bg-primary/5"
          @click="emit('promptSelect', s.prompt)"
        >
          <component
            :is="getIconComponent(s.icon)"
            class="size-3.5 text-muted-foreground transition-colors group-hover:text-primary"
          />
          <span class="text-11/snug font-medium">{{
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
    <div ref="anchorEl" class="size-0"></div>
  </div>

  <LlmInput
    :model-value="props.input"
    :is-loading="isLoading"
    :engine-status="engineStatus"
    :has-messages="messages.length > 0"
    @update:model-value="emit('update:input', $event)"
    @submit="emit('submit')"
    @stop="emit('stop')"
  />
</template>
