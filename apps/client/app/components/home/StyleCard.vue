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
    class="group border border-border bg-background p-3.5 transition-all duration-[var(--d-fast,120ms)] hover:border-primary hover:bg-primary/2.5 focus-within:border-primary"
  >
    <div class="flex gap-3.5">
      <!-- 56x56 thumbnail per A2 -->
      <div
        class="flex size-14 shrink-0 items-center justify-center overflow-hidden border border-border bg-surface-2"
      >
        <img
          v-if="!imgError"
          :src="`/styles/${style.id}/static/0,0,1/160x160.png`"
          :alt="style.name"
          class="size-full object-cover"
          loading="lazy"
          @error="handleImgError"
        />
        <Map v-else class="size-5.5 text-muted-foreground" />
      </div>

      <!-- Card content -->
      <div class="min-w-0 flex-1">
        <div class="flex items-start justify-between gap-2.5">
          <div class="min-w-0">
            <h3 class="text-[15px] font-bold leading-snug tracking-[-0.005em]">
              {{ style.name }}
            </h3>
            <p class="mt-0.5">
              <code
                class="inline-block bg-surface-2 px-1.5 py-0.5 font-mono text-[11px] font-medium text-muted-foreground"
              >
                {{ style.id }}
              </code>
            </p>
          </div>
          <Button as-child size="sm" class="shrink-0">
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
