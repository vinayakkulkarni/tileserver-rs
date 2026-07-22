<script setup lang="ts">
  const { featuresByCategory } = useMarketingPage();
</script>

<template>
  <section data-label="Features" class="border-b border-border">
    <div class="px-6 pt-16 pb-10 md:px-12 lg:px-20">
      <p
        class="
          mb-3 font-mono text-10 tracking-300 text-muted-foreground
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

    <!-- Category groups -->
    <div
      v-for="group in featuresByCategory"
      :key="group.category"
      class="border-t border-border"
    >
      <div class="px-6 py-3 md:px-12 lg:px-20">
        <h3
          class="
            font-mono text-10 tracking-300 text-muted-foreground
            uppercase
            lg:text-xs
          "
        >
          {{ group.category }}
        </h3>
      </div>
      <div class="grid gap-px border-t border-border bg-border md:grid-cols-3">
        <div
          v-for="feature in group.features"
          :key="feature.title"
          class="
            bg-background p-6
            lg:p-8
          "
        >
          <component :is="feature.icon" class="mb-4 size-5 text-primary" />
          <h4 class="mb-2 font-display text-sm font-semibold tracking-tight">
            {{ feature.title }}
          </h4>
          <p class="font-sans text-sm/relaxed text-muted-foreground">
            {{ feature.description }}
          </p>
        </div>
      </div>
    </div>
  </section>
</template>
