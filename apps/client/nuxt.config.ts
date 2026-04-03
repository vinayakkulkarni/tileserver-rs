export default defineNuxtConfig({
  modules: [
    'shadcn-nuxt',
    '@vueuse/nuxt',
    '@nuxt/eslint',
    '@nuxt/fonts',
    '@nuxtjs/color-mode',
    'motion-v/nuxt',
    'nuxt-workers',
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
    '@geoql/v-maplibre/dist/v-maplibre.css',
    'maplibre-gl-inspect/dist/style.css',
    'markstream-vue/index.css',
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
        'markstream-vue',
        '@tanstack/vue-db',
        '@tanstack/db',
      ],
    },
    server: {
      proxy: {
        // Proxy API requests to Rust backend
        '/health': 'http://localhost:8080',
        '/ping': 'http://localhost:8080',
        '/data.json': 'http://localhost:8080',
        '/styles.json': 'http://localhost:8080',
        '/fonts.json': 'http://localhost:8080',
        // Use regex to match .json and tile requests but not page routes
        '^/data/[^/]+\\.json$': 'http://localhost:8080',
        '^/data/[^/]+/\\d+/\\d+/\\d+': 'http://localhost:8080',
        '^/styles/[^/]+/style\\.json$': 'http://localhost:8080',
        '^/styles/[^/]+/static/': 'http://localhost:8080',
        '^/styles/[^/]+/\\d+/\\d+/\\d+': 'http://localhost:8080',
        '^/fonts/': 'http://localhost:8080',
        // Spatial API for LLM tool integration
        '^/api/spatial/': 'http://localhost:8080',
        // Upload API for drag-and-drop file visualization
        '^/api/upload': 'http://localhost:8080',
      },
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
