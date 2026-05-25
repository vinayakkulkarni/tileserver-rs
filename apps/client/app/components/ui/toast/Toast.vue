<script setup lang="ts">
  import { Check } from '@lucide/vue';

  defineProps<{
    visible: boolean;
    message: string;
  }>();
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition-transform duration-[var(--d-slow,320ms)] ease-[var(--ease,cubic-bezier(0.16,1,0.3,1))]"
      enter-from-class="translate-y-full opacity-0"
      enter-to-class="translate-y-0 opacity-100"
      leave-active-class="transition-transform duration-[var(--d-slow,320ms)] ease-[var(--ease,cubic-bezier(0.16,1,0.3,1))]"
      leave-from-class="translate-y-0 opacity-100"
      leave-to-class="translate-y-full opacity-0"
    >
      <div
        v-if="visible"
        class="toast fixed bottom-6 left-1/2 z-[100] flex -translate-x-1/2 items-center gap-2 border border-border bg-background px-4 py-2.5 text-sm shadow-lg"
        role="status"
        aria-live="polite"
      >
        <Check class="size-4 text-success" />
        {{ message }}
      </div>
    </Transition>
  </Teleport>
</template>
