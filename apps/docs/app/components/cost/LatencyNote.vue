<script setup lang="ts">
  import type { CityRtt, CostLatency, CostSource } from '~/types';

  defineProps<{
    latency: CostLatency;
    rtt: CityRtt | undefined;
    reach: { ms: number; source: CostSource };
  }>();
</script>

<template>
  <div class="font-sans text-sm text-muted-foreground">
    <span class="font-mono text-10 uppercase tracking-200 md:hidden">Latency
    </span>
    <template v-if="latency === 'edge'">
      <span class="font-mono text-foreground">≤ {{ reach.ms }} ms</span>
      to the nearest Cloudflare location for 95% of users (<a
        :href="reach.source.url"
        target="_blank"
        rel="noopener"
        class="underline decoration-border underline-offset-2 hover:text-foreground"
      >source</a>). R2 reads that miss the cache add the trip to the bucket region.
    </template>
    <template v-else-if="latency === 'hetzner' && rtt">
      <span class="font-mono text-foreground">{{ rtt.germanyMs }} ms</span>
      to Germany (<a
        :href="rtt.sources[0].url"
        target="_blank"
        rel="noopener"
        class="underline decoration-border underline-offset-2 hover:text-foreground"
      >ping</a>),
      <span class="font-mono text-foreground">{{ rtt.finlandMs }} ms</span>
      to Finland (<a
        :href="rtt.sources[1].url"
        target="_blank"
        rel="noopener"
        class="underline decoration-border underline-offset-2 hover:text-foreground"
      >ping</a>), round trip, before any server time.
    </template>
    <template v-else>
      Not measured.
    </template>
  </div>
</template>
