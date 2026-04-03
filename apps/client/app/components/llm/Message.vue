<script setup lang="ts">
  import { Bot, User } from 'lucide-vue-next';
  import {
    MarkdownRender,
    getMarkdown,
    parseMarkdownToStructure,
    type ParsedNode,
  } from 'markstream-vue';
  import type { ReadonlyUIMessage } from '~/types/llm';
  import {
    getTextContent,
    formatMessageTime,
  } from '~/composables/use-llm-panel';

  const props = defineProps<{
    message: ReadonlyUIMessage;
    isStreaming: boolean;
  }>();

  const md = getMarkdown();

  const textContent = computed(() => getTextContent(props.message.parts));

  const parsedNodes = computed<ParsedNode[]>(() => {
    const content = textContent.value;
    if (!content) return [];
    return parseMarkdownToStructure(content, md, {
      final: !props.isStreaming,
    });
  });
</script>

<template>
  <div
    class="flex gap-2.5"
    :class="message.role === 'user' ? 'flex-row-reverse' : 'flex-row'"
  >
    <!-- Avatar -->
    <div
      class="flex size-7 shrink-0 items-center justify-center"
      :class="
        message.role === 'user'
          ? 'bg-primary text-primary-foreground'
          : 'bg-muted text-muted-foreground'
      "
    >
      <User v-if="message.role === 'user'" class="size-3.5" />
      <Bot v-else class="size-3.5" />
    </div>

    <!-- Bubble -->
    <div
      class="max-w-[80%] px-3 py-2 text-sm leading-relaxed"
      :class="
        message.role === 'user'
          ? 'bg-primary text-primary-foreground'
          : 'border border-border/40 bg-muted/50 text-foreground'
      "
    >
      <!-- User: plain text -->
      <p v-if="message.role === 'user'" class="whitespace-pre-wrap">
        {{ textContent }}
      </p>

      <!-- Assistant: markdown rendered -->
      <ClientOnly v-else>
        <div class="ai-prose max-w-none overflow-hidden">
          <MarkdownRender
            :nodes="parsedNodes"
            :max-live-nodes="0"
            :batch-rendering="true"
            :render-batch-size="16"
            :render-batch-delay="8"
            :final="!isStreaming"
            class="text-sm/relaxed"
          />
        </div>
        <template #fallback>
          <p class="whitespace-pre-wrap">{{ textContent }}</p>
        </template>
      </ClientOnly>

      <span v-if="message.createdAt" class="mt-1 block text-[10px] opacity-50">
        {{ formatMessageTime(message.createdAt) }}
      </span>
    </div>
  </div>
</template>
