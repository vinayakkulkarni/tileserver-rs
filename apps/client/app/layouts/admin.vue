<script setup lang="ts">
import { Plug, Smartphone } from '@lucide/vue';

interface AdminNavItem {
  label: string;
  to: string;
  icon: typeof Plug;
}

const nav: AdminNavItem[] = [
  { label: 'Connected apps', to: '/admin/mcp/connected-apps', icon: Plug },
  { label: 'Devices', to: '/admin/mcp/devices', icon: Smartphone },
];

const route = useRoute();

function isActive(to: string): boolean {
  return route.path === to || route.path.startsWith(`${to}/`);
}
</script>

<template>
  <div class="admin-theme flex min-h-dvh">
    <aside
      class="sticky top-0 flex h-dvh w-64 shrink-0 flex-col border-r border-border bg-background"
    >
      <div class="flex items-center gap-3 border-b border-border px-6 py-5">
        <div
          class="flex size-7 items-center justify-center bg-primary text-primary-foreground"
        >
          <span class="font-display text-sm font-bold">T</span>
        </div>
        <div class="flex flex-col leading-tight">
          <span class="font-mono text-[11px] tracking-wider text-muted-foreground uppercase">tileserver-rs</span>
          <span class="text-sm font-semibold text-foreground">Admin</span>
        </div>
      </div>

      <nav class="flex-1 space-y-1 px-3 py-4">
        <p class="px-3 pb-2 font-mono text-[10px] tracking-[0.18em] text-muted-foreground uppercase">
          MCP
        </p>
        <NuxtLink
          v-for="item in nav"
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
      </nav>

      <div class="border-t border-border px-6 py-4">
        <NuxtLink
          to="/"
          class="font-mono text-[11px] tracking-wider text-muted-foreground uppercase hover:text-foreground"
        >
          ← Back to viewer
        </NuxtLink>
      </div>
    </aside>

    <main class="min-w-0 flex-1 bg-background">
      <slot ></slot>
    </main>
  </div>
</template>
