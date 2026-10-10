<script setup lang="ts">
  withDefaults(
    defineProps<{
      label: string;
      step: number;
      min?: number;
      max?: number;
    }>(),
    { min: 0, max: undefined },
  );

  const value = defineModel<number>({ required: true });
</script>

<template>
  <label class="bg-background p-4">
    <span
      class="mb-1.5 block font-mono text-10 uppercase tracking-200 text-muted-foreground"
    >{{ label }}</span>
    <input
      v-model.number="value"
      type="number"
      :min="min"
      :max="max"
      :step="step"
      inputmode="numeric"
      class="h-10 w-full border border-input bg-background px-3 font-mono text-sm text-foreground transition-colors focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-none"
    >
  </label>
</template>
