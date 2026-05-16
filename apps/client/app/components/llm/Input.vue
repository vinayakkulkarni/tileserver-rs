<script setup lang="ts">
  import { Send, Square } from '@lucide/vue';

  const props = defineProps<{
    modelValue: string;
    isLoading: boolean;
    engineStatus: string;
    hasMessages: boolean;
  }>();

  const emit = defineEmits<{
    'update:modelValue': [value: string];
    submit: [];
    stop: [];
  }>();

  const isDisabled = computed(
    () =>
      props.engineStatus === 'loading' ||
      (!props.modelValue.trim() && !props.isLoading),
  );

  const placeholder = computed(() => {
    if (props.engineStatus === 'loading') return 'Loading model...';
    if (props.engineStatus === 'error') return 'Model failed to load';
    return 'Ask about the map...';
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (!props.isLoading && props.modelValue.trim()) {
        emit('submit');
      }
    }
  }

  function updateValue(e: Event) {
    emit('update:modelValue', (e.target as HTMLInputElement).value);
  }
</script>

<template>
  <form
    class="flex items-end gap-2 border-t border-border/50 bg-muted/20 p-3"
    @submit.prevent="isLoading ? emit('stop') : emit('submit')"
  >
    <input
      type="text"
      :value="modelValue"
      :placeholder="placeholder"
      :disabled="engineStatus === 'loading' || engineStatus === 'error'"
      class="flex-1 border border-border/60 bg-background px-3 py-2 text-sm outline-none transition-colors placeholder:text-muted-foreground/60 focus:border-primary/50 focus:ring-1 focus:ring-primary/20 disabled:opacity-40"
      @input="updateValue"
      @keydown="handleKeydown"
    />
    <button
      type="submit"
      :disabled="isDisabled"
      class="flex size-9 shrink-0 items-center justify-center bg-primary text-primary-foreground transition-all hover:bg-primary/90 disabled:opacity-30"
    >
      <Square v-if="isLoading" class="size-4" />
      <Send v-else class="size-4" />
    </button>
  </form>
</template>
