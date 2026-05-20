<script setup lang="ts">
  import {
    ArrowUpRight,
    Check,
    Cpu,
    Layers,
    Map,
    Plug,
    RotateCcw,
    Smartphone,
    TriangleAlert,
  } from '@lucide/vue';
  import { useAdminDashboard } from '~/composables/admin/use-admin-dashboard';
  import { useReloadConfigMutation } from '~/utils/api/server';

  definePageMeta({ layout: 'admin' });
  useHead({ title: 'Admin · tileserver-rs' });

  const {
    breadcrumbs,
    isLoading,
    pingError,
    friendlyPingError,
    uptimeLabel,
    loadedSources,
    loadedStyles,
    rendererEnabled,
    versionLabel,
    configHashShort,
    clientsCount,
    sessionsCount,
    recentClients,
    clientsAreEmpty,
    clientsAreLoading,
    RECENT_SKELETON_ROWS,
    formatLastSeen,
  } = useAdminDashboard();

  const reload = useReloadConfigMutation();

  async function onReload() {
    try {
      await reload.mutateAsync();
    } catch {
      // Surfaced via reload.isError / reload.error in the template.
    }
  }
</script>

<template>
  <div class="flex min-h-dvh flex-col">
    <header class="border-b border-border px-10 py-6">
      <AdminBreadcrumb :items="breadcrumbs" />
      <div class="mt-3 flex items-start justify-between gap-6">
        <div>
          <h1
            class="font-display text-5xl font-semibold tracking-tight text-foreground"
          >
            Operator console
          </h1>
          <p class="mt-2 max-w-2xl text-sm text-muted-foreground">
            Runtime status, connected MCP clients, and active device sessions
            for this tileserver-rs instance.
          </p>
        </div>
        <div class="flex shrink-0 flex-col items-end gap-2">
          <button
            type="button"
            :disabled="reload.isPending.value"
            class="flex items-center gap-2 border border-border bg-card px-4 py-2 text-sm text-foreground transition-colors hover:bg-secondary disabled:cursor-not-allowed disabled:opacity-60"
            @click="onReload"
          >
            <RotateCcw
              :class="[
                'size-4',
                reload.isPending.value ? 'animate-spin' : '',
              ]"
            />
            {{ reload.isPending.value ? 'Reloading…' : 'Reload config' }}
          </button>
          <p
            v-if="reload.isSuccess.value"
            class="flex items-center gap-1.5 font-mono text-[11px] tracking-wider text-success uppercase"
          >
            <Check class="size-3" />
            Config reloaded
          </p>
          <p
            v-else-if="reload.isError.value"
            class="flex max-w-xs items-center gap-1.5 font-mono text-[11px] tracking-wider text-destructive uppercase"
          >
            <TriangleAlert class="size-3" />
            Reload failed
          </p>
        </div>
      </div>
    </header>

    <section v-if="pingError" class="border-b border-border px-10 py-8">
      <div class="max-w-2xl border border-border px-6 py-6">
        <p
          class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase"
        >
          {{ friendlyPingError.title }}
        </p>
        <p class="mt-3 text-sm text-foreground">
          {{ friendlyPingError.body }}
        </p>
        <p
          v-if="friendlyPingError.hint"
          class="mt-2 text-sm text-muted-foreground"
        >
          {{ friendlyPingError.hint }}
        </p>
      </div>
    </section>

    <section v-else class="border-b border-border">
      <div class="grid grid-cols-1 lg:grid-cols-[3fr_2fr]">
        <div
          class="flex flex-col justify-between border-b border-border px-10 py-8 lg:border-r lg:border-b-0"
        >
          <p
            class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase"
          >
            Uptime
          </p>
          <div v-if="isLoading" class="mt-4">
            <Skeleton class="h-16 w-48" />
          </div>
          <p
            v-else
            class="mt-4 font-display text-7xl font-semibold tabular-nums tracking-tight text-foreground"
          >
            {{ uptimeLabel }}
          </p>
          <p class="mt-4 font-mono text-[11px] text-muted-foreground">
            since last config reload
          </p>
        </div>

        <div class="grid grid-cols-2 grid-rows-2">
          <div class="border-b border-border px-6 py-6">
            <p
              class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase"
            >
              Sources
            </p>
            <p
              class="mt-3 font-display text-3xl font-semibold tabular-nums text-foreground"
            >
              {{ loadedSources }}
            </p>
          </div>
          <div class="border-b border-l border-border px-6 py-6">
            <p
              class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase"
            >
              Styles
            </p>
            <p
              class="mt-3 font-display text-3xl font-semibold tabular-nums text-foreground"
            >
              {{ loadedStyles }}
            </p>
          </div>
          <div class="px-6 py-6">
            <p
              class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase"
            >
              Clients
            </p>
            <p
              class="mt-3 font-display text-3xl font-semibold tabular-nums text-foreground"
            >
              {{ clientsCount }}
            </p>
          </div>
          <div class="border-l border-border px-6 py-6">
            <p
              class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase"
            >
              Devices
            </p>
            <p
              class="mt-3 font-display text-3xl font-semibold tabular-nums text-foreground"
            >
              {{ sessionsCount }}
            </p>
          </div>
        </div>
      </div>
    </section>

    <section class="grid flex-1 grid-cols-1 lg:grid-cols-[2fr_1fr]">
      <div class="border-b border-border px-10 py-8 lg:border-r lg:border-b-0">
        <div class="flex items-baseline justify-between">
          <p
            class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase"
          >
            Recent activity
          </p>
          <NuxtLink
            to="/admin/mcp/connected-apps"
            class="flex items-center gap-1 font-mono text-[11px] tracking-wider text-muted-foreground uppercase transition-colors hover:text-foreground"
          >
            View all clients <ArrowUpRight class="size-3" />
          </NuxtLink>
        </div>

        <ul
          v-if="clientsAreLoading"
          class="mt-4 divide-y divide-border border border-border"
          aria-busy="true"
          aria-label="Loading recent clients"
        >
          <li
            v-for="n in RECENT_SKELETON_ROWS"
            :key="n"
            class="flex items-baseline justify-between gap-4 px-4 py-3"
          >
            <div class="min-w-0 flex-1 space-y-2">
              <Skeleton class="h-4 w-40" />
              <Skeleton class="h-3 w-56" />
            </div>
            <Skeleton class="h-3 w-16 shrink-0" />
          </li>
        </ul>

        <div
          v-else-if="clientsAreEmpty"
          class="mt-6 border border-border px-6 py-10"
        >
          <p
            class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase"
          >
            Nothing to show
          </p>
          <p class="mt-2 max-w-md text-sm text-muted-foreground">
            No MCP clients have completed Dynamic Client Registration yet. Once
            one does, it appears here and on the Connected apps page.
          </p>
        </div>

        <ul v-else class="mt-4 divide-y divide-border border border-border">
          <li
            v-for="client in recentClients"
            :key="client.client_id"
            class="flex items-baseline justify-between gap-4 px-4 py-3"
          >
            <div class="min-w-0">
              <p class="truncate text-sm font-semibold text-foreground">
                {{ client.client_name ?? client.client_id }}
              </p>
              <p
                class="mt-1 truncate font-mono text-[11px] text-muted-foreground"
              >
                {{ client.client_id }}
              </p>
            </div>
            <p
              class="shrink-0 font-mono text-[11px] text-muted-foreground tabular-nums"
            >
              {{ formatLastSeen(client.last_seen_at) }}
            </p>
          </li>
        </ul>
      </div>

      <aside class="flex flex-col">
        <NuxtLink
          to="/admin/mcp/connected-apps"
          class="group flex items-start gap-4 border-b border-border px-6 py-6 transition-colors hover:bg-secondary/40"
        >
          <div
            class="flex size-10 shrink-0 items-center justify-center border border-border bg-card"
          >
            <Plug
              class="size-4 text-muted-foreground group-hover:text-foreground"
            />
          </div>
          <div class="min-w-0 flex-1">
            <p class="text-sm font-semibold text-foreground">Connected apps</p>
            <p class="mt-1 text-sm text-muted-foreground">
              Manage OAuth clients, audit scopes, revoke access.
            </p>
          </div>
          <ArrowUpRight
            class="mt-1 size-4 text-muted-foreground group-hover:text-foreground"
          />
        </NuxtLink>

        <NuxtLink
          to="/admin/mcp/devices"
          class="group flex items-start gap-4 border-b border-border px-6 py-6 transition-colors hover:bg-secondary/40"
        >
          <div
            class="flex size-10 shrink-0 items-center justify-center border border-border bg-card"
          >
            <Smartphone
              class="size-4 text-muted-foreground group-hover:text-foreground"
            />
          </div>
          <div class="min-w-0 flex-1">
            <p class="text-sm font-semibold text-foreground">Devices</p>
            <p class="mt-1 text-sm text-muted-foreground">
              Active refresh tokens, per-device session revocation.
            </p>
          </div>
          <ArrowUpRight
            class="mt-1 size-4 text-muted-foreground group-hover:text-foreground"
          />
        </NuxtLink>

        <div class="flex-1 border-b border-border px-6 py-6">
          <p
            class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase"
          >
            Build
          </p>
          <dl class="mt-3 grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
            <dt
              class="font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
            >
              Version
            </dt>
            <dd class="font-mono text-foreground">v{{ versionLabel }}</dd>
            <dt
              class="font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
            >
              Config
            </dt>
            <dd class="font-mono text-foreground">{{ configHashShort }}</dd>
            <dt
              class="font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
            >
              Renderer
            </dt>
            <dd class="flex items-center gap-2 font-mono text-foreground">
              <span
                :class="rendererEnabled ? 'bg-success' : 'bg-muted-foreground'"
                class="size-1.5"
              ></span>
              {{ rendererEnabled ? 'enabled' : 'disabled' }}
            </dd>
          </dl>
        </div>

        <div class="px-6 py-6">
          <p
            class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase"
          >
            Endpoints
          </p>
          <ul class="mt-3 grid grid-cols-1 gap-2 text-sm">
            <li class="flex items-center gap-2">
              <Map class="size-3.5 text-muted-foreground" />
              <a
                href="/data.json"
                target="_blank"
                rel="noopener"
                class="font-mono text-foreground transition-colors hover:text-primary"
                >/data.json</a
              >
            </li>
            <li class="flex items-center gap-2">
              <Layers class="size-3.5 text-muted-foreground" />
              <a
                href="/styles.json"
                target="_blank"
                rel="noopener"
                class="font-mono text-foreground transition-colors hover:text-primary"
                >/styles.json</a
              >
            </li>
            <li class="flex items-center gap-2">
              <Cpu class="size-3.5 text-muted-foreground" />
              <a
                href="/ping"
                target="_blank"
                rel="noopener"
                class="font-mono text-foreground transition-colors hover:text-primary"
                >/ping</a
              >
            </li>
          </ul>
        </div>
      </aside>
    </section>
  </div>
</template>
