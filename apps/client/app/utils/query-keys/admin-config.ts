export const ADMIN_CONFIG_QUERY_KEYS = {
  all: () => ['admin', 'config'] as const,
  view: () => [...ADMIN_CONFIG_QUERY_KEYS.all(), 'view'] as const,
};
