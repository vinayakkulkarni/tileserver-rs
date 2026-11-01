export const ADMIN_STATS_QUERY_KEYS = {
  all: () => ['admin', 'stats'] as const,
  snapshot: () => [...ADMIN_STATS_QUERY_KEYS.all(), 'snapshot'] as const,
};
