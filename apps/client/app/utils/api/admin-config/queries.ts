import { queryOptions } from '@tanstack/vue-query';
import type { AdminConfigPayload } from '~/types/admin-config';
import type { ConfigSchemaPayload } from '~/types/admin-config-schema';
import { ADMIN_CONFIG_QUERY_KEYS } from '~/utils/query-keys/admin-config';

export function adminConfigQueryOptions() {
  return queryOptions({
    queryKey: ADMIN_CONFIG_QUERY_KEYS.view(),
    queryFn: () => $fetch<AdminConfigPayload>('/__admin/config'),
    staleTime: 0,
    refetchOnMount: 'always',
  });
}

export function adminConfigSchemaQueryOptions() {
  return queryOptions({
    queryKey: ADMIN_CONFIG_QUERY_KEYS.schema(),
    queryFn: () => $fetch<ConfigSchemaPayload>('/__admin/config/schema'),
    staleTime: 1000 * 60 * 60,
  });
}
