<script setup lang="ts">
  import { Globe, Moon, Sun } from '@lucide/vue';
  import { motion } from 'motion-v';

  defineProps<{
    isDark: boolean;
  }>();

  const emit = defineEmits<{
    toggleTheme: [];
  }>();
</script>

<template>
  <header
    class="sticky top-0 z-50 border-b border-border/50 bg-background/80 backdrop-blur-xl"
  >
    <div class="flex h-14 items-center justify-between px-4">
      <motion.div
        :initial="{ opacity: 0, x: -12 }"
        :animate="{ opacity: 1, x: 0 }"
        :transition="{ duration: 0.4 }"
        class="flex items-center gap-3"
      >
        <div
          class="flex size-9 items-center justify-center rounded-xl bg-linear-to-br from-primary to-primary/80 shadow-lg shadow-primary/20"
        >
          <Globe class="size-5 text-primary-foreground" />
        </div>
        <div>
          <h1 class="text-lg font-semibold tracking-tight">Tileserver RS</h1>
          <p class="text-xs text-muted-foreground">
            High-performance vector tile server
          </p>
        </div>
      </motion.div>
      <Button
        variant="ghost"
        size="icon"
        class="rounded-xl"
        @click="emit('toggleTheme')"
      >
        <Sun v-if="isDark" class="size-5" />
        <Moon v-else class="size-5" />
      </Button>
    </div>
  </header>
</template>
