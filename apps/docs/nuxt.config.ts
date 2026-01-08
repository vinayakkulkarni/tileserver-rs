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
    url: 'https://tileserver-rs.vinayakkulkarni.dev',
  },

  compatibilityDate: '2025-07-18',

  typescript: {
    strict: true,
  },
});
