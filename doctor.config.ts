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
    // Perf benchmark harness — not part of the shipped app surface.
    'benchmarks/**',
    // Vendored shadcn-vue primitives across all three Nuxt apps — generated/
    // owned by the shadcn-vue CLI (`pnpm dlx shadcn-vue add ...`), not
    // hand-authored app code. Excluded so `shadcn-vue add` upgrades stay clean
    // and so their upstream patterns (props destructure in a `computed()`,
    // explicit reka-ui imports) are not counted as our slop.
    'apps/*/app/components/ui/**',
  ],
  rules: {
    // knip can't see CLI-invoked binaries: vue-tsc backs `nuxt typecheck` and
    // wrangler backs the deploy step (wrangler pages deploy dist) — both real,
    // neither imported.
    'dead-code/unused-dependency': 'off',
    // Nuxt's generated .nuxt/tsconfig.json already sets `strict: true`; the rule
    // reads the root tsconfig literally and misses the value inherited via
    // `extends`, so it is a false positive here.
    'vue-doctor/build-quality/tsconfig-strict-required': 'off',
  },
};
