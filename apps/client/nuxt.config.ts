export default defineNuxtConfig({
  modules: [
    'shadcn-nuxt',
    '@vueuse/nuxt',
    '@nuxt/eslint',
    '@nuxtjs/color-mode',
  ],

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
    '@geoql/v-maplibre/dist/v-maplibre.css',
    'maplibre-gl-inspect/dist/maplibre-gl-inspect.css',
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
  },

  compatibilityDate: '2024-12-23',

  nitro: {
    preset: 'static',
    prerender: {
      crawlLinks: true,
      routes: ['/'],
    },
    devProxy: {
      '/styles.json': { target: 'http://localhost:8080/styles.json' },
      '/styles': { target: 'http://localhost:8080/styles' },
      '/data.json': { target: 'http://localhost:8080/data.json' },
      '/data': { target: 'http://localhost:8080/data' },
      '/fonts.json': { target: 'http://localhost:8080/fonts.json' },
      '/fonts': { target: 'http://localhost:8080/fonts' },
      '/health': { target: 'http://localhost:8080/health' },
    },
  },

  vite: {
    optimizeDeps: {
      include: ['maplibre-gl', '@geoql/v-maplibre'],
    },
    ssr: {
      external: ['maplibre-gl', '@geoql/v-maplibre'],
    },
  },

  typescript: {
    strict: true,
    typeCheck: false,
  },

  postcss: {
    plugins: {
      '@tailwindcss/postcss': {},
    },
  },

  shadcn: {
    prefix: '',
    componentDir: '@/components/ui',
  },
});
