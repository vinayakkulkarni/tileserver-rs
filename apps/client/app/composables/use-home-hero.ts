import { usePingStats } from './use-server-info';

export function useHomeHero() {
  const { pingQuery } = usePingStats();

  const statusOk = computed(() => pingQuery.data.value?.status === 'ok');
  const isLoading = computed(() => pingQuery.isLoading.value);

  const versionLabel = computed(() => {
    if (!pingQuery.data.value) return '';
    return `v${pingQuery.data.value.version}`;
  });

  const rendererEnabled = computed(
    () => pingQuery.data.value?.renderer_enabled ?? false,
  );

  const cacheEnabled = computed(
    () => pingQuery.data.value?.cache_enabled ?? false,
  );

  const cacheMb = computed(() => {
    if (!pingQuery.data.value) return '—';
    return `${(pingQuery.data.value.cache_bytes / 1024 / 1024).toFixed(0)}`;
  });

  function formatUptime(unix: number): string {
    const now = Date.now() / 1000;
    const diff = Math.max(0, now - unix);
    if (diff < 60) return `${Math.floor(diff)}s`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h`;
    return `${Math.floor(diff / 86400)}d`;
  }

  const uptime = computed(() => {
    const unix = pingQuery.data.value?.loaded_at_unix;
    if (!unix) return '—';
    return formatUptime(unix);
  });

  const sourceCount = computed(
    () => pingQuery.data.value?.loaded_sources ?? '—',
  );
  const styleCount = computed(() => pingQuery.data.value?.loaded_styles ?? '—');

  return {
    statusOk,
    isLoading,
    versionLabel,
    rendererEnabled,
    cacheEnabled,
    cacheMb,
    uptime,
    sourceCount,
    styleCount,
  };
}
