<script setup lang="ts">
  import { Globe, Moon, Settings, Sun } from '@lucide/vue';
  import { useHomePage } from '~/composables/use-home-page';

  const { isDark, toggleColorMode, pingQuery } = useHomePage();
</script>

<template>
  <header class="header shrink-0 border-b border-border bg-background">
    <div
      class="mx-auto flex min-h-14 max-w-screen-2xl items-center justify-between gap-3 px-[clamp(12px,4vw,24px)]"
      style="min-height: 56px"
    >
      <NuxtLink
        to="/"
        class="brand flex items-center gap-3 min-w-0"
        aria-label="Tileserver RS home"
      >
        <div
          class="brand-glyph size-9 bg-primary grid place-items-center text-primary-foreground shrink-0 transition-filter duration-[var(--d-fast,120ms)]"
          aria-hidden="true"
        >
          <Globe class="size-5" />
        </div>
        <div>
          <div
            class="brand-name text-[15px] font-bold tracking-tight leading-none"
          >
            Tileserver RS
          </div>
          <div
            v-if="pingQuery.data.value"
            class="brand-tag font-mono text-[10.5px] tracking-[0.14em] uppercase text-muted-foreground mt-0.5 font-medium"
          >
            v{{ pingQuery.data.value.version }}
          </div>
        </div>
      </NuxtLink>

      <div class="flex items-center gap-1">
        <NuxtLink
          to="/admin"
          aria-label="Open admin settings"
          class="icon-btn size-11 grid place-items-center text-muted-foreground transition-colors duration-[var(--d-fast,120ms)] hover:bg-card hover:text-foreground"
        >
          <Settings class="size-[18px]" />
        </NuxtLink>
        <button
          type="button"
          class="icon-btn size-11 grid place-items-center text-muted-foreground transition-colors duration-[var(--d-fast,120ms)] hover:bg-card hover:text-foreground"
          aria-label="Toggle color mode"
          @click="toggleColorMode"
        >
          <Sun v-if="isDark" class="size-[18px]" />
          <Moon v-else class="size-[18px]" />
        </button>
      </div>
    </div>
  </header>
</template>
