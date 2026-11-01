import { queryOptions } from '@tanstack/vue-query';
import type { AdminStatsPayload } from '~/types/admin-stats';
import { ADMIN_STATS_QUERY_KEYS } from '~/utils/query-keys/admin-stats';

/** Refresh interval of the analytics dashboard. */
export const ADMIN_STATS_REFRESH_MS = 5000;

export function adminStatsQueryOptions() {
  return queryOptions({
    queryKey: ADMIN_STATS_QUERY_KEYS.snapshot(),
    queryFn: () => $fetch<AdminStatsPayload>(apiUrl('/__admin/stats')),
    refetchInterval: ADMIN_STATS_REFRESH_MS,
    staleTime: 0,
  });
}
