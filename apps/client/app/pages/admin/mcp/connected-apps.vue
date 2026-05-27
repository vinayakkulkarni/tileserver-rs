<script setup lang="ts">
  import { Plug } from '@lucide/vue';
  import { useAdminMcpConnectedApps } from '~/composables/admin/use-admin-mcp-connected-apps';

  definePageMeta({ layout: 'admin' });
  useHead({ title: 'Connected apps · tileserver-rs admin' });

  const {
    breadcrumbs,
    clients,
    isLoading,
    error,
    friendly,
    isEmpty,
    pendingClientId,
    confirmTargetId,
    openRevokeConfirm,
    closeRevokeConfirm,
    confirmRevoke,
    visibleScopes,
    overflowScopes,
    formatTimestamp,
    SKELETON_ROWS,
  } = useAdminMcpConnectedApps();
</script>

<template>
  <div class="flex min-h-dvh flex-col">
    <header
      class="border-b border-border px-[clamp(16px,4vw,40px)] py-5 sm:py-6"
    >
      <AdminBreadcrumb :items="breadcrumbs" />
      <h1 class="mt-3 text-2xl font-semibold tracking-tight text-foreground">
        Connected apps
      </h1>
      <p class="mt-2 max-w-2xl text-sm text-muted-foreground">
        OAuth clients registered with this server's MCP endpoint. Each client
        represents a third-party app (Claude desktop, Claude.ai, Cursor, etc.)
        that has been granted access via Dynamic Client Registration.
      </p>
    </header>

    <section class="flex-1 px-[clamp(16px,4vw,40px)] py-6 sm:py-8">
      <div v-if="isLoading" class="overflow-x-auto border border-border">
        <table class="w-full min-w-[640px] border-collapse">
          <thead>
            <tr class="border-b border-border bg-card">
              <th
                class="px-4 py-3 text-left font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
              >
                Client
              </th>
              <th
                class="px-4 py-3 text-left font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
              >
                Scopes
              </th>
              <th
                class="px-4 py-3 text-right font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
              >
                Sessions
              </th>
              <th
                class="px-4 py-3 text-left font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
              >
                Last seen
              </th>
              <th
                class="px-4 py-3 text-right font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
              >
                Actions
              </th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="row in SKELETON_ROWS"
              :key="row"
              class="border-b border-border align-middle last:border-b-0"
            >
              <td class="px-4 py-[18px]">
                <Skeleton class="h-4 w-40" />
                <Skeleton class="mt-2 h-3 w-28" />
              </td>
              <td class="px-4 py-[18px]">
                <div class="flex flex-wrap items-center gap-1.5">
                  <Skeleton class="h-5 w-16" />
                  <Skeleton class="h-5 w-20" />
                  <Skeleton class="h-5 w-14" />
                </div>
              </td>
              <td class="px-4 py-[18px] text-right">
                <Skeleton class="ml-auto h-4 w-6" />
              </td>
              <td class="px-4 py-[18px]">
                <Skeleton class="h-3 w-24" />
              </td>
              <td class="px-4 py-[18px] text-right">
                <Skeleton class="ml-auto h-3 w-16" />
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-else-if="error" class="border border-border px-6 py-8">
        <p
          class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase"
        >
          {{ friendly.title }}
        </p>
        <p class="mt-3 max-w-2xl text-sm text-foreground">
          {{ friendly.body }}
        </p>
        <p
          v-if="friendly.hint"
          class="mt-2 max-w-2xl text-sm text-muted-foreground"
        >
          {{ friendly.hint }}
        </p>
      </div>

      <div v-else-if="isEmpty" class="border border-border px-8 py-16">
        <div class="mx-auto flex max-w-xl flex-col items-start gap-6">
          <div
            class="flex size-12 items-center justify-center border border-border bg-card"
          >
            <Plug class="size-5 text-muted-foreground" />
          </div>
          <div>
            <p
              class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase"
            >
              Zero clients registered
            </p>
            <h2
              class="mt-3 text-3xl font-semibold tracking-tight text-foreground"
            >
              Nothing's connected yet.
            </h2>
            <p class="mt-3 text-sm text-muted-foreground">
              A connected app appears here after it completes Dynamic Client
              Registration against
              <code class="font-mono text-foreground">/oauth/register</code>.
              From there you can audit its scopes and revoke access.
            </p>
          </div>
        </div>
      </div>

      <div v-else class="overflow-x-auto border border-border">
        <table class="w-full min-w-[640px] border-collapse">
          <thead>
            <tr class="border-b border-border bg-card">
              <th
                class="px-4 py-3 text-left font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
              >
                Client
              </th>
              <th
                class="px-4 py-3 text-left font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
              >
                Scopes
              </th>
              <th
                class="px-4 py-3 text-right font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
              >
                Sessions
              </th>
              <th
                class="px-4 py-3 text-left font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
              >
                Last seen
              </th>
              <th
                class="px-4 py-3 text-right font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
              >
                Actions
              </th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="client in clients"
              :key="client.client_id"
              class="border-b border-border align-middle last:border-b-0 hover:bg-secondary/40"
            >
              <td class="px-4 py-[18px]">
                <div class="text-sm font-semibold text-foreground">
                  {{ client.client_name ?? client.client_id }}
                </div>
                <div class="mt-1 font-mono text-[11px] text-muted-foreground">
                  {{ client.client_id }}
                </div>
              </td>
              <td class="px-4 py-[18px]">
                <div class="flex flex-wrap items-center gap-1.5">
                  <span
                    v-for="scope in visibleScopes(client)"
                    :key="scope"
                    class="border border-border bg-card px-2 py-0.5 font-mono text-[11px] text-foreground"
                  >
                    {{ scope }}
                  </span>
                  <Popover v-if="overflowScopes(client).length > 0">
                    <PopoverTrigger
                      class="border border-border bg-secondary px-2 py-0.5 font-mono text-[11px] text-muted-foreground transition-colors hover:text-foreground"
                    >
                      +{{ overflowScopes(client).length }} more
                    </PopoverTrigger>
                    <PopoverContent class="w-72">
                      <p
                        class="font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
                      >
                        All scopes
                      </p>
                      <div class="mt-3 flex flex-wrap gap-1.5">
                        <span
                          v-for="scope in client.scopes"
                          :key="scope"
                          class="border border-border bg-card px-2 py-0.5 font-mono text-[11px] text-foreground"
                        >
                          {{ scope }}
                        </span>
                      </div>
                    </PopoverContent>
                  </Popover>
                </div>
              </td>
              <td class="px-4 py-[18px] text-right">
                <span class="font-mono text-sm tabular-nums text-foreground">
                  {{ client.active_sessions }}
                </span>
              </td>
              <td
                class="px-4 py-[18px] font-mono text-[11px] text-muted-foreground"
              >
                {{ formatTimestamp(client.last_seen_at) }}
              </td>
              <td class="px-4 py-[18px] text-right">
                <button
                  type="button"
                  class="font-mono text-[11px] tracking-wider text-destructive uppercase transition-colors hover:text-destructive-foreground disabled:cursor-not-allowed disabled:opacity-50"
                  :disabled="pendingClientId !== null"
                  @click="openRevokeConfirm(client.client_id)"
                >
                  × Revoke
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </section>

    <Teleport v-if="confirmTargetId" to="body">
      <div
        class="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm"
        @click.self="closeRevokeConfirm"
      >
        <div class="w-full max-w-md border border-border bg-card p-6">
          <p
            class="font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
          >
            Confirm revoke
          </p>
          <h2 class="mt-3 text-lg font-semibold text-foreground">
            Revoke this client?
          </h2>
          <p class="mt-2 text-sm text-muted-foreground">
            All active sessions for
            <code class="font-mono text-foreground">{{ confirmTargetId }}</code>
            will be terminated. The client will need to re-register to regain
            access.
          </p>
          <div class="mt-6 flex justify-end gap-3">
            <button
              type="button"
              class="border border-border px-4 py-2 font-mono text-[11px] tracking-wider text-muted-foreground uppercase transition-colors hover:bg-secondary hover:text-foreground"
              :disabled="pendingClientId !== null"
              @click="closeRevokeConfirm"
            >
              Cancel
            </button>
            <button
              type="button"
              class="border border-destructive bg-destructive px-4 py-2 font-mono text-[11px] tracking-wider text-destructive-foreground uppercase transition-colors hover:bg-destructive/80 disabled:cursor-not-allowed disabled:opacity-60"
              :disabled="pendingClientId !== null"
              @click="confirmRevoke"
            >
              {{ pendingClientId !== null ? 'Revoking…' : 'Revoke' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
