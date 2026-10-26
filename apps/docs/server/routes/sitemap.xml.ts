import { buildSitemap } from '../../config/seo';

// Prerendered at build time (see nitro.prerender in nuxt.config.ts), so
// Cloudflare Pages serves it as a static file.
export default defineEventHandler((event) => {
  const { sitemapRoutes } = useRuntimeConfig(event);
  setResponseHeader(event, 'content-type', 'application/xml; charset=utf-8');
  return buildSitemap(sitemapRoutes);
});
