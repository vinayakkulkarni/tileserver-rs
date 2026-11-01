import { useQuery } from '@tanstack/vue-query';
import type { AdminBreadcrumbCrumb, StatsFigure } from '~/types';
import {
  formatBytes,
  formatCount,
  formatMs,
  formatPercent,
  latencyBars,
  minuteBars,
  zoomBars,
} from '~/utils/analytics-format';
import { adminStatsQueryOptions } from '~/utils/api/admin-stats';
import { friendlyAdminError } from '~/utils/api/admin-mcp/friendly-error';

const BREADCRUMBS: AdminBreadcrumbCrumb[] = [
  { label: 'Home', to: '/' },
  { label: 'Admin', to: '/admin' },
  { label: 'Analytics' },
];

export function useAdminAnalytics() {
  const query = useQuery(adminStatsQueryOptions());
  const stats = computed(() => query.data.value ?? null);
  const isLoading = computed(() => query.isPending.value);
  const error = computed(() =>
    query.error.value ? friendlyAdminError(query.error.value) : null,
  );
  const isDisabled = computed(() => stats.value?.enabled === false);

  const figures = computed<StatsFigure[]>(() => {
    const s = stats.value;
    if (!s) return [];
    const cache = s.cache
      ? `${formatBytes(s.cache.size_bytes)} of ${formatBytes(s.cache.capacity_bytes)}`
      : 'no cache';
    return [
      {
        label: 'Requests',
        value: formatCount(s.totals.requests),
        hint: `${formatCount(s.totals.last_hour)} in the last hour`,
      },
      {
        label: 'Cache hit rate',
        value: formatPercent(s.totals.cache_hit_rate),
        hint: cache,
      },
      {
        label: 'p50 / p95',
        value: `${formatMs(s.latency.p50_ms)} / ${formatMs(s.latency.p95_ms)}`,
        hint: `p99 ${formatMs(s.latency.p99_ms)}`,
      },
      {
        label: 'Error rate',
        value: formatPercent(s.totals.error_rate),
        hint: `${formatCount(s.totals.errors)} errors, ${formatCount(s.totals.not_found)} not found`,
      },
    ];
  });

  return {
    breadcrumbs: BREADCRUMBS,
    stats,
    isLoading,
    error,
    isDisabled,
    figures,
    rateBars: computed(() => minuteBars(stats.value?.per_minute ?? [])),
    zoomBarsList: computed(() => zoomBars(stats.value?.zooms ?? [])),
    latencyBarsList: computed(() =>
      latencyBars(stats.value?.latency_buckets ?? []),
    ),
  };
}
