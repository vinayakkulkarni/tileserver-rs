// Configuration for `@geoql/nuxt-doctor` (https://docs.the-doctor.report).
//
// NOTE: `exclude` REPLACES the built-in default ignore list rather than
// extending it, so the defaults (node_modules, dist, .nuxt, .output, coverage)
// are re-listed here explicitly. Drop any of them and the audit will start
// walking build output.
//
// Authored as a plain default export (no `defineConfig` import) because this
// file is loaded by the `pnpm dlx @geoql/nuxt-doctor` CLI via c12 — the package
// is not a local dependency of the workspace apps, so an import would not
// resolve in CI.
export default {
  exclude: [
    'node_modules',
    'dist',
    '.nuxt',
    '.output',
    '.data',
    'coverage',
    // This config file itself: consumed by the nuxt-doctor CLI via c12, never
    // imported by app code, so knip's dead-code pass flags it as unused.
    'doctor.config.ts',
    // The Rust workspace. nuxt-doctor is a Nuxt/Vue auditor; the only reason it
    // walks here is that `crates/mbgl-sys/vendor/maplibre-native/` is a vendored
    // C++ git submodule that ships bundled JS (jQuery, Doxygen doc assets,
    // VulkanMemoryAllocator + PMTiles vendored apps). Auditing third-party
    // vendored code produced 31 of 32 "errors" (v-html/eval in minified jQuery)
    // and 77 unused-file warnings — all false positives. Doctor must only see
    // the three Nuxt apps under apps/.
    'crates/**',
    // Any other vendored/bundled third-party trees, wherever they live.
    '**/vendor/**',
    // AI agent skill scripts (build.ts/validate.ts helpers). Tooling, never
    // imported by the apps, so knip flags them as unused files.
    '.claude/**',
    '.agents/**',
    // Perf benchmark harness — not part of the shipped app surface.
    'benchmarks/**',
    // Vendored shadcn-vue primitives across all three Nuxt apps — generated/
    // owned by the shadcn-vue CLI (`pnpm dlx shadcn-vue add ...`), not
    // hand-authored app code. Excluded so `shadcn-vue add` upgrades stay clean
    // and so their upstream patterns (props destructure in a `computed()`,
    // explicit reka-ui imports) are not counted as our slop.
    'apps/*/app/components/ui/**',
  ],
  // NOTE on vue-doctor/design/no-arbitrary-tailwind-values (kept ON, `warn`):
  // the residual warnings after tokenisation are all inherent false positives —
  // Tailwind variant selectors (data-[state=*], data-[side=*], group-data-*),
  // CSS-var references that ARE design tokens (duration-[var(--d-fast)],
  // ease-[var(--ease)]), reka-ui runtime bindings (w-[--reka-*]), and computed
  // values (calc(), inherit). No @theme token can express those more correctly.
  // The rule stays on as a tripwire: any NEW fixed arbitrary value (px/rem/%)
  // must be promoted to an @theme token, not merged as-is.
  rules: {
    // knip can't see CLI-invoked binaries: vue-tsc backs `nuxt typecheck` and
    // wrangler backs the deploy step (wrangler pages deploy dist) — both real,
    // neither imported.
    'dead-code/unused-dependency': 'off',
    // False positive on pnpm catalog + CSS subpath imports. knip flags
    // `@geoql/v-maplibre` as unlisted because apps/client/nuxt.config.ts imports
    // the `@geoql/v-maplibre/style.css` subpath, which knip can't map back to the
    // package — yet it IS listed in apps/client/package.json (catalog:client) and
    // installed. Same blind spot affects any catalog:-referenced dep imported via
    // a non-root subpath.
    'dead-code/unlisted-dependency': 'off',
    // knip is blind to Nuxt auto-imports. Every composable, `app/utils/**`
    // helper, and collection-key module is consumed via auto-import (never an
    // explicit `import`), so knip reports ~25 live files (use-home-page,
    // use-style-viewer, the whole utils/api TanStack layer, api-url, etc.) as
    // unused. Across the project's history this rule surfaced exactly one truly
    // dead file amid that permanent false-positive wall, so it is pure noise for
    // a Nuxt app. Same auto-import blind spot as the two dead-code rules above.
    'dead-code/unused-file': 'off',
    // Nuxt's generated .nuxt/tsconfig.json already sets `strict: true`; the rule
    // reads the root tsconfig literally and misses the value inherited via
    // `extends`, so it is a false positive here.
    'vue-doctor/build-quality/tsconfig-strict-required': 'off',
  },
};
