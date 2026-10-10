<script setup lang="ts">
  const {
    workload,
    includeFree,
    city,
    servers,
    rtt,
    rowsIn,
    fields,
    groups,
    cities,
    cloudflareReach,
    pricesReadOn,
    formatCost,
    formatMoney,
    formatCount,
  } = useTilesCost();
</script>

<template>
  <section class="border border-border bg-background text-foreground">
    <CostInputs
      v-model:city="city"
      v-model:servers="servers"
      v-model:include-free="includeFree"
      v-model:workload="workload"
      :fields="fields"
      :cities="cities"
    />

    <div
      class="hidden border-b border-border px-4 py-3 font-mono text-10 uppercase tracking-200 text-muted-foreground md:grid md:grid-cols-[minmax(0,1.1fr)_minmax(0,1.5fr)_minmax(0,0.8fr)_minmax(0,1.1fr)] md:gap-6"
      aria-hidden="true"
    >
      <span>Option</span>
      <span>Billing basis</span>
      <span>Monthly cost</span>
      <span>Latency from {{ city }}</span>
    </div>

    <section
      v-for="g in groups"
      :key="g.id"
      :aria-label="g.label"
    >
      <h3
        class="border-b border-border bg-muted/40 px-4 py-2 font-mono text-10 uppercase tracking-200 text-muted-foreground"
      >
        {{ g.label }}
      </h3>
      <CostOption
        v-for="r in rowsIn(g.id)"
        :key="r.id"
        :row="r"
        :rtt="rtt"
        :reach="cloudflareReach"
        :format-cost="formatCost"
        :format-money="formatMoney"
        :format-count="formatCount"
      />
    </section>

    <CostNotes :prices-read-on="pricesReadOn" />
  </section>
</template>
