<script setup lang="ts">
  import { useAdminMcpDevices } from '~/composables/admin/use-admin-mcp-devices';

  definePageMeta({ layout: 'admin' });
  useHead({ title: 'Devices · tileserver-rs admin' });

  const {
    sessions,
    isLoading,
    error,
    isEmpty,
    pendingTokenId,
    confirmTargetId,
    openRevokeConfirm,
    closeRevokeConfirm,
    confirmRevoke,
    formatTimestamp,
    formatRelativeExpiry,
  } = useAdminMcpDevices();
</script>

<template>
  <div class="flex min-h-dvh flex-col">
    <header class="border-b border-border px-10 py-6">
      <p class="font-mono text-[11px] tracking-[0.18em] text-muted-foreground uppercase">
        tileserver-rs / admin / mcp / devices
      </p>
      <h1 class="mt-3 text-2xl font-semibold tracking-tight text-foreground">
        Devices
      </h1>
      <p class="mt-2 max-w-2xl text-sm text-muted-foreground">
        Each row is an active refresh token — one device or browser tab where a
        connected app is logged in. Revoking a row terminates that session
        only; the owning client stays registered.
      </p>
    </header>

    <section class="flex-1 px-10 py-8">
      <div v-if="isLoading" class="font-mono text-xs tracking-wider text-muted-foreground uppercase">
        Loading sessions…
      </div>

      <div
        v-else-if="error"
        class="border border-destructive/60 bg-destructive/10 px-4 py-3 text-sm text-destructive-foreground"
      >
        Failed to load sessions: {{ error.message }}
      </div>

      <div
        v-else-if="isEmpty"
        class="border border-border px-6 py-12 text-center"
      >
        <p class="font-mono text-[11px] tracking-wider text-muted-foreground uppercase">
          No active devices
        </p>
        <p class="mt-3 text-sm text-muted-foreground">
          When a client completes an OAuth code-exchange and receives a refresh
          token, the session will appear here.
        </p>
      </div>

      <div v-else class="border border-border">
        <table class="w-full border-collapse">
          <thead>
            <tr class="border-b border-border bg-card">
              <th class="px-4 py-3 text-left font-mono text-[11px] tracking-wider text-muted-foreground uppercase">
                Token
              </th>
              <th class="px-4 py-3 text-left font-mono text-[11px] tracking-wider text-muted-foreground uppercase">
                Client
              </th>
              <th class="px-4 py-3 text-left font-mono text-[11px] tracking-wider text-muted-foreground uppercase">
                Scope
              </th>
              <th class="px-4 py-3 text-left font-mono text-[11px] tracking-wider text-muted-foreground uppercase">
                Granted
              </th>
              <th class="px-4 py-3 text-right font-mono text-[11px] tracking-wider text-muted-foreground uppercase">
                Expires
              </th>
              <th class="px-4 py-3 text-right font-mono text-[11px] tracking-wider text-muted-foreground uppercase">
                Actions
              </th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="session in sessions"
              :key="session.token_id"
              class="border-b border-border align-middle last:border-b-0 hover:bg-secondary/40"
            >
              <td class="px-4 py-[18px]">
                <code class="font-mono text-[11px] text-foreground">
                  {{ session.token_id.slice(0, 12) }}…{{ session.token_id.slice(-4) }}
                </code>
              </td>
              <td class="px-4 py-[18px]">
                <div class="text-sm font-semibold text-foreground">
                  {{ session.client_name ?? session.client_id }}
                </div>
                <div class="mt-1 font-mono text-[11px] text-muted-foreground">
                  {{ session.client_id }}
                </div>
              </td>
              <td class="px-4 py-[18px]">
                <span class="border border-border bg-card px-2 py-0.5 font-mono text-[11px] text-foreground">
                  {{ session.scope }}
                </span>
              </td>
              <td class="px-4 py-[18px] font-mono text-[11px] text-muted-foreground">
                {{ formatTimestamp(session.granted_at) }}
              </td>
              <td class="px-4 py-[18px] text-right font-mono text-[11px] text-muted-foreground tabular-nums">
                {{ formatRelativeExpiry(session.expires_at) }}
              </td>
              <td class="px-4 py-[18px] text-right">
                <button
                  type="button"
                  class="font-mono text-[11px] tracking-wider text-destructive uppercase transition-colors hover:text-destructive-foreground disabled:cursor-not-allowed disabled:opacity-50"
                  :disabled="pendingTokenId !== null"
                  @click="openRevokeConfirm(session.token_id)"
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
        class="admin-theme fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm"
        @click.self="closeRevokeConfirm"
      >
        <div class="w-full max-w-md border border-border bg-card p-6">
          <p class="font-mono text-[11px] tracking-wider text-muted-foreground uppercase">
            Confirm revoke
          </p>
          <h2 class="mt-3 text-lg font-semibold text-foreground">
            Revoke this device?
          </h2>
          <p class="mt-2 text-sm text-muted-foreground">
            The refresh token will be deleted. The device's current access
            token continues to work until it expires (typically 1 hour);
            after that, the client must re-authenticate.
          </p>
          <div class="mt-6 flex justify-end gap-3">
            <button
              type="button"
              class="border border-border px-4 py-2 font-mono text-[11px] tracking-wider text-muted-foreground uppercase transition-colors hover:bg-secondary hover:text-foreground"
              :disabled="pendingTokenId !== null"
              @click="closeRevokeConfirm"
            >
              Cancel
            </button>
            <button
              type="button"
              class="border border-destructive bg-destructive px-4 py-2 font-mono text-[11px] tracking-wider text-destructive-foreground uppercase transition-colors hover:bg-destructive/80 disabled:cursor-not-allowed disabled:opacity-60"
              :disabled="pendingTokenId !== null"
              @click="confirmRevoke"
            >
              {{ pendingTokenId !== null ? 'Revoking…' : 'Revoke' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
