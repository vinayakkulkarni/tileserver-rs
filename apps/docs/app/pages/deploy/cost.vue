<script setup lang="ts">
  import { ArrowLeft } from '@lucide/vue';

  usePageSeo({
    title: 'Hosting Cost',
    description:
      'Estimate the monthly cost of serving map tiles with tileserver-rs on one server, edge storage (R2 + Workers with PMTiles), AWS, or hosted vendors.',
    path: '/deploy/cost',
  });
</script>

<template>
  <div class="mx-auto max-w-5xl px-6 py-12">
    <NuxtLink
      to="/"
      class="mb-8 inline-flex items-center gap-1.5 font-mono text-xs uppercase tracking-wider text-muted-foreground transition-colors hover:text-foreground"
    >
      <ArrowLeft class="size-3" />
      Documentation
    </NuxtLink>

    <article class="prose mb-10 max-w-3xl">
      <h1>Hosting cost</h1>
      <p class="text-lg/relaxed">
        What it costs to serve map tiles each month. This calculator compares
        tileserver-rs on one server against edge storage (R2 + Workers with
        PMTiles), AWS, and hosted vendors. Numbers are estimates from public
        price pages, not quotes. Change the inputs to match your workload.
      </p>
    </article>

    <CostCalculator />
  </div>
</template>
