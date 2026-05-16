<script setup lang="ts">
  import { Bot } from '@lucide/vue';
  import type { ReadonlyUIMessage } from '~/types/llm';

  defineProps<{
    messages: readonly ReadonlyUIMessage[];
    isLoading: boolean;
    isStreaming: boolean;
  }>();
</script>

<template>
  <div class="flex flex-1 flex-col gap-3 p-4">
    <!-- Empty state -->
    <div
      v-if="messages.length === 0 && !isLoading"
      class="flex flex-1 items-center justify-center text-sm text-muted-foreground"
    >
      Start a conversation about the map
    </div>

    <!-- Messages -->
    <LlmMessage
      v-for="message in messages"
      :key="message.id"
      :message="message"
      :is-streaming="
        isStreaming &&
        message.id === messages[messages.length - 1]?.id &&
        message.role === 'assistant'
      "
    />

    <!-- Loading indicator -->
    <div v-if="isLoading" class="flex gap-2.5">
      <div
        class="flex size-7 shrink-0 items-center justify-center bg-muted text-muted-foreground"
      >
        <Bot class="size-3.5" />
      </div>
      <div class="border border-border/40 bg-muted/50 px-3 py-2">
        <span class="flex gap-1">
          <span
            class="size-1.5 animate-bounce bg-muted-foreground/50 [animation-delay:-0.3s]"
          ></span>
          <span
            class="size-1.5 animate-bounce bg-muted-foreground/50 [animation-delay:-0.15s]"
          ></span>
          <span class="size-1.5 animate-bounce bg-muted-foreground/50"></span>
        </span>
      </div>
    </div>
  </div>
</template>
