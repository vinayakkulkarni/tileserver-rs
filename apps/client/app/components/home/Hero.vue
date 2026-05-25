<script setup lang="ts">
  import { Check, Globe, Moon, Settings, Sun, X } from '@lucide/vue';
  import { useHomePage } from '~/composables/use-home-page';

  const { isDark, toggleColorMode, pingQuery } = useHomePage();

  const statusOk = computed(() => pingQuery.data.value?.status === 'ok');

  function formatUptime(unix: number): string {
    const now = Date.now() / 1000;
    const diff = Math.max(0, now - unix);
    if (diff < 60) return `${Math.floor(diff)}s`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h`;
    return `${Math.floor(diff / 86400)}d`;
  }

  const cacheMb = computed(() => {
    if (!pingQuery.data.value) return '—';
    if (pingQuery.data.value.cache_enabled === false) {
      return pingQuery.data.value.renderer_enabled ? '✓' : '✗';
    }
    return `${(pingQuery.data.value.cache_bytes / 1024 / 1024).toFixed(0)}`;
  });

  const stats = computed(() => [
    { label: 'Sources', value: pingQuery.data.value?.loaded_sources ?? '—' },
    { label: 'Styles', value: pingQuery.data.value?.loaded_styles ?? '—' },
    { label: 'Cache', value: cacheMb.value },
    {
      label: 'Uptime',
      value: pingQuery.data.value?.loaded_at_unix
        ? formatUptime(pingQuery.data.value.loaded_at_unix)
        : '—',
    },
  ]);
</script>

<template>
  <header
    class="sticky top-0 z-50 border-b border-border/50 bg-background/80 backdrop-blur-xl"
  >
    <div class="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
      <div class="flex h-14 items-center justify-between">
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

        <div class="flex items-center gap-1">
          <NuxtLink to="/admin" aria-label="Open admin">
            <Button variant="ghost" size="icon" as="span">
              <Settings class="size-5" />
            </Button>
          </NuxtLink>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Toggle color mode"
            @click="toggleColorMode"
          >
            <Sun v-if="isDark" class="size-5" />
            <Moon v-else class="size-5" />
          </Button>
        </div>
      </div>

      <div
        class="flex flex-wrap items-center gap-x-4 gap-y-2 pb-4 sm:grid sm:grid-cols-[1fr_auto] sm:items-center"
      >
        <div class="flex items-center gap-3">
          <span
            v-if="pingQuery.data.value"
            class="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium"
            :class="
              statusOk
                ? 'bg-success/10 text-success'
                : 'bg-destructive/10 text-destructive'
            "
          >
            <span
              class="hero-dot relative size-2 rounded-full bg-success"
            ></span>
            Live
          </span>
          <span
            v-if="pingQuery.data.value"
            class="inline-flex items-center gap-1.5 rounded-full bg-muted px-2.5 py-0.5 text-xs font-medium text-muted-foreground"
          >
            <Check
              v-if="pingQuery.data.value.renderer_enabled"
              class="size-3"
            />
            <X v-else class="size-3" />
            Renderer
          </span>
          <span
            v-if="pingQuery.data.value"
            class="inline-flex rounded-full bg-muted px-2.5 py-0.5 text-xs font-medium text-muted-foreground"
          >
            v{{ pingQuery.data.value.version }}
          </span>
        </div>

        <div
          class="hidden sm:grid grid-cols-4 divide-x divide-border border border-border text-sm tabular-nums"
        >
          <div
            v-for="stat in stats"
            :key="stat.label"
            class="px-3 py-2.5 text-center"
          >
            <div class="text-lg font-semibold text-foreground tabular-nums">
              {{ stat.value }}
            </div>
            <div
              class="text-[10px] uppercase tracking-widest text-muted-foreground"
            >
              {{ stat.label }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </header>
</template>
