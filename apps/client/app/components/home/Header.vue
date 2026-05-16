<script setup lang="ts">
  import { Globe, Moon, Sun } from '@lucide/vue';

  defineProps<{
    isDark: boolean;
  }>();

  const emit = defineEmits<{
    toggleTheme: [];
  }>();

  function handleToggle() {
    emit('toggleTheme');
  }
</script>

<template>
  <header class="sticky top-0 z-50 border-b border-border bg-background">
    <div class="flex h-14 items-center justify-between px-6">
      <div class="flex items-center gap-3">
        <div class="flex size-9 items-center justify-center bg-primary">
          <Globe class="size-5 text-primary-foreground" />
        </div>
        <div>
          <h1 class="text-lg font-semibold tracking-tight">Tileserver RS</h1>
          <p class="text-xs text-muted-foreground">
            High-performance vector tile server
          </p>
        </div>
      </div>
      <Button variant="ghost" size="icon" @click="handleToggle">
        <Sun v-if="isDark" class="size-5" />
        <Moon v-else class="size-5" />
      </Button>
    </div>
  </header>
</template>
