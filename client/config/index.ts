import { NuxtConfig } from '@nuxt/schema';
import { head } from './head';

const app: NuxtConfig['app'] = {
  head,
};

const css: NuxtConfig['css'] = [
  'maplibre-gl/dist/maplibre-gl.css',
  'maplibre-gl-inspect/dist/maplibre-gl-inspect.css',
  '~/assets/css/global.css',
  '~/assets/css/typography.css',
];

const components: NuxtConfig['components'] = true;

const plugins: NuxtConfig['plugins'] = [];

const runtimeConfig: NuxtConfig['runtimeConfig'] = {
  public: {
    appVersion: process.env.npm_package_version,
  },
};

const ssr: NuxtConfig['ssr'] = false;

const typescript: NuxtConfig['typescript'] = {
  strict: true,
  shim: false,
};

export { modules } from './modules';
export { app, css, components, plugins, runtimeConfig, ssr, typescript };
