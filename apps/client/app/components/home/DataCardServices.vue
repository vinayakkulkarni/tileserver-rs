<script setup lang="ts">
  import { Check, Copy } from '@lucide/vue';
  import type { Data } from '~/types/data';

  const props = defineProps<{
    source: Data;
    baseUrl: string;
    isXyzExpanded: boolean;
    copiedUrl: string | null;
  }>();

  const emit = defineEmits<{
    'toggle-xyz': [];
    'copy-url': [url: string];
  }>();

  const xyzUrl = computed(
    () => `${props.baseUrl}/data/${props.source.id}/{z}/{x}/{y}.pbf`,
  );
</script>

<template>
  <div class="mt-3 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs">
    <span
      class="lbl font-mono text-10-5 tracking-widest uppercase text-muted-foreground font-medium"
      >Services:</span
    >
    <a
      :href="`/data/${source.id}.json`"
      target="_blank"
      rel="noopener noreferrer"
      class="services-link text-primary font-medium transition-colors duration-(--d-fast)"
      >TileJSON</a
    >
    <span class="sep text-muted-foreground opacity-50">·</span>
    <button
      type="button"
      class="services-link text-primary font-medium transition-colors duration-(--d-fast)"
      @click="emit('toggle-xyz')"
    >
      XYZ URL
    </button>
  </div>

  <div v-if="isXyzExpanded" class="mt-2 flex items-center gap-2 bg-muted p-2">
    <code class="flex-1 truncate text-xs text-muted-foreground font-mono">{{
      xyzUrl
    }}</code>
    <Button
      variant="ghost"
      size="icon"
      class="size-7 shrink-0"
      aria-label="Copy XYZ URL"
      @click="emit('copy-url', xyzUrl)"
    >
      <Check v-if="copiedUrl === xyzUrl" class="size-3.5 text-success" />
      <Copy v-else class="size-3.5" />
    </Button>
  </div>
</template>
