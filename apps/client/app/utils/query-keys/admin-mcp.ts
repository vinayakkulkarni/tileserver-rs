/**
 * Admin MCP OAuth Query Keys
 * Used by TanStack Query for the admin connected-apps + devices pages.
 */
export const ADMIN_MCP_QUERY_KEYS = {
  all: ['admin', 'mcp'] as const,
  clients: () => [...ADMIN_MCP_QUERY_KEYS.all, 'clients'] as const,
  sessions: () => [...ADMIN_MCP_QUERY_KEYS.all, 'sessions'] as const,
} as const;
