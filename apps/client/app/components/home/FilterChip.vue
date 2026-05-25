<script setup lang="ts">
  import type { FilterCategory } from '~/types/home-filters';

  defineProps<{
    label: string;
    count: number;
    category: FilterCategory;
    active: boolean;
  }>();

  const emit = defineEmits<{
    'select-filter': [category: FilterCategory];
  }>();
</script>

<template>
  <button
    :aria-pressed="active"
    class="inline-flex shrink-0 items-center gap-1.5 border px-3 py-1.5 font-mono text-[11px] font-medium tracking-[0.06em] transition-colors duration-[var(--d-fast,120ms)]"
    :class="
      active
        ? 'border-primary bg-primary/10 text-primary'
        : 'border-border bg-surface text-muted-foreground hover:border-border-strong hover:text-foreground'
    "
    @click="emit('select-filter', category)"
  >
    {{ label }}
    <span
      class="inline-flex px-1.5 py-px text-[10px] font-semibold"
      :class="
        active
          ? 'bg-primary text-primary-foreground'
          : 'bg-surface-2 text-muted-foreground'
      "
    >
      {{ count }}
    </span>
  </button>
</template>
