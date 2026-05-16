<script setup lang="ts">
  import { Globe, Menu, Moon, Sun, X } from '@lucide/vue';

  const {
    sections,
    sidebarOpen,
    toggleSidebar,
    closeSidebar,
    isActive,
    isInSection,
  } = useDocsNavigation();

  const { isDark, toggle: toggleTheme } = useThemeToggle();

  const route = useRoute();

  watch(
    () => route.path,
    () => closeSidebar(),
  );
</script>

<template>
  <div class="min-h-dvh bg-background">
    <!-- ═══ Header: Grid-cell nav ═══ -->
    <nav class="fixed top-0 z-50 w-full bg-background">
      <div
        class="grid h-[72px] grid-cols-[72px_1fr_48px] border-b border-border lg:h-[80px] lg:grid-cols-[80px_1fr_auto_80px]"
      >
        <!-- Logo cell -->
        <div
          class="flex items-center justify-center border-r border-border"
        >
          <button
            class="text-muted-foreground hover:text-foreground lg:hidden"
            @click="toggleSidebar()"
          >
            <Menu
              v-if="!sidebarOpen"
              class="size-5"
            />
            <X
              v-else
              class="size-5"
            />
          </button>
          <NuxtLink
            to="/"
            class="hidden items-center justify-center lg:flex"
          >
            <Globe class="size-6 text-primary" />
          </NuxtLink>
        </div>

        <!-- Nav + title -->
        <div class="flex items-center px-5 lg:px-6">
          <NuxtLink
            to="/"
            class="flex items-center gap-2.5"
          >
            <Globe class="size-5 text-primary lg:hidden" />
            <span class="font-display text-sm font-semibold uppercase tracking-[0.15em]">
              <span class="text-foreground">Tileserver</span><span class="text-primary"> RS</span>
            </span>
            <span
              class="border border-border bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wider text-muted-foreground"
            >
              docs
            </span>
          </NuxtLink>
          <div class="ml-auto hidden items-center gap-6 lg:flex">
            <NuxtLink
              to="https://tileserver.app"
              external
              class="font-mono text-xs uppercase tracking-wider text-muted-foreground transition-colors hover:text-foreground"
            >
              Home
            </NuxtLink>
          </div>
        </div>

        <!-- Theme toggle cell (desktop) -->
        <div
          class="hidden items-center justify-center border-l border-border px-4 lg:flex"
        >
          <button
            class="flex size-9 items-center justify-center text-muted-foreground transition-colors hover:text-foreground"
            aria-label="Toggle theme"
            @click="toggleTheme"
          >
            <Sun
              v-if="isDark"
              class="size-4"
            />
            <Moon
              v-else
              class="size-4"
            />
          </button>
        </div>

        <!-- GitHub cell (desktop) / Theme toggle (mobile) -->
        <div
          class="flex items-center justify-center border-l border-border lg:hidden"
        >
          <button
            class="flex items-center justify-center text-muted-foreground hover:text-foreground"
            aria-label="Toggle theme"
            @click="toggleTheme"
          >
            <Sun
              v-if="isDark"
              class="size-4"
            />
            <Moon
              v-else
              class="size-4"
            />
          </button>
        </div>
        <NuxtLink
          to="https://github.com/vinayakkulkarni/tileserver-rs"
          external
          class="hidden items-center justify-center border-l border-border text-muted-foreground transition-colors hover:text-foreground lg:flex"
        >
          <Icon
            name="simple-icons:github"
            class="size-5"
          />
        </NuxtLink>
      </div>
    </nav>

    <!-- Sidebar overlay -->
    <div
      v-if="sidebarOpen"
      class="fixed inset-0 z-30 bg-black/50 lg:hidden"
      @click="closeSidebar()"
    />

    <!-- ═══ Sidebar with geometric borders ═══ -->
    <aside
      :class="[
        'fixed top-[72px] bottom-0 z-40 w-64 overflow-y-auto border-r border-border bg-background px-4 py-6 lg:top-[80px]',
        'transition-transform lg:translate-x-0',
        sidebarOpen ? 'translate-x-0' : '-translate-x-full',
      ]"
    >
      <nav class="space-y-6">
        <div
          v-for="section in sections"
          :key="section.path"
        >
          <p
            class="mb-2 font-mono text-[10px] font-medium uppercase tracking-[0.2em] text-muted-foreground"
          >
            {{ section.title }}
          </p>
          <ul class="space-y-0.5">
            <li
              v-for="item in section.children"
              :key="item.path"
            >
              <NuxtLink
                :to="item.path"
                :class="[
                  'block py-1.5 pl-3 text-sm transition-colors',
                  'border-l',
                  isActive(item.path)
                    ? 'border-primary text-foreground font-medium'
                    : isInSection(section.path)
                      ? 'border-border text-muted-foreground hover:text-foreground hover:border-foreground/30'
                      : 'border-transparent text-muted-foreground hover:text-foreground hover:border-foreground/30',
                ]"
              >
                {{ item.title }}
              </NuxtLink>
            </li>
          </ul>
        </div>
      </nav>
    </aside>

    <!-- Main content -->
    <main class="pt-[72px] lg:pl-64 lg:pt-[80px]">
      <slot />
    </main>
  </div>
</template>
