<script setup lang="ts">
  import { useHomePage } from '~/composables/use-home-page';

  const { pingQuery } = useHomePage();

  const statusOk = computed(() => pingQuery.data.value?.status === 'ok');
  const isLoading = computed(() => pingQuery.isLoading.value);

  const versionLabel = computed(() => {
    if (!pingQuery.data.value) return '';
    return `v${pingQuery.data.value.version}`;
  });

  const rendererEnabled = computed(
    () => pingQuery.data.value?.renderer_enabled ?? false,
  );

  const cacheMb = computed(() => {
    if (!pingQuery.data.value) return '—';
    if (pingQuery.data.value.cache_enabled === false) {
      return pingQuery.data.value.renderer_enabled ? '✓' : '✗';
    }
    return `${(pingQuery.data.value.cache_bytes / 1024 / 1024).toFixed(0)}`;
  });

  const cacheEnabled = computed(
    () => pingQuery.data.value?.cache_enabled ?? false,
  );

  function formatUptime(unix: number): string {
    const now = Date.now() / 1000;
    const diff = Math.max(0, now - unix);
    if (diff < 60) return `${Math.floor(diff)}s`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h`;
    return `${Math.floor(diff / 86400)}d`;
  }

  const uptime = computed(() => {
    const unix = pingQuery.data.value?.loaded_at_unix;
    if (!unix) return '—';
    return formatUptime(unix);
  });
</script>

<template>
  <section
    class="hero w-full max-w-[1600px] mx-auto border-b border-border grid gap-6 px-[clamp(16px,4vw,32px)] py-[clamp(20px,4vw,32px)]"
    style="grid-template-columns: 1fr"
    aria-labelledby="hero-title"
  >
    <div class="flex flex-col justify-end">
      <div class="hero-meta-row flex flex-wrap gap-1.5 mb-3.5">
        <span
          v-if="!isLoading"
          class="hero-meta inline-flex items-center gap-2.5 px-3 py-1.5 text-[11px] font-medium border"
          :class="
            statusOk
              ? 'border-success/30 bg-success/10 text-success'
              : 'border-destructive/30 bg-destructive/10 text-destructive'
          "
        >
          <span
            class="hero-dot size-2 rounded-full bg-current shrink-0"
            :class="statusOk ? 'text-success' : 'text-destructive'"
            aria-hidden="true"
          ></span>
          <span
            class="uc-mono font-mono text-[11px] font-semibold tracking-[0.10em] uppercase"
            >Live</span
          >
        </span>
        <span
          class="hero-meta neutral inline-flex items-center gap-2 px-3 py-1.5 text-[11px] font-medium border border-border bg-surface text-muted-foreground"
        >
          <span
            class="uc-mono font-mono text-[11px] tracking-[0.10em] uppercase"
            >Renderer {{ rendererEnabled ? '✓' : '✗' }}</span
          >
        </span>
        <span
          v-if="versionLabel"
          class="hero-meta neutral inline-flex items-center px-3 py-1.5 text-[11px] font-medium border border-border bg-surface text-muted-foreground"
        >
          <span
            class="uc-mono font-mono text-[11px] tracking-[0.10em] uppercase"
            >{{ versionLabel }}</span
          >
        </span>
      </div>

      <h1
        id="hero-title"
        class="text-[clamp(28px,5vw,42px)] font-bold leading-[1.04] tracking-[-0.03em] mb-2.5 max-w-[22ch]"
      >
        Self-hosted tile server
      </h1>
      <p class="text-[15px] text-muted-foreground leading-[1.55] max-w-[56ch]">
        PMTiles, MBTiles, COG, STAC, OGC API and an MCP server in one Rust
        binary.
      </p>
    </div>

    <div
      class="hero-stats grid gap-0 border border-border bg-surface"
      style="
        grid-template-columns: repeat(2, 1fr);
        max-width: 460px;
        margin-top: 16px;
      "
      role="list"
      aria-label="Runtime metrics"
    >
      <div
        class="hero-stat border-r border-b border-border p-3"
        role="listitem"
      >
        <div
          class="hero-stat-l font-mono text-[10px] tracking-[0.18em] uppercase text-muted-foreground font-medium mb-1"
        >
          Sources
        </div>
        <div
          class="hero-stat-v font-mono text-[22px] font-semibold text-foreground tracking-[-0.02em] tabular-nums"
        >
          {{ pingQuery.data.value?.loaded_sources ?? '—' }}
        </div>
      </div>
      <div
        class="hero-stat border-r border-b border-border p-3"
        style="border-right: none"
        role="listitem"
      >
        <div
          class="hero-stat-l font-mono text-[10px] tracking-[0.18em] uppercase text-muted-foreground font-medium mb-1"
        >
          Styles
        </div>
        <div
          class="hero-stat-v font-mono text-[22px] font-semibold text-foreground tracking-[-0.02em] tabular-nums"
        >
          {{ pingQuery.data.value?.loaded_styles ?? '—' }}
        </div>
      </div>
      <div class="hero-stat border-r border-border p-3" role="listitem">
        <div
          class="hero-stat-l font-mono text-[10px] tracking-[0.18em] uppercase text-muted-foreground font-medium mb-1"
        >
          Cache
        </div>
        <div
          class="hero-stat-v font-mono text-[22px] font-semibold text-foreground tracking-[-0.02em] tabular-nums flex items-baseline gap-1"
        >
          <span v-if="!cacheEnabled">—</span>
          <template v-else>
            {{ cacheMb }}
            <span class="unit text-[11px] font-medium text-muted-foreground"
              >MB</span
            >
          </template>
        </div>
      </div>
      <div
        class="hero-stat border-r-0 p-3"
        style="border-right: none"
        role="listitem"
      >
        <div
          class="hero-stat-l font-mono text-[10px] tracking-[0.18em] uppercase text-muted-foreground font-medium mb-1"
        >
          Uptime
        </div>
        <div
          class="hero-stat-v font-mono text-[22px] font-semibold text-foreground tracking-[-0.02em] tabular-nums flex items-baseline gap-1"
        >
          <span v-if="!uptime">—</span>
          <template v-else>
            {{ uptime }}
          </template>
        </div>
      </div>
    </div>
  </section>
</template>
