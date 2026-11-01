<script setup lang="ts">
  import type { StatsSource } from '~/types';
  import {
    formatBytes,
    formatCount,
    formatMs,
    formatPercent,
  } from '~/utils/analytics-format';

  defineProps<{ sources: StatsSource[]; isLoading: boolean }>();
  const SKELETON_ROWS = 3;
  const HEADERS = [
    'Source',
    'Requests',
    'Hit %',
    'p50',
    'p95',
    'Avg size',
    'Errors',
  ];
</script>

<template>
  <div class="px-page-x py-6 sm:py-8">
    <p class="font-mono text-11 tracking-180 text-muted-foreground uppercase">
      Sources
    </p>
    <div class="mt-4 overflow-x-auto border border-border">
      <table class="w-full min-w-table-min text-sm">
        <thead>
          <tr class="border-b border-border text-left">
            <th
              v-for="h in HEADERS"
              :key="h"
              class="px-4 py-2 font-mono text-11 font-normal tracking-wider text-muted-foreground uppercase"
            >
              {{ h }}
            </th>
          </tr>
        </thead>
        <tbody v-if="isLoading" aria-busy="true">
          <tr
            v-for="n in SKELETON_ROWS"
            :key="n"
            class="border-b border-border"
          >
            <td v-for="h in HEADERS" :key="h" class="px-4 py-3">
              <Skeleton class="h-4 w-16" />
            </td>
          </tr>
        </tbody>
        <tbody v-else-if="sources.length === 0">
          <tr>
            <td
              :colspan="HEADERS.length"
              class="px-4 py-8 text-center text-muted-foreground"
            >
              No tile requests recorded yet.
            </td>
          </tr>
        </tbody>
        <tbody v-else class="font-mono tabular-nums">
          <tr
            v-for="s in sources"
            :key="s.id"
            class="border-b border-border last:border-b-0"
          >
            <td class="px-4 py-3 font-sans font-semibold text-foreground">
              {{ s.id }}
            </td>
            <td class="px-4 py-3">{{ formatCount(s.requests) }}</td>
            <td class="px-4 py-3">{{ formatPercent(s.cache_hit_rate) }}</td>
            <td class="px-4 py-3">{{ formatMs(s.latency.p50_ms) }}</td>
            <td class="px-4 py-3">{{ formatMs(s.latency.p95_ms) }}</td>
            <td class="px-4 py-3">
              {{
                formatBytes(
                  s.avg_bytes === null ? null : Math.round(s.avg_bytes),
                )
              }}
            </td>
            <td
              class="px-4 py-3"
              :class="s.errors > 0 ? 'text-destructive' : ''"
            >
              {{ formatCount(s.errors) }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
