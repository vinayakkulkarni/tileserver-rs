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

  llms: {
    domain: 'tileserver.app',
  },
});
