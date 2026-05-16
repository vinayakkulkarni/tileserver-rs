<script setup lang="ts">
  import { GripHorizontal, Bot, Maximize2, X } from '@lucide/vue';
  import type { LlmEngineStatus } from '~/types/llm';

  defineProps<{
    engineStatus: LlmEngineStatus;
    isDragging: boolean;
    setHandleRef: (el: Element | null) => void;
  }>();

  const emit = defineEmits<{
    expand: [];
    close: [];
  }>();
</script>

<template>
  <div :ref="setHandleRef" class="flex items-center gap-2">
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
    @click.stop="emit('expand')"
  >
    <Maximize2 class="size-3" />
  </button>
  <button
    class="flex size-5 items-center justify-center text-muted-foreground transition-colors hover:text-destructive"
    title="Close"
    @click.stop="emit('close')"
  >
    <X class="size-3" />
  </button>
</template>
