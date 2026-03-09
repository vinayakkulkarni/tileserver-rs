<script setup lang="ts">
  import { Upload, AlertCircle, CheckCircle2, Loader2 } from 'lucide-vue-next';
  import type { FileDropError, FileDropSuccess, FileDropStatus } from '~/types/file-upload';

  const props = defineProps<{
    status: FileDropStatus;
    isOver: boolean;
    error: FileDropError | null;
    success: FileDropSuccess | null;
  }>();

  const showOverlay = computed(() => props.isOver || props.status === 'processing' || props.status === 'uploading');
  const showError = computed(() => props.error !== null && props.status === 'idle');
  const showSuccess = computed(() => props.success !== null && props.status === 'idle');
</script>

<template>
  <!-- Drag-over overlay -->
  <Transition name="fade">
    <div
      v-if="showOverlay"
      class="pointer-events-none absolute inset-0 z-50 flex items-center justify-center bg-background/60 backdrop-blur-sm"
    >
      <div
        class="flex flex-col items-center gap-3 border-2 border-dashed border-primary/50 bg-background/90 px-12 py-10 shadow-2xl"
      >
        <Loader2 v-if="status === 'processing' || status === 'uploading'" class="size-10 animate-spin text-primary" />
        <Upload v-else class="size-10 text-primary" />
        <p class="text-lg font-medium">
          {{ status === 'uploading' ? 'Uploading to server…' : status === 'processing' ? 'Processing file…' : 'Drop file to visualize' }}
        </p>
        <p class="text-sm text-muted-foreground">
          GeoJSON, KML, GPX, CSV, Shapefile, PMTiles, MBTiles, SQLite, COG
        </p>
      </div>
    </div>
  </Transition>

  <!-- Error toast -->
  <Transition name="slide-up">
    <div
      v-if="showError"
      class="absolute bottom-6 left-1/2 z-50 flex max-w-md -translate-x-1/2 items-start gap-3 border border-destructive/30 bg-destructive/10 px-4 py-3 shadow-lg backdrop-blur-sm"
    >
      <AlertCircle class="mt-0.5 size-4 shrink-0 text-destructive" />
      <div class="min-w-0">
        <p class="text-sm font-medium text-destructive">
          Failed to load {{ error?.fileName }}
        </p>
        <p class="mt-0.5 text-xs text-muted-foreground">
          {{ error?.message }}
        </p>
      </div>
    </div>
  </Transition>

  <!-- Success toast -->
  <Transition name="slide-up">
    <div
      v-if="showSuccess"
      class="absolute bottom-6 left-1/2 z-50 flex max-w-md -translate-x-1/2 items-start gap-3 border border-emerald-500/30 bg-emerald-500/10 px-4 py-3 shadow-lg backdrop-blur-sm"
    >
      <CheckCircle2 class="mt-0.5 size-4 shrink-0 text-emerald-600 dark:text-emerald-400" />
      <div class="min-w-0">
        <p class="text-sm font-medium text-emerald-700 dark:text-emerald-300">
          Loaded {{ success?.fileName }}
        </p>
        <p class="mt-0.5 text-xs text-muted-foreground">
          {{ success?.featureCount ? `${success.featureCount.toLocaleString()} features` : success?.format?.toUpperCase() }} added as overlay
        </p>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
  .fade-enter-active,
  .fade-leave-active {
    transition: opacity 150ms ease;
  }

  .fade-enter-from,
  .fade-leave-to {
    opacity: 0;
  }

  .slide-up-enter-active,
  .slide-up-leave-active {
    transition: all 200ms ease;
  }

  .slide-up-enter-from,
  .slide-up-leave-to {
    opacity: 0;
    transform: translate(-50%, 1rem);
  }
</style>
