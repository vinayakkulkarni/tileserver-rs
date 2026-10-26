import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { CONTENT_DIR, contentFiles, contentRoutes } from './content-routes';
import {
  LEGACY_REDIRECTS,
  buildSitemap,
  canonicalUrl,
  contentFileToRoute,
  internalLinks,
  pageRoutes,
  prerenderOptions,
} from './seo';

const routes = pageRoutes(contentRoutes());

// URLs Google Search Console reported as "Not found (404)" on 2026-10-26.
const GSC_404_URLS = [
  '/guides/auto-detect',
  '/deployment/production',
  '/deployment/kubernetes',
  '/guides/raster-tiles',
];

describe('docs routes', () => {
  it('maps numbered content files to clean routes', () => {
    expect(contentFileToRoute('4.guides/6.auto-detect.md')).toBe(
      '/guides/auto-detect',
    );
    expect(contentRoutes()).toContain('/guides/auto-detect');
  });

  it('prerenders every page as x.html so no page needs the runtime D1 import', () => {
    const options = prerenderOptions(routes);
    expect(options.autoSubfolderIndex).toBe(false);
    expect(options.failOnError).toBe(true);
    for (const route of routes) expect(options.routes).toContain(route);
    expect(options.routes).toContain('/sitemap.xml');
  });

  it('serves or redirects every URL GSC reported as 404', () => {
    for (const url of GSC_404_URLS) {
      const live = routes.includes(url);
      const redirected = url in LEGACY_REDIRECTS;
      expect(live || redirected, url).toBe(true);
    }
  });

  it('redirects old URLs straight to a live page', () => {
    for (const [from, to] of Object.entries(LEGACY_REDIRECTS)) {
      expect(routes, `${from} must not also be a page`).not.toContain(from);
      expect(routes, `${from} -> ${to}`).toContain(to);
    }
  });

  it('links only to live pages from the markdown content', () => {
    const broken: string[] = [];
    for (const file of contentFiles()) {
      const markdown = readFileSync(join(CONTENT_DIR, file), 'utf8');
      for (const link of internalLinks(markdown)) {
        if (!routes.includes(link)) broken.push(`${file}: ${link}`);
      }
    }
    expect(broken).toEqual([]);
  });
});

describe('nuxt.config.ts', async () => {
  // nuxt.config.ts calls the Nuxt global; stub it so the file can be read.
  Object.assign(globalThis, {
    defineNuxtConfig: <T>(config: T): T => config,
  });
  const { default: config } = await import('../nuxt.config');

  it('prerenders every page and the sitemap', () => {
    const prerender = config.nitro?.prerender;
    expect(prerender?.autoSubfolderIndex).toBe(false);
    for (const route of [...routes, '/sitemap.xml']) {
      expect(prerender?.routes).toContain(route);
    }
  });

  it('301-redirects every legacy URL', () => {
    for (const [from, to] of Object.entries(LEGACY_REDIRECTS)) {
      expect(config.routeRules?.[from]).toEqual({
        redirect: { to, statusCode: 301 },
      });
      expect(config.nitro?.prerender?.ignore).toContain(from);
    }
  });
});

describe('canonical URLs', () => {
  it('uses the no-trailing-slash style', () => {
    expect(canonicalUrl('/')).toBe('https://docs.tileserver.app/');
    expect(canonicalUrl('/guides/mlt/')).toBe(
      'https://docs.tileserver.app/guides/mlt',
    );
    expect(canonicalUrl('/guides/mlt')).toBe(
      'https://docs.tileserver.app/guides/mlt',
    );
  });
});

describe('sitemap.xml', () => {
  const xml = buildSitemap(routes);

  it('lists every page once in the canonical style', () => {
    for (const route of routes) {
      expect(xml).toContain(`<loc>${canonicalUrl(route)}</loc>`);
    }
    expect(xml.match(/<loc>/g)).toHaveLength(routes.length);
    expect(xml).not.toMatch(
      /<loc>https:\/\/docs\.tileserver\.app\/[^<]+\/<\/loc>/,
    );
    expect(xml).not.toMatch(/<loc>[^<]*\/(\d+\.)[^<]*<\/loc>/);
  });

  it('never lists a redirecting URL', () => {
    for (const from of Object.keys(LEGACY_REDIRECTS)) {
      expect(xml).not.toContain(`<loc>${canonicalUrl(from)}</loc>`);
    }
  });
});

describe('robots.txt', () => {
  const robotsPath = fileURLToPath(
    new URL('../public/robots.txt', import.meta.url),
  );
  const readRobots = () =>
    existsSync(robotsPath) ? readFileSync(robotsPath, 'utf8') : '';

  it('points crawlers at the sitemap', () => {
    expect(readRobots()).toContain(
      'Sitemap: https://docs.tileserver.app/sitemap.xml',
    );
  });

  it('keeps build assets and endpoints out of the index', () => {
    const robots = readRobots();
    expect(robots).toMatch(/^Disallow: \/_nuxt\/$/m);
    expect(robots).toMatch(/^Disallow: \/__nuxt_content\/$/m);
  });
});
