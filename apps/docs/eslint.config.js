// @ts-check
import { createConfigForNuxt } from '@nuxt/eslint-config/flat';
import betterTailwindcss from 'eslint-plugin-better-tailwindcss';
import oxlint from 'eslint-plugin-oxlint';

export default createConfigForNuxt({
  features: {
    stylistic: {
      semi: true,
    },
    tooling: true,
    typescript: true,
  },
})
  .override('nuxt/stylistic', {
    rules: {
      '@stylistic/arrow-parens': 'off',
      '@stylistic/brace-style': 'off',
      '@stylistic/indent': 'off',
      '@stylistic/indent-binary-ops': 'off',
      '@stylistic/operator-linebreak': 'off',
    },
  })
  .append({
    plugins: {
      'better-tailwindcss': betterTailwindcss,
    },
    rules: {
      'better-tailwindcss/enforce-canonical-classes': [
        'error',
        {
          entryPoint: 'app/assets/css/main.css',
          ignore: ['\\[var\\('],
          logical: false,
        },
      ],
      'better-tailwindcss/no-duplicate-classes': [
        'error',
        { entryPoint: 'app/assets/css/main.css' },
      ],
      'better-tailwindcss/no-unnecessary-whitespace': [
        'error',
        { entryPoint: 'app/assets/css/main.css' },
      ],
    },
    settings: {
      'better-tailwindcss': {
        entryPoint: 'app/assets/css/main.css',
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
