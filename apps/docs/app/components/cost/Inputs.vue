<script setup lang="ts">
  import { ChevronDown } from '@lucide/vue';
  import type { CityRtt, CostField, TileWorkloadKey } from '~/types';

  defineProps<{
    fields: readonly CostField[];
    cities: readonly CityRtt[];
  }>();

  const workload = defineModel<Record<TileWorkloadKey, number>>('workload', {
    required: true,
  });

  const city = defineModel<string>('city', { required: true });
  const servers = defineModel<number>('servers', { required: true });
  const includeFree = defineModel<boolean>('includeFree', { required: true });

  /** Replace one workload field, keeping the others. */
  function setField(key: TileWorkloadKey, value: number) {
    workload.value = { ...workload.value, [key]: value };
  }
</script>

<template>
  <div
    class="grid gap-px border-b border-border bg-border sm:grid-cols-2 lg:grid-cols-3"
  >
    <CostNumberField
      v-for="f in fields"
      :key="f.key"
      :model-value="workload[f.key]"
      :label="f.label"
      :step="f.step"
      :max="f.max"
      @update:model-value="setField(f.key, $event)"
    />
    <CostNumberField
      v-model="servers"
      label="Hetzner servers"
      :step="1"
      :min="1"
    />

    <label class="bg-background p-4">
      <span
        class="mb-1.5 block font-mono text-10 uppercase tracking-200 text-muted-foreground"
      >User city (latency)</span>
      <span class="relative block">
        <select
          v-model="city"
          class="h-10 w-full appearance-none border border-input bg-background px-3 pr-9 font-mono text-sm text-foreground transition-colors focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-none"
        >
          <option
            v-for="c in cities"
            :key="c.city"
            :value="c.city"
          >
            {{ c.city }}
          </option>
        </select>
        <ChevronDown
          class="pointer-events-none absolute top-1/2 right-3 size-3 -translate-y-1/2 text-muted-foreground"
          aria-hidden="true"
        />
      </span>
    </label>

    <label
      class="flex cursor-pointer items-center gap-3 bg-background p-4 lg:col-span-2"
    >
      <input
        v-model="includeFree"
        type="checkbox"
        class="size-4 shrink-0 appearance-none border border-input bg-background transition-colors checked:border-primary checked:bg-primary focus-visible:ring-1 focus-visible:ring-ring focus-visible:outline-none"
      >
      <span class="font-sans text-sm">Include free tiers and plan allowances</span>
    </label>
  </div>
</template>
