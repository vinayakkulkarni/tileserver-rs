import { useQuery } from '@tanstack/vue-query';
import type { AdminBreadcrumbCrumb } from '~/types';
import {
  adminMcpClientsQueryOptions,
  adminMcpSessionsQueryOptions,
} from '~/utils/api/admin-mcp';
import { friendlyAdminError } from '~/utils/api/admin-mcp/friendly-error';
import { pingQueryOptions } from '~/utils/api/server';

const BREADCRUMBS: AdminBreadcrumbCrumb[] = [
  { label: 'Home', to: '/' },
  { label: 'Admin' },
];

const RECENT_CLIENT_LIMIT = 5;

function formatUptime(loadedAtUnix: number, nowMs: number): string {
  const deltaSec = Math.max(0, Math.floor(nowMs / 1000 - loadedAtUnix));
  if (deltaSec < 60) return `${deltaSec}s`;
  const minutes = Math.floor(deltaSec / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 48) return `${hours}h ${minutes % 60}m`;
  const days = Math.floor(hours / 24);
  return `${days}d ${hours % 24}h`;
}

export function useAdminDashboard() {
  const pingQuery = useQuery(pingQueryOptions());
  const clientsQuery = useQuery(adminMcpClientsQueryOptions());
  const sessionsQuery = useQuery(adminMcpSessionsQueryOptions());

  const now = useNow({ interval: 60_000 });

  const ping = computed(() => pingQuery.data.value ?? null);
  const clients = computed(() => clientsQuery.data.value ?? []);
  const sessions = computed(() => sessionsQuery.data.value ?? []);

  const isLoading = computed(() => pingQuery.isPending.value);
  const pingError = computed(() => pingQuery.error.value);
  const friendlyPingError = computed(() => friendlyAdminError(pingError.value));

  const clientsAreLoading = computed(() => clientsQuery.isPending.value);
  const RECENT_SKELETON_ROWS = 4;

  const uptimeLabel = computed(() => {
    if (!ping.value) return '—';
    return formatUptime(ping.value.loaded_at_unix, now.value.getTime());
  });

  const loadedSources = computed(() => ping.value?.loaded_sources ?? 0);
  const loadedStyles = computed(() => ping.value?.loaded_styles ?? 0);
  const rendererEnabled = computed(() => ping.value?.renderer_enabled ?? false);
  const versionLabel = computed(() => ping.value?.version ?? '—');
  const configHashShort = computed(() => {
    const h = ping.value?.config_hash;
    return h ? h.slice(0, 12) : '—';
  });

  const clientsCount = computed(() => clients.value.length);
  const sessionsCount = computed(() => sessions.value.length);

  const recentClients = computed(() =>
    [...clients.value]
      .sort((a, b) => (b.last_seen_at ?? 0) - (a.last_seen_at ?? 0))
      .slice(0, RECENT_CLIENT_LIMIT),
  );

  const clientsAreEmpty = computed(
    () => !clientsQuery.isPending.value && clients.value.length === 0,
  );

  function formatLastSeen(unixSecs: number | null): string {
    if (unixSecs === null) return 'never';
    const delta = Math.floor(now.value.getTime() / 1000 - unixSecs);
    if (delta < 60) return `${delta}s ago`;
    if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
    if (delta < 86400) return `${Math.floor(delta / 3600)}h ago`;
    return `${Math.floor(delta / 86400)}d ago`;
  }

  return {
    breadcrumbs: BREADCRUMBS,
    isLoading,
    pingError,
    friendlyPingError,
    uptimeLabel,
    loadedSources,
    loadedStyles,
    rendererEnabled,
    versionLabel,
    configHashShort,
    clientsCount,
    sessionsCount,
    recentClients,
    clientsAreEmpty,
    clientsAreLoading,
    RECENT_SKELETON_ROWS,
    formatLastSeen,
  };
}
