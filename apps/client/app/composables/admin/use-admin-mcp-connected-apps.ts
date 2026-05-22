import { useQuery } from '@tanstack/vue-query';
import type { AdminBreadcrumbCrumb, AdminMcpClient } from '~/types';
import {
  adminMcpClientsQueryOptions,
  useDeleteAdminMcpClientMutation,
} from '~/utils/api/admin-mcp';
import { friendlyAdminError } from '~/utils/api/admin-mcp/friendly-error';

const SCOPE_VISIBLE_LIMIT = 3;
const SKELETON_ROWS = 5;

const BREADCRUMBS: AdminBreadcrumbCrumb[] = [
  { label: 'Home', to: '/' },
  { label: 'Admin', to: '/admin' },
  { label: 'Connected apps' },
];

export function useAdminMcpConnectedApps() {
  const clientsQuery = useQuery(adminMcpClientsQueryOptions());
  const deleteMutation = useDeleteAdminMcpClientMutation();

  const clients = computed<AdminMcpClient[]>(
    () => clientsQuery.data.value ?? [],
  );
  const isLoading = computed(() => clientsQuery.isPending.value);
  const error = computed(() => clientsQuery.error.value);
  const friendly = computed(() => friendlyAdminError(error.value));
  const isEmpty = computed(
    () => !isLoading.value && !error.value && clients.value.length === 0,
  );

  const pendingClientId = ref<string | null>(null);
  const confirmTargetId = ref<string | null>(null);

  function openRevokeConfirm(clientId: string): void {
    confirmTargetId.value = clientId;
  }
  function closeRevokeConfirm(): void {
    confirmTargetId.value = null;
  }

  async function confirmRevoke(): Promise<void> {
    const id = confirmTargetId.value;
    if (!id) return;
    pendingClientId.value = id;
    try {
      await deleteMutation.mutateAsync(id);
    } finally {
      pendingClientId.value = null;
      confirmTargetId.value = null;
    }
  }

  function visibleScopes(client: AdminMcpClient): string[] {
    return client.scopes.slice(0, SCOPE_VISIBLE_LIMIT);
  }

  function overflowScopes(client: AdminMcpClient): string[] {
    return client.scopes.length > SCOPE_VISIBLE_LIMIT
      ? client.scopes.slice(SCOPE_VISIBLE_LIMIT)
      : [];
  }

  function formatTimestamp(unixSecs: number | null): string {
    if (unixSecs === null) return '—';
    return new Date(unixSecs * 1000).toLocaleString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  return {
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
    SCOPE_VISIBLE_LIMIT,
    SKELETON_ROWS,
    breadcrumbs: BREADCRUMBS,
  };
}
