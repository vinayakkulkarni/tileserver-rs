<script setup lang="ts">
  const {
    activeProvider,
    isDirect,
    apiKey,
    remoteTarget,
    testResult,
    testing,
    runTest,
    onForgetKey,
    closeSettings,
  } = useLlmProviderSettings();
</script>

<template>
  <p
    v-if="testResult"
    :role="testResult.ok ? 'status' : 'alert'"
    :class="testResult.ok ? 'text-success' : 'text-destructive'"
  >
    {{ testResult.message }}
  </p>

  <div class="mt-auto flex flex-wrap gap-2">
    <button
      v-if="activeProvider !== 'webllm'"
      type="button"
      :disabled="!remoteTarget || testing"
      class="h-11 border border-border/60 px-3 font-medium hover:border-primary/40 disabled:opacity-40"
      @click="runTest"
    >
      {{ testing ? 'Testing…' : 'Test connection' }}
    </button>
    <button
      v-if="isDirect && apiKey"
      type="button"
      class="h-11 border border-border/60 px-3 font-medium hover:border-destructive/40 hover:text-destructive"
      @click="onForgetKey"
    >
      Forget key
    </button>
    <button
      type="button"
      class="ml-auto h-11 bg-primary px-4 font-medium text-primary-foreground hover:bg-primary/90"
      @click="closeSettings"
    >
      Done
    </button>
  </div>
</template>
