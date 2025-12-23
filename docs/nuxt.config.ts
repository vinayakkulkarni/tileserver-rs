export default defineNuxtConfig({
  extends: ['docus'],

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
