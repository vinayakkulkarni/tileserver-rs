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
  .override('nuxt/vue/rules', {
    files: ['app/components/ui/**/*.vue'],
    rules: {
      'vue/require-default-prop': 'off',
      'vue/one-component-per-file': 'off',
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
          entryPoint: 'app/assets/css/tailwind.css',
          // Skip arbitrary values that embed a CSS custom property, e.g.
          // `duration-[var(--d-fast,120ms)]`. Collapsing these to a static
          // utility (`duration-120`) would drop the design-token binding —
          // the motion tokens (--d-fast/--d-base/--d-slow) are the single
          // source of truth (CLAUDE.md Rule #20.H), so the reference must
          // survive canonicalization.
          ignore: ['\\[var\\('],
          // LTR-only app: don't rewrite physical properties (pl-/pr-/text-left)
          // to logical (ps-/pe-/text-start). Pure churn with no RTL surface.
          logical: false,
        },
      ],
      'better-tailwindcss/no-conflicting-classes': [
        'error',
        { entryPoint: 'app/assets/css/tailwind.css' },
      ],
      'better-tailwindcss/no-duplicate-classes': [
        'error',
        { entryPoint: 'app/assets/css/tailwind.css' },
      ],
      'better-tailwindcss/no-unnecessary-whitespace': [
        'error',
        { entryPoint: 'app/assets/css/tailwind.css' },
      ],
    },
    settings: {
      'better-tailwindcss': {
        entryPoint: 'app/assets/css/tailwind.css',
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
