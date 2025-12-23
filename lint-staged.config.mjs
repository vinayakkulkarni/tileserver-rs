export default {
  '*.{cjs,mjs,js,ts,vue}': 'bun run --filter @tileserver-rs/client lint',
  '*.rs': 'cargo fmt --all -- --check',
};
