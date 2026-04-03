<script setup lang="ts">
  const { features } = useMarketingPage();
</script>

<template>
  <section data-label="Features" class="border-b border-border">
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
        Capabilities
      </p>
      <h2
        class="
          max-w-2xl font-display text-3xl font-semibold
          lg:text-4xl
        "
        style="letter-spacing: -0.03em; line-height: 1.15"
      >
        Everything you need to serve vector tiles
      </h2>
    </div>

    <!-- Feature grid -->
    <div
class="
  grid gap-px border-t border-border bg-border
  md:grid-cols-3
">
      <div
        v-for="feature in features"
        :key="feature.title"
        class="
          bg-background p-6
          lg:p-8
        "
      >
        <component :is="feature.icon" class="mb-4 size-5 text-primary" />
        <h3 class="mb-2 font-display text-sm font-semibold tracking-tight">
          {{ feature.title }}
        </h3>
        <p class="font-sans text-sm/relaxed text-muted-foreground">
          {{ feature.description }}
        </p>
      </div>
    </div>
  </section>
</template>
