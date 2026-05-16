<script setup lang="ts">
  import { AlertCircle, CheckCircle2 } from '@lucide/vue';
  import type { FileDropError, FileDropSuccess } from '~/types/file-upload';

  defineProps<{
    showError: boolean;
    showSuccess: boolean;
    error: FileDropError | null;
    success: FileDropSuccess | null;
  }>();
</script>

<template>
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

  <Transition name="slide-up">
    <div
      v-if="showSuccess"
      class="absolute bottom-6 left-1/2 z-50 flex max-w-md -translate-x-1/2 items-start gap-3 border border-emerald-500/30 bg-emerald-500/10 px-4 py-3 shadow-lg backdrop-blur-sm"
    >
      <CheckCircle2
        class="mt-0.5 size-4 shrink-0 text-emerald-600 dark:text-emerald-400"
      />
      <div class="min-w-0">
        <p class="text-sm font-medium text-emerald-700 dark:text-emerald-300">
          Loaded {{ success?.fileName }}
        </p>
        <p class="mt-0.5 text-xs text-muted-foreground">
          {{
            success?.featureCount
              ? `${success.featureCount.toLocaleString()} features`
              : success?.format?.toUpperCase()
          }}
          added as overlay
        </p>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
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
