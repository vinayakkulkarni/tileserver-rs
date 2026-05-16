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
    toggleXyz: [];
    copyUrl: [url: string];
  }>();

  const xyzUrl = computed(
    () => `${props.baseUrl}/data/${props.source.id}/{z}/{x}/{y}.pbf`,
  );
</script>

<template>
  <template v-if="source.vector_layers?.length">
    <div class="mt-3 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs">
      <span class="text-muted-foreground">Services:</span>
      <a
        :href="`/data/${source.id}.json`"
        target="_blank"
        class="text-primary hover:underline"
        >TileJSON</a
      >
      <span class="text-muted-foreground/30">•</span>
      <button class="text-primary hover:underline" @click="emit('toggleXyz')">
        XYZ URL
      </button>
    </div>

    <div
      v-if="isXyzExpanded"
      class="mt-2 flex items-center gap-2 rounded-lg bg-muted/50 p-2"
    >
      <code class="flex-1 truncate text-xs text-muted-foreground">{{
        xyzUrl
      }}</code>
      <Button
        variant="ghost"
        size="icon"
        class="size-7 shrink-0 rounded-lg"
        @click="emit('copyUrl', xyzUrl)"
      >
        <Check v-if="copiedUrl === xyzUrl" class="size-3.5 text-green-500" />
        <Copy v-else class="size-3.5" />
      </Button>
    </div>
  </template>
</template>
