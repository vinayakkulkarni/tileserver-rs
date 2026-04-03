<script setup lang="ts">
  import { ArrowRight } from 'lucide-vue-next';

  const { performanceStats } = useMarketingPage();
</script>

<template>
  <section data-label="Performance" class="border-b border-border">
    <div
class="
  px-6 pt-16 pb-10
  md:px-12
  lg:px-20
">
      <p
        class="
          mb-3 font-mono text-[10px] tracking-[0.3em] text-muted-foreground
          uppercase
          lg:text-xs
        "
      >
        Benchmarks
      </p>
      <h2
        class="
          mb-4 max-w-2xl font-display text-3xl font-semibold
          lg:text-4xl
        "
        style="letter-spacing: -0.03em; line-height: 1.15"
      >
        Benchmarked &amp; Proven
      </h2>
      <p
        class="
          max-w-2xl font-sans text-sm/relaxed text-muted-foreground
          lg:text-base
        "
      >
        Real numbers from Docker-to-Docker benchmarks on ARM64. No marketing
        fluff.
      </p>
    </div>

    <!-- Stats grid -->
    <div
      class="
        grid grid-cols-2 gap-px border-y border-border bg-border
        md:grid-cols-4
      "
    >
      <div
        v-for="stat in performanceStats"
        :key="stat.label"
        class="
          bg-background p-6 text-center
          lg:p-8
        "
      >
        <component :is="stat.icon" class="mx-auto mb-3 size-5 text-primary" />
        <p
class="
  font-display text-4xl font-bold text-primary
  lg:text-5xl
">
          <span v-if="stat.prefix">{{ stat.prefix }}</span>
          <CountUp :to="stat.value" :duration="2.5" />
        </p>
        <p
          class="
            mt-3 font-mono text-[10px] tracking-wider text-muted-foreground
            uppercase
          "
        >
          {{ stat.label }}
        </p>
        <p class="mt-1 font-sans text-xs text-emerald-400">{{ stat.detail }}</p>
      </div>
    </div>

    <div
class="
  border-t border-border px-6 py-8
  md:px-12
  lg:px-20
">
      <NuxtLink
        to="https://docs.tileserver.app/benchmarks/performance"
        external
        class="
          group inline-flex items-center gap-2 font-mono text-xs tracking-wider
          text-muted-foreground uppercase transition-colors
          hover:text-foreground
        "
      >
        View Full Benchmarks
        <ArrowRight
          class="
            size-3.5 transition-transform
            group-hover:translate-x-0.5
          "
        />
      </NuxtLink>
    </div>
  </section>
</template>
