<script setup lang="ts">
  import type { StatsFigure } from '~/types';

  defineProps<{ figures: StatsFigure[]; isLoading: boolean }>();
  const SKELETON_CELLS = 4;
</script>

<template>
  <section class="grid grid-cols-2 border-b border-border lg:grid-cols-4">
    <template v-if="isLoading">
      <div
        v-for="n in SKELETON_CELLS"
        :key="n"
        class="space-y-3 border-b border-border px-5 py-5 even:border-l sm:px-6 sm:py-6 lg:border-b-0 lg:border-l lg:first:border-l-0"
        aria-busy="true"
      >
        <Skeleton class="h-3 w-24" />
        <Skeleton class="h-9 w-32" />
        <Skeleton class="h-3 w-40" />
      </div>
    </template>
    <div
      v-for="figure in figures"
      v-else
      :key="figure.label"
      class="border-b border-border px-5 py-5 even:border-l sm:px-6 sm:py-6 lg:border-b-0 lg:border-l lg:first:border-l-0"
    >
      <p class="font-mono text-11 tracking-180 text-muted-foreground uppercase">
        {{ figure.label }}
      </p>
      <p
        class="mt-3 font-display text-2xl font-semibold tabular-nums text-foreground sm:text-3xl"
      >
        {{ figure.value }}
      </p>
      <p class="mt-1 font-mono text-11 text-muted-foreground">
        {{ figure.hint }}
      </p>
    </div>
  </section>
</template>
