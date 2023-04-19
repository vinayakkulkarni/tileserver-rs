module.exports = {
  '*.{cjs,mjs,js,vue}': 'cd client && npm run lint:js',
  '*.{css,vue}': 'cd client && npm run lint:css',
  '*.rs': 'cargo fmt -q',
};
