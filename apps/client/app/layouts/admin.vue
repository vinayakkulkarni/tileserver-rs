<script setup lang="ts">
  import { Menu, Moon, Sun, X } from '@lucide/vue';
  import { useEventListener } from '@vueuse/core';
  import { useAdminLayout } from '~/composables/admin/use-admin-layout';

  const {
    navGroups,
    isActive,
    isDark,
    toggleTheme,
    isMobileNavOpen,
    openMobileNav,
    closeMobileNav,
  } = useAdminLayout();

  useEventListener('keydown', (event: KeyboardEvent) => {
    if (event.key === 'Escape' && isMobileNavOpen.value) {
      closeMobileNav();
    }
  });
</script>

<template>
  <div class="flex h-dvh w-full overflow-hidden bg-background">
    <!-- Mobile top bar (<lg only) — brand left + hamburger right. -->
    <header
      class="fixed inset-x-0 top-0 z-40 flex h-14 shrink-0 items-center justify-between border-b border-border bg-background px-4 lg:hidden"
      role="banner"
    >
      <NuxtLink to="/admin" class="flex items-center gap-2.5">
        <div
          class="flex size-7 items-center justify-center bg-primary text-primary-foreground"
        >
          <span class="font-display text-sm font-bold">T</span>
        </div>
        <div class="flex flex-col leading-tight">
          <span
            class="font-mono text-[10px] uppercase tracking-wider text-muted-foreground"
            >tileserver-rs</span
          >
          <span class="text-[13px] font-semibold text-foreground">Admin</span>
        </div>
      </NuxtLink>
      <button
        type="button"
        aria-label="Open navigation menu"
        aria-controls="admin-mobile-drawer"
        :aria-expanded="isMobileNavOpen"
        class="flex size-10 items-center justify-center border border-border text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
        @click="openMobileNav"
      >
        <Menu class="size-5" />
      </button>
    </header>

    <!-- Mobile drawer backdrop (<lg) — closes on tap. -->
    <Transition
      enter-from-class="opacity-0"
      leave-to-class="opacity-0"
      enter-active-class="transition-opacity duration-200"
      leave-active-class="transition-opacity duration-200"
    >
      <button
        v-if="isMobileNavOpen"
        type="button"
        aria-label="Close navigation menu"
        class="fixed inset-0 z-40 bg-foreground/40 lg:hidden"
        @click="closeMobileNav"
      ></button>
    </Transition>

    <!-- Sidebar — desktop persistent, mobile drawer (slide-in from left). -->
    <Transition
      enter-from-class="-translate-x-full"
      leave-to-class="-translate-x-full"
      enter-active-class="transition-transform duration-200 ease-out"
      leave-active-class="transition-transform duration-200 ease-out"
    >
      <aside
        v-if="isMobileNavOpen"
        id="admin-mobile-drawer"
        class="fixed left-0 top-0 z-50 flex h-dvh w-72 flex-col border-r border-border bg-background lg:hidden"
        role="dialog"
        aria-modal="true"
        aria-label="Navigation menu"
      >
        <div
          class="flex items-center justify-between border-b border-border px-5 py-4"
        >
          <div class="flex items-center gap-3">
            <div
              class="flex size-7 items-center justify-center bg-primary text-primary-foreground"
            >
              <span class="font-display text-sm font-bold">T</span>
            </div>
            <div class="flex flex-col leading-tight">
              <span
                class="font-mono text-[11px] uppercase tracking-wider text-muted-foreground"
                >tileserver-rs</span
              >
              <span class="text-sm font-semibold text-foreground">Admin</span>
            </div>
          </div>
          <button
            type="button"
            aria-label="Close navigation menu"
            class="flex size-9 items-center justify-center border border-border text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
            @click="closeMobileNav"
          >
            <X class="size-4" />
          </button>
        </div>

        <nav class="flex-1 space-y-4 overflow-y-auto px-3 py-4">
          <div
            v-for="(group, gIdx) in navGroups"
            :key="group.heading ?? gIdx"
            class="space-y-1"
          >
            <p
              v-if="group.heading"
              class="px-3 pb-2 font-mono text-[10px] uppercase tracking-[0.18em] text-muted-foreground"
            >
              {{ group.heading }}
            </p>
            <NuxtLink
              v-for="item in group.items"
              :key="item.to"
              :to="item.to"
              :class="[
                'flex items-center gap-3 px-3 py-3 text-sm transition-colors',
                isActive(item.to)
                  ? 'bg-accent text-accent-foreground'
                  : 'text-muted-foreground hover:bg-secondary hover:text-foreground',
              ]"
            >
              <component :is="item.icon" class="size-4" />
              <span>{{ item.label }}</span>
            </NuxtLink>
          </div>
        </nav>

        <div
          class="flex items-center justify-between gap-2 border-t border-border px-3 py-3"
        >
          <NuxtLink
            to="/"
            class="px-3 py-2 font-mono text-[11px] uppercase tracking-wider text-muted-foreground transition-colors hover:text-foreground"
          >
            ← Back to viewer
          </NuxtLink>
          <button
            type="button"
            :aria-label="
              isDark ? 'Switch to light mode' : 'Switch to dark mode'
            "
            class="flex size-9 items-center justify-center border border-border text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
            @click="toggleTheme"
          >
            <Sun v-if="isDark" class="size-4" />
            <Moon v-else class="size-4" />
          </button>
        </div>
      </aside>
    </Transition>

    <!-- Desktop sidebar — persistent rail (>=lg). -->
    <aside
      class="hidden h-dvh w-64 shrink-0 flex-col border-r border-border bg-background lg:flex"
    >
      <div class="flex items-center gap-3 border-b border-border px-6 py-5">
        <div
          class="flex size-7 items-center justify-center bg-primary text-primary-foreground"
        >
          <span class="font-display text-sm font-bold">T</span>
        </div>
        <div class="flex flex-col leading-tight">
          <span
            class="font-mono text-[11px] uppercase tracking-wider text-muted-foreground"
            >tileserver-rs</span
          >
          <span class="text-sm font-semibold text-foreground">Admin</span>
        </div>
      </div>

      <nav class="flex-1 space-y-4 px-3 py-4">
        <div
          v-for="(group, gIdx) in navGroups"
          :key="group.heading ?? gIdx"
          class="space-y-1"
        >
          <p
            v-if="group.heading"
            class="px-3 pb-2 font-mono text-[10px] uppercase tracking-[0.18em] text-muted-foreground"
          >
            {{ group.heading }}
          </p>
          <NuxtLink
            v-for="item in group.items"
            :key="item.to"
            :to="item.to"
            :class="[
              'flex items-center gap-3 px-3 py-2 text-sm transition-colors',
              isActive(item.to)
                ? 'bg-accent text-accent-foreground'
                : 'text-muted-foreground hover:bg-secondary hover:text-foreground',
            ]"
          >
            <component :is="item.icon" class="size-4" />
            <span>{{ item.label }}</span>
          </NuxtLink>
        </div>
      </nav>

      <div
        class="flex items-center justify-between gap-2 border-t border-border px-3 py-3"
      >
        <NuxtLink
          to="/"
          class="px-3 py-2 font-mono text-[11px] uppercase tracking-wider text-muted-foreground transition-colors hover:text-foreground"
        >
          ← Back to viewer
        </NuxtLink>
        <button
          type="button"
          :aria-label="isDark ? 'Switch to light mode' : 'Switch to dark mode'"
          class="flex size-9 items-center justify-center border border-border text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
          @click="toggleTheme"
        >
          <Sun v-if="isDark" class="size-4" />
          <Moon v-else class="size-4" />
        </button>
      </div>
    </aside>

    <main class="min-w-0 flex-1 overflow-y-auto bg-background pt-14 lg:pt-0">
      <slot></slot>
    </main>
  </div>
</template>
