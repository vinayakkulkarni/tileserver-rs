<script setup lang="ts">
  import { Map } from '@lucide/vue';
  import { motion } from 'motion-v';
  import type { Style } from '~/types/style';

  const props = defineProps<{
    style: Style;
    index: number;
    baseUrl: string;
    isXyzExpanded: boolean;
    copiedUrl: string | null;
  }>();

  const emit = defineEmits<{
    toggleXyz: [styleId: string];
    copyUrl: [url: string];
  }>();

  const imgError = ref(false);

  function handleImgError() {
    imgError.value = true;
  }

  function handleToggleXyz() {
    emit('toggleXyz', props.style.id);
  }

  function handleServiceCopyUrl(url: string) {
    emit('copyUrl', url);
  }
</script>

<template>
  <motion.div
    :initial="{ opacity: 0, y: 12 }"
    :animate="{ opacity: 1, y: 0 }"
    :transition="{ duration: 0.3, delay: 0.05 * index }"
    class="group rounded-xl border border-border/50 bg-background/50 p-4 transition-all hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5"
  >
    <div class="flex gap-4">
      <div
        class="flex size-20 shrink-0 items-center justify-center overflow-hidden rounded-xl bg-muted ring-1 ring-border/50"
      >
        <img
          v-if="!imgError"
          :src="`/styles/${style.id}/static/0,0,1/160x160.png`"
          :alt="style.name"
          class="size-full object-cover transition-transform group-hover:scale-105"
          loading="lazy"
          @error="handleImgError"
        />
        <Map v-else class="size-8 text-muted-foreground" />
      </div>

      <div class="min-w-0 flex-1">
        <div class="flex items-start justify-between gap-2">
          <div>
            <h3 class="font-semibold">{{ style.name }}</h3>
            <p class="mt-0.5 text-sm text-muted-foreground">
              <code
                class="rounded-md bg-muted px-1.5 py-0.5 text-xs font-medium"
                >{{ style.id }}</code
              >
            </p>
          </div>
          <Button as-child size="sm" class="rounded-lg">
            <NuxtLink :to="`/styles/${style.id}/`">
              <Map class="mr-1.5 size-4" />
              Viewer
            </NuxtLink>
          </Button>
        </div>

        <HomeStyleCardServices
          :style="style"
          :base-url="baseUrl"
          :is-xyz-expanded="isXyzExpanded"
          :copied-url="copiedUrl"
          @toggle-xyz="handleToggleXyz"
          @copy-url="handleServiceCopyUrl"
        />
      </div>
    </div>
  </motion.div>
</template>
