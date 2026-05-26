<script setup lang="ts">
  import { Check } from '@lucide/vue';

  defineProps<{
    visible: boolean;
    message: string;
  }>();
</script>

<template>
  <Teleport to="body">
    <div
      class="toast fixed bottom-6 left-1/2 z-50 flex -translate-x-1/2 items-center gap-2 border border-border bg-foreground px-4 py-3 font-mono text-[12px] tracking-[0.08em] uppercase font-semibold text-background shadow-lg transition-transform duration-[var(--d-slow,320ms)] ease-[var(--ease,cubic-bezier(0.16,1,0.3,1))]"
      :class="visible ? 'translate-y-0' : 'translate-y-[150%]'"
      role="status"
      aria-live="polite"
    >
      <Check class="size-4 shrink-0 text-success" aria-hidden="true" />
      {{ message }}
    </div>
  </Teleport>
</template>
