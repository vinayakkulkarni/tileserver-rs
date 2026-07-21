<script setup lang="ts">
  import type { ComparisonCell, ComparisonColumn } from '~/types/marketing';

  const { comparisonColumns, comparisonRows } = useMarketingPage();

  function cellClass(cell: ComparisonCell): string {
    if (cell === '✓') return 'text-emerald-400 font-bold text-base';
    if (cell === '◐') return 'text-amber-400 text-base';
    return 'text-muted-foreground/40 text-base';
  }

  function colHeaderClass(col: ComparisonColumn): string {
    return col === 'tileserver-rs' ? 'text-primary' : 'text-muted-foreground';
  }
</script>

<template>
  <section data-label="Compare" class="border-b border-border">
    <div class="px-6 pt-16 pb-10 md:px-12 lg:px-20">
      <p
        class="
          mb-3 font-mono text-10 tracking-300 text-muted-foreground
          uppercase
          lg:text-xs
        "
      >
        Compare
      </p>
      <h2
        class="
          max-w-2xl font-display text-3xl font-semibold
          lg:text-4xl
        "
        style="letter-spacing: -0.03em; line-height: 1.15"
      >
        Compare against the alternatives
      </h2>
      <p
        class="
          mt-4 max-w-2xl font-sans text-sm/relaxed text-muted-foreground
          lg:text-base
        "
      >
        We didn&rsquo;t rebuild these. We rolled them into one binary.
      </p>
    </div>

    <div class="overflow-x-auto px-6 pb-12 md:px-12 lg:px-20">
      <table class="w-full min-w-[640px] border-collapse">
        <thead>
          <tr class="border-b border-border">
            <th
              scope="col"
              class="
                pr-6 pb-3 text-left font-mono text-10 tracking-300
                text-muted-foreground uppercase
                lg:text-xs
              "
            >
              Feature
            </th>
            <th
              v-for="col in comparisonColumns"
              :key="col"
              scope="col"
              class="
                px-4 pb-3 text-center font-mono text-10 tracking-200
                uppercase
                lg:text-xs
              "
              :class="colHeaderClass(col)"
            >
              {{ col }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="row in comparisonRows"
            :key="row.feature"
            class="
              border-b border-border/50 transition-colors
              hover:bg-muted/20
            "
          >
            <td class="py-3 pr-6 font-sans text-sm text-foreground">
              {{ row.feature }}
            </td>
            <td
              v-for="col in comparisonColumns"
              :key="col"
              class="px-4 py-3 text-center"
            >
              <span :class="cellClass(row.values[col])">{{
                row.values[col]
              }}</span>
            </td>
          </tr>
        </tbody>
      </table>
      <p class="mt-6 font-sans text-xs text-muted-foreground/70">
        Features as of April 2026. Alternatives may have added features since
        &mdash; check their docs.
      </p>
    </div>
  </section>
</template>
