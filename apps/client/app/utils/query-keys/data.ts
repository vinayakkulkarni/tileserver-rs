/**
 * Data Sources Query Keys
 * Used by TanStack Query and TanStack DB collections
 */
export const DATA_SOURCES_QUERY_KEYS = {
  all: ['data-sources'] as const,
  lists: () => [...DATA_SOURCES_QUERY_KEYS.all, 'list'] as const,
  list: (params?: { type?: string }) =>
    [...DATA_SOURCES_QUERY_KEYS.lists(), params ?? {}] as const,
  details: () => [...DATA_SOURCES_QUERY_KEYS.all, 'detail'] as const,
  detail: (sourceId: string) =>
    [...DATA_SOURCES_QUERY_KEYS.details(), sourceId] as const,
} as const
