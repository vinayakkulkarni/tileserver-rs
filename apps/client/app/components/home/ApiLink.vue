<script setup lang="ts">
  import { ChevronRight, ExternalLink } from '@lucide/vue';
</script>

<template>
  <a href="/_openapi" target="_blank" class="block">
    <div
      class="flex items-center gap-3 border border-border bg-card p-4 transition-colors duration-[var(--d-fast,120ms)] hover:border-primary/30"
    >
      <div class="size-8 shrink-0 bg-surface-2 grid place-items-center">
        <ExternalLink class="size-4 text-muted-foreground" />
      </div>
      <div class="flex-1">
        <span class="font-medium">API Documentation</span>
        <p class="text-xs text-muted-foreground">
          OpenAPI 3.1 specification with Scalar
        </p>
      </div>
      <ChevronRight class="size-4 text-muted-foreground shrink-0" />
    </div>
  </a>
</template>
