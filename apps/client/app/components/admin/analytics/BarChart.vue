<script setup lang="ts">
  import type { StatsBar } from '~/types';

  defineProps<{
    title: string;
    caption: string;
    bars: StatsBar[];
    isLoading: boolean;
    showLabels?: boolean;
  }>();
  const SKELETON_BARS = 12;
</script>

<template>
  <div class="px-page-x py-6 sm:py-8">
    <div class="flex items-baseline justify-between gap-4">
      <p class="font-mono text-11 tracking-180 text-muted-foreground uppercase">
        {{ title }}
      </p>
      <p class="font-mono text-11 text-muted-foreground">{{ caption }}</p>
    </div>
    <div
      v-if="isLoading"
      class="mt-4 flex h-32 items-end gap-1"
      aria-busy="true"
    >
      <Skeleton v-for="n in SKELETON_BARS" :key="n" class="h-full flex-1" />
    </div>
    <p
      v-else-if="bars.every((b) => b.value === 0)"
      class="mt-4 flex h-32 items-center justify-center border border-dashed border-border text-sm text-muted-foreground"
    >
      No requests yet
    </p>
    <div v-else class="mt-4">
      <div class="flex h-32 items-end gap-px border-b border-border">
        <div
          v-for="bar in bars"
          :key="bar.key"
          :title="bar.title"
          :aria-label="bar.title"
          class="flex-1 bg-primary/70 transition-colors hover:bg-primary"
          :style="bar.style"
        ></div>
      </div>
      <div
        v-if="showLabels"
        class="mt-1 flex gap-px font-mono text-10 text-muted-foreground"
      >
        <span
          v-for="bar in bars"
          :key="bar.key"
          class="flex-1 truncate text-center"
          >{{ bar.label }}</span
        >
      </div>
    </div>
  </div>
</template>
