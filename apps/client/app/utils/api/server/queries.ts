import type { PingResponse } from '~/types';
import { SERVER_QUERY_KEYS } from '~/utils/query-keys';

export async function fetchPing(): Promise<PingResponse> {
  return await $fetch<PingResponse>('/ping');
}

export function pingQueryOptions() {
  return {
    queryKey: SERVER_QUERY_KEYS.ping(),
    queryFn: fetchPing,
    staleTime: 15 * 1000,
    refetchInterval: 30 * 1000,
  };
}
