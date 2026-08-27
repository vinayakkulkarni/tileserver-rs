/**
 * Build a server URL that respects the runtime base path.
 *
 * When tileserver-rs is deployed under a URL subfolder (e.g. `/maps`), the
 * server rewrites the embedded SPA's runtime `app.baseURL` at startup, so
 * `useRuntimeConfig().app.baseURL` reflects the subfolder at runtime. All
 * direct server requests — `$fetch`, native `fetch`, MapLibre source URLs, and
 * `:href`/`:src` asset links — must be prefixed with it so the browser requests
 * subfolder-prefixed URLs. `<NuxtLink :to>` and chunk/asset loading are already
 * base-aware via Nuxt's router and build config, so they must NOT be wrapped.
 *
 * For a root deployment `app.baseURL` is `/`, so this returns the path
 * unchanged.
 *
 * @param path - A root-absolute server path, e.g. `/ping` or `/data/x.json`.
 * @returns The path prefixed with the runtime base, e.g. `/maps/ping`.
 */
export function apiUrl(path: string): string {
  const { app } = useRuntimeConfig();
  // `app.baseURL` is `/` (root) or `/maps/` (subfolder). Drop the trailing
  // slash so joining with a leading-slash path yields `/ping` or `/maps/ping`
  // rather than a doubled slash.
  const base = app.baseURL.replace(/\/$/, '');
  return `${base}${path}`;
}
