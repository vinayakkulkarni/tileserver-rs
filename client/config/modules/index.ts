import type { NuxtConfig } from '@nuxt/schema';
import { windicss } from './windicss';

export const modules: NuxtConfig['modules'] = [
  // https://vueuse.org/guide/#nuxt
  '@vueuse/nuxt',
  // https://windicss.org/integrations/nuxt.html
  ['nuxt-windicss', windicss],
];
