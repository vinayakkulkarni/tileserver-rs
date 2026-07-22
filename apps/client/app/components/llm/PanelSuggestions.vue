<script setup lang="ts">
  import type { SuggestedPrompt } from '~/types/llm';

  defineProps<{
    suggestions: SuggestedPrompt[];
    getIconComponent: (icon: string) => unknown;
  }>();

  const emit = defineEmits<{
    promptSelect: [prompt: string];
  }>();
</script>

<template>
  <div class="flex flex-col gap-3 p-5">
    <p
      class="text-11 font-medium uppercase tracking-widest text-muted-foreground/60"
    >
      Suggestions
    </p>
    <div class="grid grid-cols-2 gap-2">
      <button
        v-for="suggestion in suggestions"
        :key="suggestion.title"
        class="group flex flex-col gap-2 border border-border/60 bg-card p-3.5 text-left transition-all duration-200 hover:border-primary/40 hover:bg-primary/5 hover:shadow-sm dark:hover:bg-primary/10"
        @click="emit('promptSelect', suggestion.prompt)"
      >
        <component
          :is="getIconComponent(suggestion.icon)"
          class="size-4 text-muted-foreground transition-colors group-hover:text-primary"
        />
        <span class="text-xs/snug font-medium">{{
          suggestion.title
        }}</span>
      </button>
    </div>
  </div>
</template>
