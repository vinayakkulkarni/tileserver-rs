<script setup lang="ts">
  import { Moon, Sun } from '@lucide/vue';
  import { useAdminLayout } from '~/composables/admin/use-admin-layout';

  const { navGroups, isActive, isDark, toggleTheme } = useAdminLayout();
</script>

<template>
  <div class="flex min-h-dvh w-full bg-background">
    <aside
      class="sticky top-0 flex h-dvh w-64 shrink-0 flex-col self-start border-r border-border bg-background"
    >
      <div class="flex items-center gap-3 border-b border-border px-6 py-5">
        <div
          class="flex size-7 items-center justify-center bg-primary text-primary-foreground"
        >
          <span class="font-display text-sm font-bold">T</span>
        </div>
        <div class="flex flex-col leading-tight">
          <span
            class="font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
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
            class="px-3 pb-2 font-mono text-[10px] tracking-[0.18em] text-muted-foreground uppercase"
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
          class="px-3 py-2 font-mono text-[11px] tracking-wider text-muted-foreground uppercase transition-colors hover:text-foreground"
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

    <main class="min-w-0 flex-1 bg-background">
      <slot></slot>
    </main>
  </div>
</template>
