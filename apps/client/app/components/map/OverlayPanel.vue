<script setup lang="ts">
  import { Eye, EyeOff, Trash2, Layers, X } from 'lucide-vue-next';
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

  function formatCount(count: number): string {
    if (count === 0) return 'tiles';
    return count === 1 ? '1 feature' : `${count.toLocaleString()} features`;
  }

  function formatBadge(format: string): string {
    return format.toUpperCase();
  }
</script>

<template>
  <div
    v-if="overlays.length > 0"
    class="absolute top-4 right-4 z-10 w-64 border border-border bg-background/95 shadow-lg backdrop-blur-sm"
  >
    <!-- Header -->
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

    <!-- Layer list -->
    <div v-if="expanded" class="border-t border-border">
      <UiScrollArea class="max-h-64">
        <div class="divide-y divide-border">
          <div
            v-for="overlay in overlays"
            :key="overlay.id"
            class="flex items-center gap-2 px-3 py-2"
          >
            <!-- Color indicator -->
            <span
              class="size-3 shrink-0 rounded-full"
              :style="{ backgroundColor: overlay.color }"
            ></span>

            <!-- File info -->
            <div class="min-w-0 flex-1">
              <p class="truncate text-xs font-medium" :title="overlay.fileName">
                {{ overlay.fileName }}
              </p>
              <p class="text-[10px] text-muted-foreground">
                <span class="border border-border/60 px-1 py-px text-[9px]">
                  {{ formatBadge(overlay.format) }}
                </span>
                {{ ' ' }}
                {{ formatCount(overlay.featureCount) }}
              </p>
            </div>

            <!-- Actions -->
            <button
              class="shrink-0 p-0.5 text-muted-foreground transition-colors hover:text-foreground"
              :title="overlay.visible ? 'Hide layer' : 'Show layer'"
              @click="emit('toggle', overlay.id)"
            >
              <Eye v-if="overlay.visible" class="size-3.5" />
              <EyeOff v-else class="size-3.5" />
            </button>
            <button
              class="shrink-0 p-0.5 text-muted-foreground transition-colors hover:text-destructive"
              title="Remove layer"
              @click="emit('remove', overlay.id)"
            >
              <Trash2 class="size-3.5" />
            </button>
          </div>
        </div>
      </UiScrollArea>

      <!-- Remove all -->
      <div v-if="overlays.length > 1" class="border-t border-border px-3 py-1.5">
        <button
          class="flex w-full items-center justify-center gap-1.5 py-1 text-xs text-muted-foreground transition-colors hover:text-destructive"
          @click="emit('removeAll')"
        >
          <X class="size-3" />
          Remove all
        </button>
      </div>
    </div>
  </div>
</template>
