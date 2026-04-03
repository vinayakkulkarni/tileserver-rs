<script setup lang="ts">
  import { Github, Globe, Menu, Moon, Sun, X } from 'lucide-vue-next';

  const { isDark, toggle } = useThemeToggle();
  const mobileOpen = ref(false);
</script>

<template>
  <header class="fixed top-0 z-50 w-full bg-background">
    <div
      class="
        grid h-[72px] grid-cols-[72px_1fr_48px] border-b border-border
        lg:h-[80px] lg:grid-cols-[80px_1fr_auto_80px]
      "
    >
      <!-- Logo cell -->
      <div class="flex items-center justify-center border-r border-border">
        <NuxtLink to="/" class="flex items-center justify-center">
          <Globe
class="
  size-6 text-primary
  lg:size-7
" />
        </NuxtLink>
      </div>

      <!-- Nav + title -->
      <div
class="
  flex items-center px-5
  lg:px-6
">
        <NuxtLink to="/" class="flex items-center gap-2.5">
          <span
            class="
              font-display text-sm font-semibold tracking-[0.15em] uppercase
            "
          >
            <span class="text-foreground">Tileserver</span
            ><span class="text-primary"> RS</span>
          </span>
        </NuxtLink>
        <div
class="
  ml-auto hidden items-center gap-6
  lg:flex
">
          <NuxtLink
            to="https://docs.tileserver.app"
            external
            class="
              font-mono text-xs tracking-wider text-muted-foreground uppercase
              transition-colors
              hover:text-foreground
            "
          >
            Docs
          </NuxtLink>
          <NuxtLink
            to="https://demo.tileserver.app"
            external
            class="
              font-mono text-xs tracking-wider text-muted-foreground uppercase
              transition-colors
              hover:text-foreground
            "
          >
            Demo
          </NuxtLink>
        </div>
      </div>

      <!-- Theme toggle cell (desktop) -->
      <div
        class="
          hidden items-center justify-center border-l border-border px-4
          lg:flex
        "
      >
        <button
          class="
            flex size-9 items-center justify-center text-muted-foreground
            transition-colors
            hover:text-foreground
          "
          aria-label="Toggle theme"
          @click="toggle"
        >
          <Sun v-if="isDark" class="size-4" />
          <Moon v-else class="size-4" />
        </button>
      </div>

      <!-- Hamburger (mobile) -->
      <div
        class="
          flex items-center justify-center border-l border-border
          lg:hidden
        "
      >
        <button
          class="
            flex items-center justify-center text-muted-foreground
            hover:text-foreground
          "
          @click="mobileOpen = !mobileOpen"
        >
          <Menu v-if="!mobileOpen" class="size-5" />
          <X v-else class="size-5" />
        </button>
      </div>
      <NuxtLink
        to="https://github.com/vinayakkulkarni/tileserver-rs"
        external
        class="
          hidden items-center justify-center border-l border-border
          text-muted-foreground transition-colors
          hover:text-foreground
          lg:flex
        "
      >
        <Github class="size-5" />
      </NuxtLink>
    </div>
  </header>

  <!-- Mobile menu -->
  <div
    v-if="mobileOpen"
    class="
      fixed inset-0 z-40 bg-background/95 backdrop-blur-md
      lg:hidden
    "
  >
    <nav class="flex h-full flex-col items-center justify-center gap-8">
      <NuxtLink
        to="https://docs.tileserver.app"
        external
        class="font-display text-2xl font-semibold tracking-[0.15em] uppercase"
        @click="mobileOpen = false"
      >
        Docs
      </NuxtLink>
      <NuxtLink
        to="https://demo.tileserver.app"
        external
        class="font-display text-2xl font-semibold tracking-[0.15em] uppercase"
        @click="mobileOpen = false"
      >
        Demo
      </NuxtLink>
      <NuxtLink
        to="https://github.com/vinayakkulkarni/tileserver-rs"
        external
        class="font-display text-2xl font-semibold tracking-[0.15em] uppercase"
        @click="mobileOpen = false"
      >
        GitHub
      </NuxtLink>
    </nav>
  </div>
</template>
