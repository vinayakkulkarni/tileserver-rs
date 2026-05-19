/**
 * Admin MCP OAuth Queries
 *
 * Query options + fetch functions for `/__admin/oauth/clients` and
 * `/__admin/oauth/sessions`. Mirrors the existing `data/queries.ts`
 * pattern: pure fetch functions + Options factories consumed by
 * `useQuery` in pages.
 *
 * The admin endpoints live on the admin listener (`server.admin_bind`),
 * NOT the public listener. Operators reach them either by binding the
 * admin listener to the same port as the public one, or by reverse-
 * proxying `/__admin/*` to the admin port.
 */

import type { AdminMcpClient, AdminMcpSession } from '~/types';
import { ADMIN_MCP_QUERY_KEYS } from '~/utils/query-keys';

export async function fetchAdminMcpClients(): Promise<AdminMcpClient[]> {
  const result = await $fetch<AdminMcpClient[]>('/__admin/oauth/clients');
  return result ?? [];
}

export async function fetchAdminMcpSessions(): Promise<AdminMcpSession[]> {
  const result = await $fetch<AdminMcpSession[]>('/__admin/oauth/sessions');
  return result ?? [];
}

export function adminMcpClientsQueryOptions() {
  return {
    queryKey: ADMIN_MCP_QUERY_KEYS.clients(),
    queryFn: fetchAdminMcpClients,
    staleTime: 10 * 1000,
  };
}

export function adminMcpSessionsQueryOptions() {
  return {
    queryKey: ADMIN_MCP_QUERY_KEYS.sessions(),
    queryFn: fetchAdminMcpSessions,
    staleTime: 10 * 1000,
  };
}
