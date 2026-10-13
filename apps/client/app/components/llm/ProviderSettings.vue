<script setup lang="ts">
  import {
    Select as UiSelect,
    SelectContent as UiSelectContent,
    SelectItem as UiSelectItem,
    SelectTrigger as UiSelectTrigger,
    SelectValue as UiSelectValue,
  } from '~/components/ui/select';

  const { options, activeProvider, isDirect, serverConfig, onProviderChange } =
    useLlmProviderSettings();
</script>

<template>
  <section
    aria-labelledby="llm-settings-title"
    class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4 text-xs"
  >
    <h2
      id="llm-settings-title"
      class="text-10 font-semibold tracking-widest text-muted-foreground uppercase"
    >
      AI provider
    </h2>
    <label class="flex flex-col gap-1">
      <span class="font-medium">Provider</span>
      <UiSelect
        :model-value="activeProvider"
        @update:model-value="onProviderChange"
      >
        <UiSelectTrigger
          class="h-11 w-full border border-border/60 bg-background px-2.5 text-xs shadow-none"
          aria-label="AI provider"
        >
          <UiSelectValue />
        </UiSelectTrigger>
        <UiSelectContent>
          <UiSelectItem
            v-for="o in options"
            :key="o.value"
            :value="o.value"
            class="text-xs"
          >
            {{ o.label }}
          </UiSelectItem>
        </UiSelectContent>
      </UiSelect>
    </label>

    <p v-if="activeProvider === 'webllm'" class="text-muted-foreground">
      Runs in your browser with WebGPU. Prompts never leave this device.
    </p>
    <p v-else-if="activeProvider === 'proxy'" class="text-muted-foreground">
      This server sends your prompts to
      {{ serverConfig?.upstream_host ?? 'its provider' }} with model
      {{ serverConfig?.model }}. No key needed.
    </p>

    <LlmProviderKeyFields v-if="isDirect" />
    <LlmProviderActions :key="activeProvider" />
  </section>
</template>
