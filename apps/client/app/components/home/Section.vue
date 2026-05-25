<script setup lang="ts">
  import { ChevronDown } from '@lucide/vue';

  defineProps<{
    title: string;
    count: number;
    icon: string;
    isOpen: boolean;
  }>();

  const emit = defineEmits<{
    'toggle-section': [];
  }>();
</script>

<template>
  <section class="section" :class="{ open: isOpen }">
    <button
      class="section-toggle w-full flex items-center justify-between gap-3 px-[clamp(14px,3vw,20px)] py-3.5 min-h-14 transition-colors duration-[var(--d-fast,120ms)] hover:bg-surface-2/50"
      :aria-expanded="isOpen"
      :aria-controls="`section-body-${title.toLowerCase().replace(/\s+/g, '-')}`"
      @click="emit('toggle-section')"
    >
      <div class="section-left flex items-center gap-3 min-w-0">
        <div
          class="section-icon size-9 bg-surface-2 grid place-items-center text-foreground transition-colors duration-[var(--d-fast,120ms)]"
          :class="{ 'bg-primary/10 text-primary': isOpen }"
          aria-hidden="true"
        >
          <component :is="icon" class="size-[18px]" />
        </div>
        <div>
          <div class="section-title font-semibold text-[16px] leading-tight">
            {{ title }}
          </div>
          <div
            class="section-count font-mono text-[11px] text-muted-foreground mt-0.5 tracking-wide font-medium"
          >
            {{ count }}
            {{ title.toLowerCase().includes('style') ? 'styles' : 'sources' }}
          </div>
        </div>
      </div>
      <ChevronDown
        class="section-chev size-5 text-muted-foreground shrink-0 transition-transform duration-[var(--d-base,180ms)] ease-[var(--ease,cubic-bezier(0.16,1,0.3,1))]"
        :class="{ 'rotate-180': isOpen }"
        aria-hidden="true"
      />
    </button>
    <div
      :id="`section-body-${title.toLowerCase().replace(/\s+/g, '-')}`"
      class="section-body-wrap"
    >
      <div class="section-body-inner">
        <div
          class="section-body grid gap-2 border-t border-border px-[clamp(12px,3vw,20px)] py-3"
        >
          <slot></slot>
        </div>
      </div>
    </div>
  </section>
</template>
