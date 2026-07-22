<script setup lang="ts">
  import {
    Select as UiSelect,
    SelectContent as UiSelectContent,
    SelectItem as UiSelectItem,
    SelectTrigger as UiSelectTrigger,
    SelectValue as UiSelectValue,
  } from '~/components/ui/select';
  import type { LlmModelConfig } from '~/types/llm';

  defineProps<{
    models: LlmModelConfig[];
    selectedId: string;
    disabled: boolean;
  }>();

  const emit = defineEmits<{
    select: [modelId: string];
  }>();

  function handleChange(value: string) {
    emit('select', value);
  }
</script>

<template>
  <UiSelect
    :model-value="selectedId"
    :disabled="disabled"
    @update:model-value="handleChange"
  >
    <UiSelectTrigger
      class="h-8 w-full gap-1.5 border border-border/60 bg-background px-2.5 text-11 shadow-none transition-colors hover:border-primary/40 hover:bg-primary/5"
    >
      <UiSelectValue placeholder="Select model" />
    </UiSelectTrigger>
    <UiSelectContent class="w-[--reka-popper-anchor-width]">
      <UiSelectItem
        v-for="model in models"
        :key="model.id"
        :value="model.id"
        class="text-xs"
      >
        <span class="flex items-center gap-1.5">
          {{ model.name }}
          <span class="text-muted-foreground">({{ model.sizeGb }}GB)</span>
        </span>
      </UiSelectItem>
    </UiSelectContent>
  </UiSelect>
</template>
