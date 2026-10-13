<script setup lang="ts">
  import { Globe } from '@lucide/vue';

  const { isRemote, destinationHost } = useLlmProvider();
</script>

<template>
  <p
    v-if="isRemote"
    role="note"
    class="flex items-center gap-1.5 border-b border-border/50 bg-muted/30 px-4 py-1.5 text-11 text-muted-foreground"
  >
    <Globe class="size-3 shrink-0" aria-hidden="true" />
    <span class="truncate">Prompts are sent to {{ destinationHost }}</span>
  </p>
</template>
