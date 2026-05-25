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
    class="inline-flex shrink-0 items-center gap-1.5 border px-3 py-1.5 text-xs font-medium transition-colors duration-[var(--d-fast,120ms)]"
    :class="
      active
        ? 'border-primary bg-primary/10 text-primary'
        : 'border-border bg-background text-muted-foreground hover:border-primary/50 hover:bg-primary/5'
    "
    @click="emit('select-filter', category)"
  >
    {{ label }}
    <span
      class="inline-flex size-5 items-center justify-center rounded-full text-[10px] tabular-nums"
      :class="
        active
          ? 'bg-primary text-primary-foreground'
          : 'bg-muted text-muted-foreground'
      "
    >
      {{ count }}
    </span>
  </button>
</template>
