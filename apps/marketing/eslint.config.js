// @ts-check
import { createConfigForNuxt } from '@nuxt/eslint-config/flat';
import betterTailwindcss from 'eslint-plugin-better-tailwindcss';
import oxlint from 'eslint-plugin-oxlint';

export default createConfigForNuxt({
  features: {
    stylistic: false,
    tooling: true,
    typescript: true,
  },
})
  .override('nuxt/vue/rules', {
    rules: {
      'vue/html-self-closing': [
        'error',
        {
          html: { normal: 'never', void: 'always', component: 'always' },
          svg: 'always',
          math: 'always',
        },
      ],
    },
  })
  .override('nuxt/vue/rules', {
    files: ['app/pages/**/*.vue'],
    rules: {
      'vue/multi-word-component-names': 'off',
    },
  })
  .append({
    plugins: {
      'better-tailwindcss': betterTailwindcss,
    },
    rules: {
      ...betterTailwindcss.configs['recommended-warn'].rules,
      'better-tailwindcss/no-unknown-classes': [
        'warn',
        {
          ignore: ['^dark$'],
        },
      ],
    },
    settings: {
      'better-tailwindcss': {
        entryPoint: 'app/assets/css/tailwind.css',
        detectComponentClasses: true,
      },
    },
  })
  .append(...oxlint.configs['flat/recommended']);
