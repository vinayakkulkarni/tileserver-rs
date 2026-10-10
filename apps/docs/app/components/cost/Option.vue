<script setup lang="ts">
  import type { CityRtt, CostEstimate, CostRow, CostSource } from '~/types';

  defineProps<{
    row: CostRow;
    rtt: CityRtt | undefined;
    reach: { ms: number; source: CostSource };
    formatCost: (estimate: CostEstimate) => string;
    formatMoney: (value: number) => string;
    formatCount: (value: number) => string;
  }>();
</script>

<template>
  <article class="border-b border-border px-4 py-4 last:border-b-0">
    <div
      class="grid gap-3 md:grid-cols-[minmax(0,1.1fr)_minmax(0,1.5fr)_minmax(0,0.8fr)_minmax(0,1.1fr)] md:gap-6"
    >
      <h4 class="font-display text-sm font-semibold">
        {{ row.name }}
      </h4>
      <p class="font-sans text-sm/relaxed text-muted-foreground">
        {{ row.basis }}
      </p>
      <div>
        <span
          class="font-mono text-10 uppercase tracking-200 text-muted-foreground md:hidden"
        >Monthly cost
        </span>
        <a
          v-if="row.estimate.kind === 'contact'"
          :href="row.estimate.source.url"
          target="_blank"
          rel="noopener"
          class="font-mono text-sm text-muted-foreground underline decoration-border underline-offset-2 transition-colors hover:text-foreground"
        >{{ formatCost(row.estimate) }}</a>
        <span
          v-else
          class="font-mono text-sm text-foreground"
        >{{
          formatCost(row.estimate)
        }}</span>
        <p
          v-if="row.estimate.kind === 'usd' && row.estimate.setup"
          class="mt-1 font-mono text-10 text-muted-foreground"
        >
          + {{ formatMoney(row.estimate.setup) }} one-time setup
        </p>
      </div>
      <CostLatencyNote
        :latency="row.latency"
        :rtt="rtt"
        :reach="reach"
      />
    </div>

    <CostBreakdown
      v-if="row.estimate.kind === 'usd'"
      :lines="row.estimate.lines"
      :format-money="formatMoney"
      :format-count="formatCount"
    />
  </article>
</template>
