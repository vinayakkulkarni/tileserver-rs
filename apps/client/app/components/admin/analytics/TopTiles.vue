<script setup lang="ts">
  import type { StatsTopTile } from '~/types';
  import { formatCount } from '~/utils/analytics-format';

  defineProps<{ tiles: StatsTopTile[]; isLoading: boolean }>();
  const SKELETON_ROWS = 5;
</script>

<template>
  <div class="px-page-x py-6 sm:py-8">
    <p class="font-mono text-11 tracking-180 text-muted-foreground uppercase">
      Most requested tiles
    </p>
    <ul
      v-if="isLoading"
      class="mt-4 divide-y divide-border border border-border"
      aria-busy="true"
    >
      <li
        v-for="n in SKELETON_ROWS"
        :key="n"
        class="flex justify-between px-4 py-3"
      >
        <Skeleton class="h-4 w-40" />
        <Skeleton class="h-4 w-10" />
      </li>
    </ul>
    <p
      v-else-if="tiles.length === 0"
      class="mt-4 border border-border px-4 py-8 text-center text-sm text-muted-foreground"
    >
      No tiles requested yet.
    </p>
    <ul
      v-else
      class="mt-4 divide-y divide-border border border-border font-mono text-sm"
    >
      <li
        v-for="t in tiles"
        :key="`${t.source}/${t.z}/${t.x}/${t.y}`"
        class="flex items-baseline justify-between gap-4 px-4 py-2"
      >
        <a
          :href="`/data/${t.source}/${t.z}/${t.x}/${t.y}.pbf`"
          class="truncate text-foreground transition-colors hover:text-primary"
          target="_blank"
          rel="noopener"
          >{{ t.source }}/{{ t.z }}/{{ t.x }}/{{ t.y }}</a
        >
        <span class="shrink-0 tabular-nums text-muted-foreground">{{
          formatCount(t.requests)
        }}</span>
      </li>
    </ul>
  </div>
</template>
