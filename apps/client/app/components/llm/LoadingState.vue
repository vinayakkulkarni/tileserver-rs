<script setup lang="ts">
  import { BrainCircuit } from '@lucide/vue';

  defineProps<{
    stageText: string;
    progress: number;
    modelName: string;
    modelSize: number;
  }>();
</script>

<template>
  <div class="flex flex-col items-center justify-center px-6 py-12">
    <!-- Pulsing icon -->
    <div class="relative mb-6">
      <span class="absolute inset-0 animate-ping bg-primary/15"></span>
      <div class="relative bg-primary/10 p-4">
        <BrainCircuit class="size-6 text-primary" />
      </div>
    </div>

    <!-- Model info -->
    <p class="mb-1 font-display text-sm font-semibold tracking-tight">
      Loading AI Model
    </p>
    <p class="mb-6 text-[11px] text-muted-foreground">
      {{ modelName }} · {{ modelSize }} GB
    </p>

    <!-- Progress bar -->
    <div class="w-full max-w-[220px]">
      <div class="relative h-1 overflow-hidden bg-muted">
        <div
          class="absolute inset-y-0 left-0 bg-primary transition-[width] duration-500 ease-out"
          :style="{ width: `${Math.max(progress * 100, 2)}%` }"
        ></div>
      </div>
      <div class="mt-2.5 flex items-center justify-between">
        <span
          class="text-[10px] uppercase tracking-wider text-muted-foreground"
          >{{ stageText }}</span
        >
        <span class="text-[10px] font-semibold tabular-nums text-foreground">
          {{ Math.round(progress * 100) }}%
        </span>
      </div>
    </div>

    <!-- First-load hint -->
    <p
      class="mt-8 max-w-[200px] text-center text-[10px] leading-relaxed text-muted-foreground/40"
    >
      First load downloads the model. Future loads are near-instant.
    </p>
  </div>
</template>
