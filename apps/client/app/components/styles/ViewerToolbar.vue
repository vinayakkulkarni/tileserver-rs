<script setup lang="ts">
  import { ArrowLeft, Palette, Sparkles } from '@lucide/vue';

  defineProps<{
    styleId: string;
    isScreenshot: boolean;
    isChatVisible: boolean;
  }>();

  const emit = defineEmits<{
    navigateBack: [];
    toggleChat: [];
  }>();
</script>

<template>
  <button
    v-if="!isScreenshot"
    class="absolute top-4 left-4 z-10 flex items-center gap-2 border border-border bg-background px-3 py-2 text-sm font-medium shadow-sm transition-colors hover:bg-accent"
    @click="emit('navigateBack')"
  >
    <ArrowLeft class="size-4" />
    <Palette class="size-4" />
    <span>{{ styleId }}</span>
  </button>

  <button
    v-if="!isScreenshot && !isChatVisible"
    class="absolute bottom-6 left-1/2 z-10 flex -translate-x-1/2 items-center gap-3 border border-border bg-background/95 px-5 py-2.5 shadow-lg backdrop-blur-sm transition-all hover:border-primary/50 hover:shadow-xl"
    @click="emit('toggleChat')"
  >
    <Sparkles class="size-4 text-primary" />
    <span class="text-sm text-muted-foreground">Ask about the map…</span>
    <kbd
      class="border border-border/60 bg-muted/50 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground"
      >⌘K</kbd
    >
  </button>
</template>
