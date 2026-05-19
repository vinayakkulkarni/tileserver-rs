<script setup lang="ts">
  import { ChevronRight, ExternalLink } from '@lucide/vue';
  import { motion } from 'motion-v';
</script>

<template>
  <motion.a
    href="/_openapi"
    target="_blank"
    :initial="{ opacity: 0, y: 12 }"
    :animate="{ opacity: 1, y: 0 }"
    :transition="{ duration: 0.35, delay: 0.35 }"
    class="block"
  >
    <Card
      class="overflow-hidden border-border/50 bg-card/50 backdrop-blur-sm transition-all hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5"
    >
      <div class="flex items-center gap-3 p-4">
        <div
          class="flex size-8 items-center justify-center bg-muted/50"
        >
          <ExternalLink class="size-4 text-muted-foreground" />
        </div>
        <div class="flex-1">
          <span class="font-medium">API Documentation</span>
          <p class="text-xs text-muted-foreground">
            OpenAPI 3.1 specification with Scalar
          </p>
        </div>
        <ChevronRight class="size-4 text-muted-foreground" />
      </div>
    </Card>
  </motion.a>
</template>
