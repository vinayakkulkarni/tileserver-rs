<script setup lang="ts">
  import { useHomeHero } from '~/composables/use-home-hero';

  import { ChevronDown } from '@lucide/vue';

  const {
    statusOk,
    isLoading,
    versionLabel,
    rendererEnabled,
    cacheEnabled,
    cacheMb,
    uptime,
    sourceCount,
    styleCount,
    compressionEnabled,
    compressionLabel,
    ogcEnabled,
    heroExpanded,
    toggleHero,
  } = useHomeHero();
</script>

<template>
  <section
    class="hero shrink-0 border-b border-border"
    aria-label="Runtime status"
  >
    <!-- Compact disclosure (<lg) — glanceable summary line + tap-to-expand
         detail, so pinned chrome stays small on phones/tablets. -->
    <div class="hero-compact lg:hidden" :class="{ open: heroExpanded }">
      <button
        type="button"
        class="flex min-h-11 w-full items-center justify-between gap-2.5 px-[clamp(12px,4vw,24px)] py-2.5 text-left transition-colors duration-[var(--d-fast,120ms)] hover:bg-primary/[0.04]"
        :aria-expanded="heroExpanded"
        aria-controls="hero-detail"
        @click="toggleHero"
      >
        <span
          class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 font-mono text-[11px] font-medium tracking-[0.06em] text-muted-foreground"
        >
          <span
            v-if="!isLoading"
            class="inline-flex items-center gap-1.5 border px-2 py-0.5 text-[9.5px] font-semibold uppercase tracking-[0.14em]"
            :class="
              statusOk
                ? 'border-success/30 bg-success/10 text-success'
                : 'border-destructive/30 bg-destructive/10 text-destructive'
            "
          >
            <span
              class="hero-dot size-1.5 shrink-0 bg-current"
              aria-hidden="true"
              style="border-radius: 50%"
            ></span>
            Live
          </span>
          <span
            ><span class="font-semibold tabular-nums text-foreground">{{
              sourceCount
            }}</span>
            sources</span
          >
          <span aria-hidden="true" class="opacity-40">·</span>
          <span
            ><span class="font-semibold tabular-nums text-foreground">{{
              styleCount
            }}</span>
            styles</span
          >
          <template v-if="versionLabel">
            <span aria-hidden="true" class="opacity-40">·</span>
            <span class="font-semibold text-foreground">{{
              versionLabel
            }}</span>
          </template>
        </span>
        <ChevronDown
          class="size-[18px] shrink-0 text-muted-foreground transition-transform duration-[var(--d-base,180ms)] ease-[var(--ease,cubic-bezier(0.16,1,0.3,1))]"
          :class="{ 'rotate-180': heroExpanded }"
          aria-hidden="true"
        />
      </button>
      <div id="hero-detail" class="hero-detail-wrap">
        <div class="hero-detail-inner">
          <div
            class="flex flex-col gap-2.5 border-t border-border px-[clamp(12px,4vw,24px)] pb-3 pt-3"
          >
            <p
              class="flex flex-wrap items-center gap-x-3.5 gap-y-1.5 font-mono text-[10px] font-medium uppercase tracking-[0.14em] text-muted-foreground"
            >
              <span
                >Renderer
                <span
                  :class="rendererEnabled ? 'font-semibold text-success' : ''"
                  >{{ rendererEnabled ? '✓' : '✗' }}</span
                ></span
              >
              <span
                >Compression
                <span
                  :class="
                    compressionEnabled ? 'font-semibold text-success' : ''
                  "
                  >{{ compressionEnabled ? '✓' : '✗' }}</span
                ></span
              >
              <span
                >OGC
                <span :class="ogcEnabled ? 'font-semibold text-success' : ''">{{
                  ogcEnabled ? '✓' : '✗'
                }}</span></span
              >
            </p>
            <dl
              class="grid grid-cols-3 gap-x-3 text-foreground"
              aria-label="Runtime metrics"
            >
              <div class="flex flex-col">
                <dt
                  class="font-mono text-[9.5px] font-medium uppercase tracking-[0.14em] text-muted-foreground"
                >
                  Cache
                </dt>
                <dd
                  class="flex items-baseline gap-0.5 font-mono text-[15px] font-semibold leading-none tabular-nums"
                >
                  <span v-if="!cacheEnabled" class="text-muted-foreground"
                    >—</span
                  >
                  <template v-else>
                    {{ cacheMb
                    }}<span
                      class="text-[10px] font-medium text-muted-foreground"
                      >MB</span
                    >
                  </template>
                </dd>
              </div>
              <div class="flex flex-col">
                <dt
                  class="font-mono text-[9.5px] font-medium uppercase tracking-[0.14em] text-muted-foreground"
                >
                  Uptime
                </dt>
                <dd
                  class="font-mono text-[15px] font-semibold leading-none tabular-nums"
                >
                  {{ uptime }}
                </dd>
              </div>
              <div class="flex flex-col">
                <dt
                  class="font-mono text-[9.5px] font-medium uppercase tracking-[0.14em] text-muted-foreground"
                >
                  Codec
                </dt>
                <dd
                  class="font-mono text-[13px] font-semibold leading-none text-foreground"
                >
                  <span v-if="!compressionEnabled" class="text-muted-foreground"
                    >off</span
                  >
                  <template v-else>{{ compressionLabel }}</template>
                </dd>
              </div>
            </dl>
          </div>
        </div>
      </div>
    </div>

    <!-- Admin-style band (lg+) — 2-col 3fr_2fr grid mirroring /admin Operator console. -->
    <div class="mx-auto hidden max-w-screen-2xl lg:grid lg:grid-cols-[3fr_2fr]">
      <div
        class="flex flex-col justify-between gap-2 border-r border-border px-[clamp(16px,4vw,32px)] py-3"
      >
        <p
          class="font-mono text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground"
        >
          Catalog
        </p>
        <p
          v-if="isLoading"
          class="font-display text-[32px] font-semibold leading-none tabular-nums tracking-tight text-muted-foreground"
        >
          —
        </p>
        <p
          v-else
          class="font-display text-[32px] font-semibold leading-none tabular-nums tracking-tight text-foreground"
        >
          {{ uptime }}
        </p>
        <p
          class="flex flex-wrap items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted-foreground"
        >
          <span
            class="inline-flex items-center gap-1.5 border px-2 py-0.5 font-semibold"
            :class="
              statusOk
                ? 'border-success/30 bg-success/10 text-success'
                : 'border-destructive/30 bg-destructive/10 text-destructive'
            "
          >
            <span
              class="hero-dot size-1.5 shrink-0 bg-current"
              aria-hidden="true"
              style="border-radius: 50%"
            ></span>
            Live
          </span>
          <span>· Renderer {{ rendererEnabled ? '✓' : '✗' }}</span>
          <span>· Compression {{ compressionEnabled ? '✓' : '✗' }}</span>
          <span>· OGC {{ ogcEnabled ? '✓' : '✗' }}</span>
          <span v-if="versionLabel">· {{ versionLabel }}</span>
        </p>
      </div>

      <div class="grid grid-cols-2 grid-rows-2">
        <div class="border-b border-border px-5 py-2.5">
          <p
            class="font-mono text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground"
          >
            Sources
          </p>
          <p
            class="mt-1 font-display text-2xl font-semibold leading-none tabular-nums text-foreground"
          >
            {{ sourceCount }}
          </p>
        </div>
        <div class="border-b border-l border-border px-5 py-2.5">
          <p
            class="font-mono text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground"
          >
            Styles
          </p>
          <p
            class="mt-1 font-display text-2xl font-semibold leading-none tabular-nums text-foreground"
          >
            {{ styleCount }}
          </p>
        </div>
        <div class="px-5 py-2.5">
          <p
            class="font-mono text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground"
          >
            Cache
          </p>
          <p
            class="mt-1 flex items-baseline gap-0.5 font-display text-2xl font-semibold leading-none tabular-nums text-foreground"
          >
            <span v-if="!cacheEnabled" class="text-muted-foreground">—</span>
            <template v-else>
              {{ cacheMb
              }}<span class="text-[10px] font-medium text-muted-foreground"
                >MB</span
              >
            </template>
          </p>
        </div>
        <div class="border-l border-border px-5 py-2.5">
          <p
            class="font-mono text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground"
          >
            Compression
          </p>
          <p
            class="mt-1 font-mono text-[13px] font-semibold leading-tight text-foreground"
          >
            <span v-if="!compressionEnabled" class="text-muted-foreground"
              >off</span
            >
            <template v-else>{{ compressionLabel }}</template>
          </p>
        </div>
      </div>
    </div>
  </section>
</template>
