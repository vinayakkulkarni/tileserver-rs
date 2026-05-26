<script setup lang="ts">
  import type { Component } from 'vue';
  import { ChevronDown } from '@lucide/vue';

  defineProps<{
    title: string;
    count: number;
    countLabel: string;
    icon: Component;
    bodyId: string;
    isOpen: boolean;
  }>();

  defineEmits<{
    'toggle-section': [];
  }>();
</script>

<template>
  <section class="section" :class="{ open: isOpen }">
    <button
      type="button"
      class="section-toggle group/toggle min-h-14 w-full flex items-center justify-between gap-3 px-[clamp(14px,3vw,20px)] py-3.5 transition-colors duration-[var(--d-fast,120ms)] hover:bg-muted/50"
      :aria-expanded="isOpen"
      :aria-controls="bodyId"
      @click="$emit('toggle-section')"
    >
      <div class="section-left min-w-0 flex items-center gap-3 text-left">
        <div
          class="section-icon size-9 grid place-items-center bg-muted text-foreground transition-colors duration-[var(--d-fast,120ms)] group-hover/toggle:bg-primary/10 group-hover/toggle:text-primary"
          aria-hidden="true"
        >
          <component :is="icon" class="size-[18px]" />
        </div>
        <div>
          <div class="section-title text-[16px] font-semibold leading-tight">
            {{ title }}
          </div>
          <div
            class="section-count mt-0.5 font-mono text-[11px] font-medium tracking-[0.04em] text-muted-foreground"
          >
            {{ count }} {{ countLabel }}
          </div>
        </div>
      </div>
      <ChevronDown
        class="section-chev size-5 shrink-0 text-muted-foreground transition-transform duration-[var(--d-base,180ms)] ease-[var(--ease,cubic-bezier(0.16,1,0.3,1))]"
        :class="{ 'rotate-180': isOpen }"
        aria-hidden="true"
      />
    </button>
    <div :id="bodyId" class="section-body-wrap">
      <div class="section-body-inner">
        <div
          class="section-body grid gap-2 border-t border-border px-[clamp(12px,3vw,20px)] py-3"
          style="
            grid-template-columns: repeat(
              auto-fit,
              minmax(min(100%, 360px), 1fr)
            );
          "
        >
          <slot></slot>
        </div>
      </div>
    </div>
  </section>
</template>
