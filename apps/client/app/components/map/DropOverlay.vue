<script setup lang="ts">
  import { Upload, Loader2 } from '@lucide/vue';
  import type {
    FileDropError,
    FileDropSuccess,
    FileDropStatus,
  } from '~/types/file-upload';

  const props = defineProps<{
    status: FileDropStatus;
    isOver: boolean;
    error: FileDropError | null;
    success: FileDropSuccess | null;
  }>();

  const showOverlay = computed(
    () =>
      props.isOver ||
      props.status === 'processing' ||
      props.status === 'uploading',
  );
  const showError = computed(
    () => props.error !== null && props.status === 'idle',
  );
  const showSuccess = computed(
    () => props.success !== null && props.status === 'idle',
  );
</script>

<template>
  <Transition name="fade">
    <div
      v-if="showOverlay"
      class="pointer-events-none absolute inset-0 z-50 flex items-center justify-center bg-background/60 backdrop-blur-sm"
    >
      <div
        class="flex flex-col items-center gap-3 border-2 border-dashed border-primary/50 bg-background/90 px-12 py-10 shadow-2xl"
      >
        <Loader2
          v-if="status === 'processing' || status === 'uploading'"
          class="size-10 animate-spin text-primary"
        />
        <Upload v-else class="size-10 text-primary" />
        <p class="text-lg font-medium">
          {{
            status === 'uploading'
              ? 'Uploading to server…'
              : status === 'processing'
                ? 'Processing file…'
                : 'Drop file to visualize'
          }}
        </p>
        <p class="text-sm text-muted-foreground">
          GeoJSON, KML, GPX, CSV, Shapefile, PMTiles, MBTiles, SQLite, COG,
          GeoParquet
        </p>
      </div>
    </div>
  </Transition>

  <MapDropOverlayToasts
    :show-error="showError"
    :show-success="showSuccess"
    :error="error"
    :success="success"
  />
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
</style>
