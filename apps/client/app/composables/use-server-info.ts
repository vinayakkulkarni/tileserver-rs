import type { PingResponse } from '~/types';
import { useQuery } from '@tanstack/vue-query';
import { pingQueryOptions } from '~/utils/api/server/queries';

export function useServerInfo() {
  const { data, error } = useFetch<PingResponse>(apiUrl('/ping'));

  const versionLabel = computed(() => {
    if (!data.value) return '';
    return `v${data.value.version}`;
  });

  return {
    ping: data,
    pingError: error,
    versionLabel,
  };
}

export function usePingStats() {
  const pingQuery = useQuery(pingQueryOptions());

  const versionLabel = computed(() => {
    if (!pingQuery.data.value) return '';
    return `v${pingQuery.data.value.version}`;
  });

  const sourceCount = computed(() => pingQuery.data.value?.loaded_sources ?? 0);
  const styleCount = computed(() => pingQuery.data.value?.loaded_styles ?? 0);
  const rendererEnabled = computed(
    () => pingQuery.data.value?.renderer_enabled ?? false,
  );
  const cacheMB = computed(() => {
    const bytes = pingQuery.data.value?.cache_bytes ?? 0;
    if (bytes === 0) return '0';
    return `${(bytes / 1024 / 1024).toFixed(0)}`;
  });
  const cacheEnabled = computed(
    () => pingQuery.data.value?.cache_enabled ?? false,
  );
  const uptime = computed(() => {
    const unix = pingQuery.data.value?.loaded_at_unix;
    if (!unix) return 'n/a';
    const seconds = Math.floor(Date.now() / 1000 - unix);
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
    const days = Math.floor(seconds / 86400);
    return `${days}d`;
  });

  return {
    pingQuery,
    ping: pingQuery.data,
    pingError: pingQuery.error,
    isLoading: pingQuery.isLoading,
    versionLabel,
    sourceCount,
    styleCount,
    rendererEnabled,
    cacheMB,
    cacheEnabled,
    uptime,
  };
}
