// Search-engine facts for docs.tileserver.app, shared by nuxt.config.ts,
// the sitemap route and the tests. Pure: no file system access here.

export const SITE_URL = 'https://docs.tileserver.app';

// Pages that are not in content/ (Vue pages under app/pages).
export const STATIC_PAGE_ROUTES = ['/', '/deploy/cost'] as const;

// Old URLs that Google still crawls, mapped to the page that now holds the
// content. Each target must be a live page, never another redirect.
export const LEGACY_REDIRECTS: Readonly<Record<string, string>> = {
  // Deduplicated into /getting-started/config in v2.30.1.
  '/getting-started/configuration': '/getting-started/config',
  // Raster (COG/VRT) sources are documented in the config reference.
  '/guides/raster-tiles': '/getting-started/config',
  // Linked from the Docker guide; deployment docs live in the deploy guide.
  '/deployment/production': '/guides/deploy',
  '/deployment/kubernetes': '/guides/deploy',
};

/** Maps a content file path (e.g. `4.guides/6.auto-detect.md`) to its route. */
export function contentFileToRoute(file: string): string {
  const segments = file
    .replace(/\\/g, '/')
    .replace(/\.md$/, '')
    .split('/')
    .map((segment) => segment.replace(/^\d+\./, ''));
  return `/${segments.join('/')}`;
}

/** Every page the site serves, sorted, in the no-trailing-slash style. */
export function pageRoutes(contentRoutes: readonly string[]): string[] {
  return [...new Set([...STATIC_PAGE_ROUTES, ...contentRoutes])].sort();
}

/**
 * Nitro prerender options. Every page is rendered to static HTML at build
 * time, so a page never depends on the runtime D1 content import (which
 * stopped part way in production and left 22 of 33 guides as 404s).
 * `autoSubfolderIndex: false` writes `x.html`, so Cloudflare Pages serves
 * `/x` and 308-redirects `/x/` to `/x`.
 */
export function prerenderOptions(routes: readonly string[]) {
  return {
    routes: [...routes, '/sitemap.xml'],
    // Legacy URLs are real 301s (_redirects), not meta-refresh HTML pages.
    ignore: Object.keys(LEGACY_REDIRECTS),
    crawlLinks: true,
    autoSubfolderIndex: false,
    failOnError: true,
  };
}

/** Absolute canonical URL for a route, in the no-trailing-slash style. */
export function canonicalUrl(path: string): string {
  const clean = path.length > 1 ? path.replace(/\/+$/, '') : '/';
  return `${SITE_URL}${clean}`;
}

function escapeXml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

/** Renders a sitemap.xml for the given routes. */
export function buildSitemap(routes: readonly string[]): string {
  const urls = routes
    .map((route) => `  <url><loc>${escapeXml(canonicalUrl(route))}</loc></url>`)
    .join('\n');
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    urls,
    '</urlset>',
    '',
  ].join('\n');
}

/** Internal links (`](/path)`) in a markdown document, without fragments. */
export function internalLinks(markdown: string): string[] {
  const links: string[] = [];
  for (const match of markdown.matchAll(/\]\((\/[^)\s#]*)/g)) {
    const link = match[1];
    if (link) links.push(link);
  }
  return links;
}
