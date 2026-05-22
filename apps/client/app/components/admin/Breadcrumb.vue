<script setup lang="ts">
  import type { AdminBreadcrumbCrumb } from '~/types/admin-mcp';

  defineProps<{ items: AdminBreadcrumbCrumb[] }>();
</script>

<template>
  <p
    class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase"
  >
    <template v-for="(crumb, idx) in items" :key="idx">
      <NuxtLink
        v-if="crumb.to"
        :to="crumb.to"
        class="transition-colors hover:text-foreground"
      >
        {{ crumb.label }}
      </NuxtLink>
      <span v-else>{{ crumb.label }}</span>
      <span v-if="idx < items.length - 1" class="mx-2 text-muted-foreground/60"
        >/</span
      >
    </template>
  </p>
</template>
