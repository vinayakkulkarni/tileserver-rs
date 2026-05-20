import { queryOptions } from '@tanstack/vue-query';
import type { AdminConfigPayload } from '~/types/admin-config';
import { ADMIN_CONFIG_QUERY_KEYS } from '~/utils/query-keys/admin-config';

export function adminConfigQueryOptions() {
  return queryOptions({
    queryKey: ADMIN_CONFIG_QUERY_KEYS.view(),
    queryFn: () => $fetch<AdminConfigPayload>('/__admin/config'),
    staleTime: 0,
    refetchOnMount: 'always',
  });
}
