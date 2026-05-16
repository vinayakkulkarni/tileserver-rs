<script setup lang="ts">
  import { Layers } from '@lucide/vue';
  import type { OverlayLayer } from '~/types/file-upload';

  defineProps<{
    overlays: OverlayLayer[];
  }>();

  const emit = defineEmits<{
    toggle: [overlayId: string];
    remove: [overlayId: string];
    removeAll: [];
  }>();

  const expanded = ref(true);

  function togglePanel() {
    expanded.value = !expanded.value;
  }

  function handleToggle(id: string) {
    emit('toggle', id);
  }

  function handleRemove(id: string) {
    emit('remove', id);
  }

  function handleRemoveAll() {
    emit('removeAll');
  }
</script>

<template>
  <div
    v-if="overlays.length > 0"
    class="absolute top-4 right-4 z-10 w-64 border border-border bg-background/95 shadow-lg backdrop-blur-sm"
  >
    <button
      class="flex w-full items-center justify-between px-3 py-2 text-sm font-medium transition-colors hover:bg-accent"
      @click="togglePanel"
    >
      <span class="flex items-center gap-2">
        <Layers class="size-4" />
        <span>Overlays ({{ overlays.length }})</span>
      </span>
      <span class="text-xs text-muted-foreground">
        {{ expanded ? '▾' : '▸' }}
      </span>
    </button>
    <MapOverlayPanelList
      v-if="expanded"
      :overlays="overlays"
      @toggle="handleToggle"
      @remove="handleRemove"
      @remove-all="handleRemoveAll"
    />
  </div>
</template>
