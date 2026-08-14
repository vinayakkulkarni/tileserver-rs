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
      class="section-toggle group/toggle min-h-14 w-full flex items-center justify-between gap-3 px-section-x py-3.5 transition-colors duration-(--d-fast) hover:bg-muted/50"
      :aria-expanded="isOpen"
      :aria-controls="bodyId"
      @click="$emit('toggle-section')"
    >
      <div class="section-left min-w-0 flex items-center gap-3 text-left">
        <div
          class="section-icon size-9 grid place-items-center bg-muted text-foreground transition-colors duration-(--d-fast) group-hover/toggle:bg-primary/10 group-hover/toggle:text-primary"
          aria-hidden="true"
        >
          <component :is="icon" class="size-panel" />
        </div>
        <div>
          <div class="section-title text-16/tight font-semibold">
            {{ title }}
          </div>
          <div
            class="section-count mt-0.5 font-mono text-11 font-medium tracking-40 text-muted-foreground"
          >
            {{ count }} {{ countLabel }}
          </div>
        </div>
      </div>
      <ChevronDown
        class="section-chev size-5 shrink-0 text-muted-foreground transition-transform duration-(--d-base) ease-brand"
        :class="{ 'rotate-180': isOpen }"
        aria-hidden="true"
      />
    </button>
    <div :id="bodyId" class="section-body-wrap">
      <div class="section-body-inner">
        <div
          class="section-body grid grid-cols-section gap-2 border-t border-border px-section-x-sm py-3"
        >
          <slot></slot>
        </div>
      </div>
    </div>
  </section>
</template>
