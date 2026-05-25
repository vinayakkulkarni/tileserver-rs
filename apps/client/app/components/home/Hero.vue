<script setup lang="ts">
  import { Check, X } from '@lucide/vue';
  import { useHomePage } from '~/composables/use-home-page';

  const { pingQuery } = useHomePage();

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

  const isLoading = computed(() => pingQuery.isLoading.value);

  const stats = computed(() => [
    { label: 'Sources', value: pingQuery.data.value?.loaded_sources ?? '—' },
    { label: 'Styles', value: pingQuery.data.value?.loaded_styles ?? '—' },
    { label: 'Cache', value: cacheMb.value, unit: 'MB' },
    {
      label: 'Uptime',
      value: pingQuery.data.value?.loaded_at_unix
        ? formatUptime(pingQuery.data.value.loaded_at_unix)
        : '—',
      unit: '',
    },
  ]);
</script>

<template>
  <section
    class="mx-auto w-full max-w-[1600px] border-b border-border px-4 pb-5 pt-7 sm:px-6 sm:pb-8 sm:pt-12 lg:px-8"
    aria-labelledby="hero-title"
  >
    >
    <div
      class="grid grid-cols-1 items-end gap-6 sm:grid-cols-[1fr_auto] sm:gap-12"
    >
      <!-- Left: kicker pills + headline + subtitle -->
      <div>
        <div class="mb-3.5 flex flex-wrap gap-1.5">
          <!-- Live pill -->
          <span
            v-if="!isLoading"
            class="inline-flex items-center gap-2 rounded-full px-3 py-1.5 text-xs font-medium border"
            :class="
              statusOk
                ? 'border-success/30 bg-success/10 text-success'
                : 'border-destructive/30 bg-destructive/10 text-destructive'
            "
          >
            <span
              class="hero-dot relative size-2 rounded-full bg-success"
            ></span>
            <span
              class="font-mono text-[11px] font-semibold tracking-[0.10em] uppercase"
              :class="statusOk ? 'text-success' : 'text-destructive'"
              >Live</span
            >
          </span>
          <!-- Renderer pill -->
          <span
            v-if="!isLoading"
            class="inline-flex items-center gap-1.5 rounded-full border border-border bg-surface px-3 py-1.5"
          >
            <Check
              v-if="pingQuery.data.value?.renderer_enabled"
              class="size-3 text-success"
            />
            <X v-else class="size-3 text-muted-foreground" />
            <span
              class="font-mono text-[11px] font-semibold tracking-[0.10em] uppercase text-muted-foreground"
              >Renderer</span
            >
          </span>
          <!-- Version pill -->
          <span
            v-if="pingQuery.data.value"
            class="inline-flex items-center rounded-full border border-border bg-surface px-3 py-1.5"
          >
            <span
              class="font-mono text-[11px] font-semibold tracking-[0.10em] uppercase text-muted-foreground"
              >v{{ pingQuery.data.value.version }}</span
            >
          </span>
        </div>

        <h1
          id="hero-title"
          class="font-bold text-[clamp(28px,5vw,42px)] leading-[1.04] tracking-[-0.03em] mb-2.5 max-w-[22ch]"
        >
          Self-hosted tile server
        </h1>
        <p
          class="text-[15px] text-muted-foreground leading-[1.55] max-w-[56ch]"
        >
          PMTiles, MBTiles, COG, STAC, OGC API and an MCP server in one Rust
          binary.
        </p>
      </div>

      <!-- Right: stats grid -->
      <div
        v-if="!isLoading"
        class="stats-grid grid grid-cols-2 border border-border bg-surface sm:grid-cols-4 sm:max-w-[460px]"
      >
        <div
          v-for="(stat, idx) in stats"
          :key="stat.label"
          class="px-3.5 py-3 border-r border-border"
          :class="[
            idx % 2 === 1 ? 'border-r-0 sm:border-r' : '',
            idx >= stats.length - 2 ? 'border-b-0' : 'border-b sm:border-b-0',
            idx === stats.length - 1 ? 'sm:border-r-0' : '',
          ]"
        >
          <div
            class="text-[10px] font-mono font-medium uppercase tracking-[0.12em] text-muted-foreground"
          >
            {{ stat.label }}
          </div>
          <div
            class="mt-0.5 text-lg font-semibold tabular-nums text-foreground"
          >
            {{ stat.value
            }}<span
              v-if="'unit' in stat && stat.unit"
              class="ml-0.5 text-sm font-medium text-muted-foreground"
            >
              {{ stat.unit }}</span
            >
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
