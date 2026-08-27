/**
 * Single-session revoke. The owning client's row in the clients table
 * shows a lower `active_sessions` count after the next refetch, so both
 * caches are invalidated together. `deleted: false` is a normal
 * idempotent success case, not an error — callers must not throw on it.
 */

import { useMutation, useQueryClient } from '@tanstack/vue-query';
import type { AdminMcpDeleteResponse } from '~/types';
import { ADMIN_MCP_QUERY_KEYS } from '~/utils/query-keys';

export function useDeleteAdminMcpSessionMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (token: string): Promise<AdminMcpDeleteResponse> => {
      return $fetch<AdminMcpDeleteResponse>(
        apiUrl(`/__admin/oauth/sessions/${encodeURIComponent(token)}`),
        {
          method: 'DELETE',
        },
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ADMIN_MCP_QUERY_KEYS.sessions(),
      });
      queryClient.invalidateQueries({
        queryKey: ADMIN_MCP_QUERY_KEYS.clients(),
      });
    },
  });
}
