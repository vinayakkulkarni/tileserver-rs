export default defineNuxtConfig({
  extends: ['docus'],

  modules: [
    [
      '@nuxtjs/plausible',
      {
        domain: 'docs.tileserver.app',
        apiHost: 'https://analytics.geoql.in',
        autoOutboundTracking: true,
      },
    ],
  ],

  site: {
    name: 'Tileserver RS',
    description: 'High-performance vector tile server built in Rust',
    url: 'https://tileserver.app',
  },

  content: {
    database: {
      type: 'd1',
      bindingName: 'DB',
    },
  },

  routeRules: {
    '/**': { prerender: false },
  },

  compatibilityDate: '2025-07-18',

  nitro: {
    preset: 'cloudflare-pages',
    cloudflare: {
      nodeCompat: true,
    },
    prerender: {
      crawlLinks: false,
      routes: [],
      ignore: ['/**'],
    },
    experimental: {
      legacyExternals: true,
    },
    unenv: {
      external: ['cloudflare:workers'],
    },
    rollupConfig: {
      output: {
        generatedCode: {
          constBindings: true,
        },
      },
      external: [/^cloudflare:/],
    },
    replace: {
      'process.stdout': 'undefined',
    },
  },

  typescript: {
    strict: true,
  },

  hooks: {
    'prerender:routes': ({ routes }) => {
      routes.clear();
    },
  },

  llms: {
    domain: 'tileserver.app',
  },
});
