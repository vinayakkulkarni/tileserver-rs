/**
 * Query Client Singleton
 *
 * Provides a way to share the QueryClient instance between
 * Vue Query plugin and TanStack DB collections.
 */

import type { QueryClient } from '@tanstack/vue-query';

let queryClient: QueryClient | null = null;

/**
 * Set the QueryClient instance for use by collections.
 * Called by the vue-query plugin during initialization.
 */
export function setQueryClient(client: QueryClient): void {
  queryClient = client;
}

/**
 * Get the QueryClient instance.
 * Returns null if not yet initialized.
 */
export function getQueryClient(): QueryClient | null {
  return queryClient;
}
