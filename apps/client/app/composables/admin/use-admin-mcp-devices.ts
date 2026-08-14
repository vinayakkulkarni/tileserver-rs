import { useQuery } from '@tanstack/vue-query';
import type { AdminBreadcrumbCrumb, AdminMcpSession } from '~/types';
import {
  adminMcpSessionsQueryOptions,
  useDeleteAdminMcpSessionMutation,
} from '~/utils/api/admin-mcp';
import { friendlyAdminError } from '~/utils/api/admin-mcp/friendly-error';

const SKELETON_ROWS = 5;

/** Stable empty fallback so the computed returns the same reference across recomputes. */
const EMPTY_SESSIONS: AdminMcpSession[] = [];

const BREADCRUMBS: AdminBreadcrumbCrumb[] = [
  { label: 'Home', to: '/' },
  { label: 'Admin', to: '/admin' },
  { label: 'Devices' },
];

export function useAdminMcpDevices() {
  const sessionsQuery = useQuery(adminMcpSessionsQueryOptions());
  const deleteMutation = useDeleteAdminMcpSessionMutation();

  const sessions = computed<AdminMcpSession[]>(
    () => sessionsQuery.data.value ?? EMPTY_SESSIONS,
  );
  const isLoading = computed(() => sessionsQuery.isPending.value);
  const error = computed(() => sessionsQuery.error.value);
  const friendly = computed(() => friendlyAdminError(error.value));
  const isEmpty = computed(
    () => !isLoading.value && !error.value && sessions.value.length === 0,
  );

  const pendingTokenId = ref<string | null>(null);
  const confirmTargetId = ref<string | null>(null);

  function openRevokeConfirm(tokenId: string): void {
    confirmTargetId.value = tokenId;
  }
  function closeRevokeConfirm(): void {
    confirmTargetId.value = null;
  }

  async function confirmRevoke(): Promise<void> {
    const id = confirmTargetId.value;
    if (!id) return;
    pendingTokenId.value = id;
    try {
      await deleteMutation.mutateAsync(id);
    } finally {
      pendingTokenId.value = null;
      confirmTargetId.value = null;
    }
  }

  function formatTimestamp(unixSecs: number): string {
    return new Date(unixSecs * 1000).toLocaleString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function formatRelativeExpiry(unixSecs: number): string {
    const deltaMs = unixSecs * 1000 - Date.now();
    const deltaMinutes = Math.round(deltaMs / 60_000);
    if (deltaMinutes < 0) return `expired ${Math.abs(deltaMinutes)}m ago`;
    if (deltaMinutes < 60) return `${deltaMinutes}m`;
    const hours = Math.round(deltaMinutes / 60);
    if (hours < 48) return `${hours}h`;
    const days = Math.round(hours / 24);
    return `${days}d`;
  }

  return {
    sessions,
    isLoading,
    error,
    friendly,
    isEmpty,
    pendingTokenId,
    confirmTargetId,
    openRevokeConfirm,
    closeRevokeConfirm,
    confirmRevoke,
    formatTimestamp,
    formatRelativeExpiry,
    SKELETON_ROWS,
    breadcrumbs: BREADCRUMBS,
  };
}
