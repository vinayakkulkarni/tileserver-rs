<script setup lang="ts">
  import { Palette } from '@lucide/vue';
  import type { Style } from '~/types/style';

  defineProps<{
    styles: Style[];
    isLoading: boolean;
    hasStyles: boolean;
    searchQuery: string;
    baseUrl: string;
    expandedXyz: Set<string>;
    copiedUrl: string | null;
  }>();

  const emit = defineEmits<{
    toggleXyz: [styleId: string];
    copyUrl: [url: string];
  }>();

  function handleToggleXyz(styleId: string) {
    emit('toggleXyz', styleId);
  }

  function handleCopyUrl(url: string) {
    emit('copyUrl', url);
  }
</script>

<template>
  <Separator class="bg-border/50" />
  <div class="p-4">
    <div v-if="isLoading" class="flex justify-center py-12">
      <div
        class="size-8 animate-spin rounded-full border-2 border-muted border-t-primary"
      ></div>
    </div>
    <div v-else-if="!hasStyles" class="py-12 text-center">
      <div
        class="mx-auto mb-4 flex size-16 items-center justify-center bg-muted/50"
      >
        <Palette class="size-8 text-muted-foreground" />
      </div>
      <p class="font-medium">No styles configured</p>
      <p class="mt-1 text-sm text-muted-foreground">
        Add styles to your config.toml
      </p>
    </div>
    <div
      v-else-if="styles.length === 0"
      class="py-12 text-center text-muted-foreground"
    >
      No styles match "{{ searchQuery }}"
    </div>
    <div v-else class="space-y-3">
      <HomeStyleCard
        v-for="(style, i) in styles"
        :key="style.id"
        :style="style"
        :index="i"
        :base-url="baseUrl"
        :is-xyz-expanded="expandedXyz.has(style.id)"
        :copied-url="copiedUrl"
        @toggle-xyz="handleToggleXyz"
        @copy-url="handleCopyUrl"
      />
    </div>
  </div>
</template>
