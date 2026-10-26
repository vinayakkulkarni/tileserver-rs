// Post-build check for the Cloudflare Pages output in dist/.
// Fails the build when a page in sitemap.xml was not prerendered or its
// canonical tag does not match its own URL (no trailing slash).
// Run: node config/verify-dist.ts (Node strips the types).
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const SITE_URL = 'https://docs.tileserver.app';
const dist = fileURLToPath(new URL('../dist/', import.meta.url));
const errors: string[] = [];

const sitemapPath = `${dist}sitemap.xml`;
const sitemap = existsSync(sitemapPath)
  ? readFileSync(sitemapPath, 'utf8')
  : '';
const locs = [...sitemap.matchAll(/<loc>([^<]+)<\/loc>/g)].map(
  (match) => match[1] ?? '',
);
if (locs.length === 0) errors.push('dist/sitemap.xml is missing or empty');

for (const loc of locs) {
  const route = loc.slice(SITE_URL.length);
  if (route !== '/' && route.endsWith('/')) {
    errors.push(`${loc}: sitemap URL has a trailing slash`);
  }
  const file = route === '/' ? 'index.html' : `${route.slice(1)}.html`;
  if (!existsSync(`${dist}${file}`)) {
    errors.push(`${loc}: not prerendered (dist/${file} missing)`);
    continue;
  }
  const html = readFileSync(`${dist}${file}`, 'utf8');
  const canonical = /<link rel="canonical" href="([^"]+)"/.exec(html)?.[1];
  if (canonical !== loc) {
    errors.push(`${loc}: canonical is ${canonical ?? 'missing'}`);
  }
}

// Without 404.html, Cloudflare Pages serves index.html with status 200 for
// unknown static paths (soft 404).
if (!existsSync(`${dist}404.html`)) {
  errors.push('dist/404.html missing (unknown assets would be soft 404s)');
}

const robotsPath = `${dist}robots.txt`;
const robots = existsSync(robotsPath) ? readFileSync(robotsPath, 'utf8') : '';
if (!robots.includes(`Sitemap: ${SITE_URL}/sitemap.xml`)) {
  errors.push('dist/robots.txt does not reference the sitemap');
}

if (errors.length > 0) {
  console.error(`verify-dist: ${errors.length} problem(s)`);
  for (const error of errors) console.error(`  - ${error}`);
  process.exit(1);
}
console.log(`verify-dist: ${locs.length} pages prerendered with canonicals`);
