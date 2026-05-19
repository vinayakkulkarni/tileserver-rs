<script setup lang="ts">
  import { Search } from '@lucide/vue';
  import { motion } from 'motion-v';

  defineProps<{
    modelValue: string;
  }>();

  defineEmits<{
    'update:modelValue': [value: string];
  }>();
</script>

<template>
  <motion.div
    :initial="{ opacity: 0, y: 8 }"
    :animate="{ opacity: 1, y: 0 }"
    :transition="{ duration: 0.3, delay: 0.1 }"
    class="relative"
  >
    <Search
      class="absolute top-1/2 left-4 size-4 -translate-y-1/2 text-muted-foreground"
    />
    <Input
      :model-value="modelValue"
      placeholder="Search styles and data sources..."
      class="h-11 border-border/50 bg-muted/30 pl-11 transition-all focus:bg-background"
      @update:model-value="$emit('update:modelValue', $event)"
    />
  </motion.div>
</template>
