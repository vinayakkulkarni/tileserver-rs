<script setup lang="ts">
  defineProps<{ pricesReadOn: string }>();
</script>

<template>
  <footer
    class="space-y-1 border-t border-border px-4 py-4 font-sans text-xs/relaxed text-muted-foreground"
  >
    <p>
      Prices read {{ pricesReadOn }} from each vendor's public page; open a
      breakdown to see each source. With free tiers off, free units cost the
      first paid band's price. Contract discounts are not included.
    </p>
    <p>
      Round trips: median of WonderNetwork's hourly pings, 18 Sep – 2 Oct 2026.
      Frankfurt stands in for Hetzner Falkenstein and Nuremberg.
    </p>
  </footer>
</template>
