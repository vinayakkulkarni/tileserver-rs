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
      // recommended-warn omits enforce-canonical-classes (it lives in
      // recommended-error); add it explicitly as an error so `--fix`
      // canonicalizes classes. ignore `[var(...)]` to preserve design-token
      // CSS-variable bindings (CLAUDE.md Rule #20.H motion tokens); logical:
      // false because this is an LTR-only marketing site.
      'better-tailwindcss/enforce-canonical-classes': [
        'error',
        {
          ignore: ['\\[var\\('],
          logical: false,
        },
      ],
      'better-tailwindcss/no-unknown-classes': [
        'warn',
        {
          ignore: ['^dark$'],
        },
      ],
      'better-tailwindcss/enforce-consistent-line-wrapping': 'off',
    },
    settings: {
      'better-tailwindcss': {
        entryPoint: 'app/assets/css/tailwind.css',
        detectComponentClasses: true,
      },
    },
  })
  .append({
    files: ['app/components/ui/**/*.vue'],
    rules: {
      'better-tailwindcss/enforce-canonical-classes': 'off',
    },
  })
  .append(...oxlint.configs['flat/recommended']);
