/**
 * HTTP fetch / API error narrowing types.
 *
 * Nuxt's `$fetch` (built on ofetch) throws a `FetchError` with a
 * specific runtime shape. We do NOT import ofetch's class directly
 * because:
 *
 *   1. We rely on a small, stable subset of fields (`statusCode`,
 *      `status`, `statusMessage`, `response.status`, `response.statusText`,
 *      `data`, `request`) — pulling the full ofetch type forces every
 *      consumer to traverse the whole `FetchResponse<T>` chain.
 *   2. The structural shape is what we narrow with at runtime; a
 *      duck-typed interface documents exactly which fields we read
 *      and survives upstream ofetch type-surface changes.
 *
 * This interface intentionally extends `Error` so consumers can pass
 * the same value they would pass to `Error` constructors.
 */
export interface OfetchLikeError extends Error {
  statusCode?: number;
  status?: number;
  statusMessage?: string;
  response?: { status?: number; statusText?: string };
  data?: unknown;
  request?: string;
}
