<script setup lang="ts">
  import type { CostLine } from '~/types';

  defineProps<{
    lines: CostLine[];
    formatMoney: (value: number) => string;
    formatCount: (value: number) => string;
  }>();
</script>

<template>
  <details class="group mt-3">
    <summary
      class="cursor-pointer list-none font-mono text-10 uppercase tracking-200 text-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:text-foreground [&::-webkit-details-marker]:hidden"
    >
      <span class="group-open:hidden">+ Show cost breakdown</span>
      <span class="hidden group-open:inline">− Hide cost breakdown</span>
    </summary>
    <ul class="mt-3 divide-y divide-border border border-border">
      <li
        v-for="line in lines"
        :key="line.label"
        class="grid gap-1 px-3 py-2 font-mono text-xs sm:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_minmax(0,0.6fr)] sm:gap-4"
      >
        <a
          :href="line.source.url"
          target="_blank"
          rel="noopener"
          class="font-sans text-foreground underline decoration-border underline-offset-2 transition-colors hover:decoration-foreground"
        >{{ line.label }}</a>
        <span class="text-muted-foreground">{{ formatCount(line.units) }} {{ line.unit }}</span>
        <span class="text-foreground sm:text-right">{{
          formatMoney(line.cost)
        }}</span>
      </li>
    </ul>
  </details>
</template>
