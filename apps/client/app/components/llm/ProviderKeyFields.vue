<script setup lang="ts">
  const {
    activeProvider,
    activeSettings,
    baseUrlPlaceholder,
    keyRequired,
    setupHint,
    destinationHost,
    apiKey,
    rememberKey,
    onBaseUrlInput,
    onModelInput,
    onKeyInput,
    onRememberChange,
  } = useLlmProviderSettings();

  const fieldClass =
    'h-11 border border-border/60 bg-background px-3 outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20';
</script>

<template>
  <div v-if="activeSettings" class="flex flex-col gap-3">
    <label class="flex flex-col gap-1">
      <span class="font-medium">Base URL</span>
      <input
        type="url"
        inputmode="url"
        autocomplete="off"
        spellcheck="false"
        :value="activeSettings.baseUrl"
        :placeholder="baseUrlPlaceholder"
        :class="fieldClass"
        @input="onBaseUrlInput"
      />
    </label>
    <label class="flex flex-col gap-1">
      <span class="font-medium">Model</span>
      <input
        type="text"
        autocomplete="off"
        spellcheck="false"
        :value="activeSettings.model"
        :placeholder="
          activeProvider === 'anthropic' ? 'claude-sonnet-4-5' : 'gpt-4.1-mini'
        "
        :class="fieldClass"
        @input="onModelInput"
      />
    </label>
    <label class="flex flex-col gap-1">
      <span class="font-medium">
        API key
        <span v-if="!keyRequired" class="font-normal text-muted-foreground">
          (optional for local servers)
        </span>
      </span>
      <input
        type="password"
        autocomplete="off"
        spellcheck="false"
        :value="apiKey"
        :class="fieldClass"
        @input="onKeyInput"
      />
    </label>
    <label class="flex min-h-11 items-center gap-2">
      <input
        type="checkbox"
        :checked="rememberKey"
        class="size-4 accent-primary"
        @change="onRememberChange"
      />
      <span>Remember key on this device</span>
    </label>
    <p class="text-muted-foreground">
      The key goes from your browser straight to {{ destinationHost }}. It never
      reaches this server.
      {{
        rememberKey
          ? 'It is saved in this browser until you forget it; any script on this page can read it.'
          : 'It is cleared when you close this tab.'
      }}
    </p>
    <p v-if="setupHint" role="alert" class="text-destructive">
      {{ setupHint }}
    </p>
  </div>
</template>
