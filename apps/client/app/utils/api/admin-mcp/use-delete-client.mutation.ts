/**
 * The server cascade-deletes every refresh token issued to the deleted
 * client, so this mutation invalidates BOTH the clients and sessions
 * caches. `deleted: false` is a normal idempotent success case, not an
 * error — callers must not throw on it.
 */

import { useMutation, useQueryClient } from '@tanstack/vue-query';
import type { AdminMcpDeleteResponse } from '~/types';
import { ADMIN_MCP_QUERY_KEYS } from '~/utils/query-keys';

export function useDeleteAdminMcpClientMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (clientId: string): Promise<AdminMcpDeleteResponse> => {
      return $fetch<AdminMcpDeleteResponse>(
        `/__admin/oauth/clients/${encodeURIComponent(clientId)}`,
        {
          method: 'DELETE',
        },
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ADMIN_MCP_QUERY_KEYS.clients(),
      });
      queryClient.invalidateQueries({
        queryKey: ADMIN_MCP_QUERY_KEYS.sessions(),
      });
    },
  });
}
