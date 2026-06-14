import tailwindcss from '@tailwindcss/vite';

const BACKEND_PORT = process.env.TILESERVER_BACKEND_PORT ?? '8080';
const ADMIN_PORT = process.env.TILESERVER_ADMIN_PORT ?? '8081';
const BACKEND = `http://localhost:${BACKEND_PORT}`;
const ADMIN = `http://localhost:${ADMIN_PORT}`;

export default defineNuxtConfig({
  modules: [
    'shadcn-nuxt',
    '@vueuse/nuxt',
    '@nuxt/eslint',
    '@nuxt/fonts',
    '@nuxtjs/color-mode',
    'motion-v/nuxt',
    '@comark/nuxt',
  ],

  fonts: {
    providers: {
      google: false,
      fontshare: false,
      bunny: false,
      fontsource: false,
      adobe: false,
    },
    families: [
      {
        name: 'General Sans',
        src: '/fonts/general-sans-200.woff2',
        weight: 200,
      },
      {
        name: 'General Sans',
        src: '/fonts/general-sans-300.woff2',
        weight: 300,
      },
      {
        name: 'General Sans',
        src: '/fonts/general-sans-400.woff2',
        weight: 400,
      },
      {
        name: 'General Sans',
        src: '/fonts/general-sans-500.woff2',
        weight: 500,
      },
      {
        name: 'General Sans',
        src: '/fonts/general-sans-600.woff2',
        weight: 600,
      },
      {
        name: 'General Sans',
        src: '/fonts/general-sans-700.woff2',
        weight: 700,
      },
      { name: 'Switzer', src: '/fonts/switzer-300.woff2', weight: 300 },
      { name: 'Switzer', src: '/fonts/switzer-400.woff2', weight: 400 },
      { name: 'Switzer', src: '/fonts/switzer-500.woff2', weight: 500 },
      { name: 'Switzer', src: '/fonts/switzer-600.woff2', weight: 600 },
      { name: 'Switzer', src: '/fonts/switzer-700.woff2', weight: 700 },
      {
        name: 'JetBrains Mono',
        src: '/fonts/jetbrains-mono-latin.woff2',
        weight: [100, 800],
      },
      {
        name: 'JetBrains Mono',
        src: '/fonts/jetbrains-mono-latin-italic.woff2',
        weight: [100, 800],
        style: 'italic',
      },
      {
        name: 'Source Serif 4',
        src: '/fonts/source-serif-4-latin.woff2',
        weight: [200, 900],
      },
      {
        name: 'Source Serif 4',
        src: '/fonts/source-serif-4-latin-italic.woff2',
        weight: [200, 900],
        style: 'italic',
      },
    ],
  },

  // SPA mode - embedded in Rust binary
  ssr: false,

  devtools: { enabled: false },

  app: {
    head: {
      htmlAttrs: { lang: 'en' },
      title: 'Tileserver RS - Vector Maps',
      meta: [
        { charset: 'utf-8' },
        { name: 'viewport', content: 'width=device-width, initial-scale=1' },
        {
          name: 'description',
          content:
            'High-performance vector tile server built in Rust. Serve PMTiles and MBTiles with MapLibre GL JS visualization.',
        },
        {
          name: 'keywords',
          content:
            'tileserver, vector tiles, pmtiles, mbtiles, maplibre, rust, gis, mapping',
        },
        { name: 'theme-color', content: '#3b82f6' },
      ],
      link: [{ rel: 'icon', type: 'image/x-icon', href: '/favicon.ico' }],
    },
  },

  css: [
    '~/assets/css/tailwind.css',
    'maplibre-gl/dist/maplibre-gl.css',
    '@geoql/v-maplibre/style.css',
    'maplibre-gl-inspect/dist/style.css',
  ],

  colorMode: {
    classSuffix: '',
    preference: 'system',
    fallback: 'light',
  },

  future: {
    compatibilityVersion: 4,
  },

  experimental: {
    typedPages: true,
    viewTransition: true,
    payloadExtraction: true,
    // Workaround for Nuxt 4.4.5 + Vite 8 SPA-mode bug: `rollupOptions.input.entry`
    // is dropped during config merge with `ssr: false`, crashing dev with
    // "No entry found in rollupOptions.input". Upstream fix landed in
    // https://github.com/nuxt/nuxt/pull/35037 but not yet released; remove
    // this flag once Nuxt > 4.4.5 ships.
    viteEnvironmentApi: true,
  },

  compatibilityDate: '2024-12-23',

  nitro: {
    preset: 'static',
    prerender: {
      crawlLinks: true,
      routes: ['/'],
    },
  },

  vite: {
    plugins: [tailwindcss()],
    optimizeDeps: {
      include: ['maplibre-gl', '@geoql/v-maplibre', '@mlc-ai/web-llm'],
    },
    worker: {
      format: 'es',
    },
    ssr: {
      external: [
        'maplibre-gl',
        '@geoql/v-maplibre',
        '@tanstack/vue-db',
        '@tanstack/db',
      ],
    },
    server: {
      // Dev-only proxy. Production builds embed the SPA in the Rust binary
      // (nitro.preset: 'static'), so requests are same-origin and ports
      // are whatever the operator sets in config.toml.
      //
      // Override at dev time (these are build-time env vars consumed by
      // nuxt.config.ts itself, so they use the unprefixed `TILESERVER_*`
      // convention rather than `NUXT_*` — the latter is reserved by Nuxt
      // for `runtimeConfig` auto-mapping, which we don't use here):
      //   TILESERVER_BACKEND_PORT=9000 TILESERVER_ADMIN_PORT=9001 pnpm dev
      //
      // Defaults match data/configs/dev.toml + data/configs/mcp.toml.
      proxy: {
        '/health': BACKEND,
        '/ping': BACKEND,
        '/data.json': BACKEND,
        '/styles.json': BACKEND,
        '/fonts.json': BACKEND,
        '/openapi.json': BACKEND,
        '^/_openapi': BACKEND,
        '^/data/[^/]+\\.json$': BACKEND,
        '^/data/[^/]+/\\d+/\\d+/\\d+': BACKEND,
        '^/styles/[^/]+\\.json$': BACKEND,
        '^/styles/[^/]+/style\\.json$': BACKEND,
        '^/styles/[^/]+/wmts\\.xml': BACKEND,
        '^/styles/[^/]+/sprite(@\\dx)?\\.(png|json)$': BACKEND,
        '^/styles/[^/]+/static/': BACKEND,
        '^/styles/[^/]+/\\d+/\\d+/\\d+': BACKEND,
        '^/fonts/': BACKEND,
        '^/api/spatial/': BACKEND,
        '^/api/upload': BACKEND,
        // Admin endpoints — backend mounts these on a SEPARATE bind
        // (config.server.admin_bind, default 127.0.0.1:8081).
        '^/__admin/': ADMIN,
      },
    },
  },

  typescript: {
    strict: true,
    typeCheck: false,
  },

  shadcn: {
    prefix: '',
    componentDir: '@/components/ui',
  },
});
