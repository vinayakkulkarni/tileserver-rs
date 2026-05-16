<script setup lang="ts">
  import { Check, Copy } from '@lucide/vue';

  defineProps<{
    code?: string;
    language?: string;
    filename?: string;
    highlights?: number[];
  }>();

  const { copy, copied } = useClipboard();
</script>

<template>
  <div class="group relative">
    <div
      v-if="filename"
      class="flex items-center border-x border-t border-border bg-muted/30 px-4 py-2"
    >
      <span class="font-mono text-xs text-muted-foreground">{{
        filename
      }}</span>
    </div>
    <button
      v-if="code"
      class="absolute top-3 right-3 z-10 flex size-8 items-center justify-center border border-border bg-background text-muted-foreground opacity-0 transition-all hover:text-foreground group-hover:opacity-100"
      :class="{ 'text-foreground opacity-100': copied }"
      aria-label="Copy code"
      @click="copy(code)"
    >
      <Check
        v-if="copied"
        class="size-3.5"
      />
      <Copy
        v-else
        class="size-3.5"
      />
    </button>
    <pre :class="$attrs.class"><slot /></pre>
  </div>
</template>
