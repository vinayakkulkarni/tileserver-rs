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
        class="flex min-h-11 w-full items-center justify-between gap-2.5 px-[clamp(12px,4vw,24px)] py-2.5 text-left transition-colors duration-[var(--d-fast,120ms)] hover:bg-primary/4"
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

    <!-- Dense status strip (lg+) — single horizontal row, pills on the left,
         inline label:value metrics on the right. Linear/Vercel-status-bar dense. -->
    <div
      class="mx-auto hidden max-w-screen-2xl lg:flex lg:items-center lg:gap-5 px-[clamp(16px,4vw,32px)] py-2.5"
      role="group"
      aria-label="Runtime status"
    >
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
        <span class="font-medium">· Renderer {{ rendererEnabled ? '✓' : '✗' }}</span>
        <span class="font-medium">· Compression {{ compressionEnabled ? '✓' : '✗' }}</span>
        <span class="font-medium">· OGC {{ ogcEnabled ? '✓' : '✗' }}</span>
        <span v-if="versionLabel" class="font-semibold text-foreground">· {{ versionLabel }}</span>
      </p>

      <span
        aria-hidden="true"
        class="hidden h-4 w-px shrink-0 bg-border lg:block"
      ></span>

      <dl
        class="flex flex-wrap items-baseline gap-x-4 gap-y-0.5 font-mono text-[10px] uppercase tracking-[0.14em] text-muted-foreground"
        aria-label="Runtime metrics"
      >
        <div class="inline-flex items-baseline gap-1.5">
          <dt>Sources</dt>
          <dd class="font-mono text-[13px] font-semibold leading-none tabular-nums tracking-normal text-foreground">
            {{ sourceCount }}
          </dd>
        </div>
        <div class="inline-flex items-baseline gap-1.5">
          <dt>Styles</dt>
          <dd class="font-mono text-[13px] font-semibold leading-none tabular-nums tracking-normal text-foreground">
            {{ styleCount }}
          </dd>
        </div>
        <div class="inline-flex items-baseline gap-1.5">
          <dt>Cache</dt>
          <dd class="font-mono text-[13px] font-semibold leading-none tabular-nums tracking-normal text-foreground">
            <span v-if="!cacheEnabled" class="text-muted-foreground">—</span>
            <template v-else>
              {{ cacheMb
              }}<span class="text-[10px] font-medium text-muted-foreground"
                >MB</span
              >
            </template>
          </dd>
        </div>
        <div class="inline-flex items-baseline gap-1.5">
          <dt>Uptime</dt>
          <dd class="font-mono text-[13px] font-semibold leading-none tabular-nums tracking-normal text-foreground">
            {{ isLoading ? '—' : uptime }}
          </dd>
        </div>
      </dl>
    </div>
  </section>
</template>
