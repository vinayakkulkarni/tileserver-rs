<script setup lang="ts">
  import { ArrowRight } from '@lucide/vue';

  defineProps<{
    title?: string;
    to?: string;
  }>();
</script>

<template>
  <NuxtLink
    v-if="to"
    :to="to"
    class="group block border border-border p-4 transition-colors hover:border-foreground/20"
  >
    <h4
      class="mb-1.5 font-display text-sm font-semibold tracking-tight group-hover:text-primary"
    >
      {{ title }}
    </h4>
    <div class="text-sm/relaxed text-muted-foreground">
      <slot />
    </div>
    <span
      class="mt-3 inline-flex items-center gap-1 font-mono text-[10px] uppercase tracking-wider text-muted-foreground group-hover:text-foreground"
    >
      Read more
      <ArrowRight
        class="size-3 transition-transform group-hover:translate-x-0.5"
      />
    </span>
  </NuxtLink>
  <div
    v-else
    class="border border-border p-4"
  >
    <h4 class="mb-1.5 font-display text-sm font-semibold tracking-tight">
      {{ title }}
    </h4>
    <div class="text-sm/relaxed text-muted-foreground">
      <slot />
    </div>
  </div>
</template>
