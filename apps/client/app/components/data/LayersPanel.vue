<script setup lang="ts">
  import { Eye, EyeOff } from '@lucide/vue';
  import { motion, AnimatePresence } from 'motion-v';
  import type { LayerColor } from '~/types/data';

  defineProps<{
    panelOpen: boolean;
    layerColors: LayerColor[];
  }>();

  const emit = defineEmits<{
    toggleLayerVisibility: [layerId: string];
  }>();
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
          :class="{ 'opacity-40': !layer.visible }"
          @click="emit('toggleLayerVisibility', layer.id)"
        >
          <div
            class="size-3.5 shrink-0"
            :style="{ backgroundColor: layer.color }"
          ></div>
          <span
            class="flex-1 truncate text-left text-muted-foreground"
            :class="{ 'line-through': !layer.visible }"
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
