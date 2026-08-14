<script setup lang="ts">
  import { Eye, EyeOff } from '@lucide/vue';
  import { motion, AnimatePresence } from 'motion-v';
  import type { LayerColor } from '~/types/data';

  const props = defineProps<{
    panelOpen: boolean;
    layerColors: LayerColor[];
  }>();

  const emit = defineEmits<{
    toggleLayerVisibility: [layerId: string];
  }>();

  // Stable per-layer refs so the inline class/style objects are not recreated
  // on every render inside the v-for subtree.
  const rowClassMap = computed(() =>
    new Map(
      props.layerColors.map((layer) => [
        layer.id,
        { 'opacity-40': !layer.visible },
      ]),
    ),
  );
  const dotStyleMap = computed(() =>
    new Map(
      props.layerColors.map((layer) => [
        layer.id,
        { backgroundColor: layer.color },
      ]),
    ),
  );
  const labelClassMap = computed(() =>
    new Map(
      props.layerColors.map((layer) => [
        layer.id,
        { 'line-through': !layer.visible },
      ]),
    ),
  );
</script>

<template>
  <AnimatePresence>
    <motion.div
      v-if="panelOpen"
      :initial="{ opacity: 0, x: 20, scale: 0.95 }"
      :animate="{ opacity: 1, x: 0, scale: 1 }"
      :exit="{ opacity: 0, x: 20, scale: 0.95 }"
      :transition="{ type: 'spring', stiffness: 300, damping: 25 }"
      class="absolute top-16 right-4 z-10 w-56 border border-border bg-background p-4 shadow-sm"
    >
      <h3 class="mb-3 text-sm font-semibold">Layers</h3>
      <div class="space-y-1">
        <button
          v-for="layer in layerColors"
          :key="layer.id"
          class="flex w-full items-center gap-2 px-1.5 py-1 text-sm transition-colors hover:bg-accent"
          :class="rowClassMap.get(layer.id)"
          @click="emit('toggleLayerVisibility', layer.id)"
        >
          <div
            class="size-3.5 shrink-0"
            :style="dotStyleMap.get(layer.id)"
          ></div>
          <span
            class="flex-1 truncate text-left text-muted-foreground"
            :class="labelClassMap.get(layer.id)"
          >
            {{ layer.id }}
          </span>
          <Eye
            v-if="layer.visible"
            class="size-3.5 shrink-0 text-muted-foreground"
          />
          <EyeOff v-else class="size-3.5 shrink-0 text-muted-foreground" />
        </button>
        <div
          v-if="layerColors.length === 0"
          class="px-1.5 py-1 text-sm text-muted-foreground"
        >
          Loading layers...
        </div>
      </div>
    </motion.div>
  </AnimatePresence>
</template>
