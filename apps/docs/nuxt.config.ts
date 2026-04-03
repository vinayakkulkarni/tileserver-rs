export default defineNuxtConfig({
  modules: [
    '@nuxt/content',
    '@nuxt/fonts',
    '@nuxt/eslint',
    '@nuxtjs/color-mode',
    '@vueuse/nuxt',
    'motion-v/nuxt',
    'shadcn-nuxt',
    [
      '@nuxtjs/plausible',
      {
        domain: 'docs.tileserver.app',
        apiHost: 'https://analytics.geoql.in',
        autoOutboundTracking: true,
      },
    ],
  ],

  devtools: { enabled: false },

  app: {
    head: {
      htmlAttrs: { lang: 'en' },
      title: 'Tileserver RS Docs - High-Performance Vector Tile Server',
      meta: [
        { charset: 'utf-8' },
        { name: 'viewport', content: 'width=device-width, initial-scale=1' },
        {
          name: 'description',
          content:
            'Documentation for tileserver-rs — a high-performance vector tile server built in Rust.',
        },
        {
          name: 'theme-color',
          content: '#111119',
          media: '(prefers-color-scheme: dark)',
        },
        {
          name: 'theme-color',
          content: '#ffffff',
          media: '(prefers-color-scheme: light)',
        },
      ],
      link: [
        { rel: 'icon', type: 'image/svg+xml', href: '/favicon.svg' },
        { rel: 'icon', type: 'image/x-icon', href: '/favicon.ico' },
      ],
    },
  },

  css: ['~/assets/css/main.css'],

  colorMode: {
    classSuffix: '',
    preference: 'dark',
    fallback: 'dark',
  },

  content: {
    build: {
      markdown: {
        highlight: {
          theme: {
            light: 'material-theme-lighter',
            default: 'material-theme',
            dark: 'material-theme-palenight',
          },
          langs: [
            'bash',
            'json',
            'js',
            'ts',
            'html',
            'css',
            'vue',
            'shell',
            'md',
            'yaml',
            'toml',
            'rust',
            'docker',
          ],
        },
      },
    },
    database: {
      type: 'd1',
      bindingName: 'DB',
    },
  },

  runtimeConfig: {
    public: {
      baseUrl: process.env.NUXT_PUBLIC_BASE_URL || 'https://docs.tileserver.app',
    },
  },

  future: {
    compatibilityVersion: 4,
  },

  compatibilityDate: '2025-07-18',

  nitro: {
    preset: 'cloudflare-pages',
    cloudflare: {
      nodeCompat: true,
    },
    rollupConfig: {
      output: {
        generatedCode: {
          constBindings: true,
        },
      },
    },
    replace: {
      'process.stdout': 'undefined',
    },
  },

  vite: {
    optimizeDeps: {
      include: [
        '@plausible-analytics/tracker',
        'lucide-vue-next',
        'ogl',
        'clsx',
        'tailwind-merge',
      ],
    },
    ssr: {
      external: ['ogl'],
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

  fonts: {
    providers: {
      google: false,
      fontshare: false,
      bunny: false,
      fontsource: false,
      adobe: false,
    },
    families: [
      { name: 'General Sans', src: '/fonts/general-sans-200.woff2', weight: 200 },
      { name: 'General Sans', src: '/fonts/general-sans-300.woff2', weight: 300 },
      { name: 'General Sans', src: '/fonts/general-sans-400.woff2', weight: 400 },
      { name: 'General Sans', src: '/fonts/general-sans-500.woff2', weight: 500 },
      { name: 'General Sans', src: '/fonts/general-sans-600.woff2', weight: 600 },
      { name: 'General Sans', src: '/fonts/general-sans-700.woff2', weight: 700 },
      { name: 'Switzer', src: '/fonts/switzer-300.woff2', weight: 300 },
      { name: 'Switzer', src: '/fonts/switzer-400.woff2', weight: 400 },
      { name: 'Switzer', src: '/fonts/switzer-500.woff2', weight: 500 },
      { name: 'Switzer', src: '/fonts/switzer-600.woff2', weight: 600 },
      { name: 'Switzer', src: '/fonts/switzer-700.woff2', weight: 700 },
      { name: 'JetBrains Mono', src: '/fonts/jetbrains-mono-latin.woff2', weight: [100, 800] },
      { name: 'JetBrains Mono', src: '/fonts/jetbrains-mono-latin-italic.woff2', weight: [100, 800], style: 'italic' },
      { name: 'Source Serif 4', src: '/fonts/source-serif-4-latin.woff2', weight: [200, 900] },
      { name: 'Source Serif 4', src: '/fonts/source-serif-4-latin-italic.woff2', weight: [200, 900], style: 'italic' },
    ],
  },

  shadcn: {
    prefix: '',
    componentDir: './app/components/ui',
  },
});
