<script setup lang="ts">
  import type { StatsOutcome, StatsRecentRequest } from '~/types';
  import { formatBytes, formatClock, formatMs } from '~/utils/analytics-format';

  defineProps<{ recent: StatsRecentRequest[]; isLoading: boolean }>();
  const SKELETON_ROWS = 5;
  const OUTCOME_CLASS: Record<StatsOutcome, string> = {
    hit: 'text-success',
    miss: 'text-foreground',
    not_found: 'text-muted-foreground',
    error: 'text-destructive',
  };
</script>

<template>
  <div class="px-page-x py-6 sm:py-8">
    <p class="font-mono text-11 tracking-180 text-muted-foreground uppercase">
      Recent requests
    </p>
    <ul
      v-if="isLoading"
      class="mt-4 divide-y divide-border border border-border"
      aria-busy="true"
    >
      <li v-for="n in SKELETON_ROWS" :key="n" class="px-4 py-3">
        <Skeleton class="h-4 w-full" />
      </li>
    </ul>
    <p
      v-else-if="recent.length === 0"
      class="mt-4 border border-border px-4 py-8 text-center text-sm text-muted-foreground"
    >
      No requests yet.
    </p>
    <ul
      v-else
      class="mt-4 max-h-96 divide-y divide-border overflow-y-auto border border-border font-mono text-11"
    >
      <li
        v-for="r in recent"
        :key="`${r.at_unix_ms}-${r.source}-${r.z}/${r.x}/${r.y}`"
        class="grid grid-cols-2 gap-x-3 px-4 py-2 sm:grid-cols-6"
      >
        <span class="text-muted-foreground">{{
          formatClock(r.at_unix_ms)
        }}</span>
        <span class="truncate text-foreground sm:col-span-2">
          {{ r.source }}/{{ r.z }}/{{ r.x }}/{{ r.y }}
        </span>
        <span :class="OUTCOME_CLASS[r.outcome]">{{ r.outcome }}</span>
        <span
          class="whitespace-nowrap text-muted-foreground tabular-nums sm:col-span-2 sm:text-right"
        >
          {{ formatMs(r.duration_ms) }} · {{ formatBytes(r.bytes) }}
        </span>
      </li>
    </ul>
  </div>
</template>
