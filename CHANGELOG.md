# Changelog

## [2.24.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.23.0...v2.24.0) (2026-04-08)


### Features

* **api:** replace Swagger UI with Scalar API Reference ([#769](https://github.com/vinayakkulkarni/tileserver-rs/issues/769)) ([a220716](https://github.com/vinayakkulkarni/tileserver-rs/commit/a220716fc9e49df8115d3dde3f1268c222407dcc))
* **sources:** add cloud object storage support (S3/Azure/GCS) ([#766](https://github.com/vinayakkulkarni/tileserver-rs/issues/766)) ([fac2924](https://github.com/vinayakkulkarni/tileserver-rs/commit/fac2924e40e75a50869bb2a6483dd9c44f889c68))


### Bug Fixes

* **ci:** change mbgl-sys release-type from rust to simple ([056873c](https://github.com/vinayakkulkarni/tileserver-rs/commit/056873c54e2a40326aefdbf9879afde36c26bc6f))
* **ci:** disable cargo-workspace plugin merge to allow independent mbgl-sys versioning ([85093e0](https://github.com/vinayakkulkarni/tileserver-rs/commit/85093e08897bb8c0b52dfba8f920c28e9cda270e))
* **ci:** read mbgl-sys version from manifest, revert to 0.1.3 ([2db6ada](https://github.com/vinayakkulkarni/tileserver-rs/commit/2db6ada8a9d2de269e741512f58fe58d7282a832))
* **ci:** remove cargo-workspace plugin entirely to prevent mbgl-sys version collision ([6c4360f](https://github.com/vinayakkulkarni/tileserver-rs/commit/6c4360f581008e93f0950880f4c1170695895f99))
* **ci:** remove mbgl-sys from release-please to prevent version collision ([e01ce92](https://github.com/vinayakkulkarni/tileserver-rs/commit/e01ce920baa0ed3b02d606e04bd635c8f5a5de11))


### Miscellaneous

* **deps:** bump reka-ui ^2.9.5, eslint-plugin-oxlint ^1.59.0, vite ^8.0.7 ([58c6b3b](https://github.com/vinayakkulkarni/tileserver-rs/commit/58c6b3b3da016eb1ca3232eb9df42323ef01e0ac))
* **homebrew:** update formula to v2.23.0 ([ac4b769](https://github.com/vinayakkulkarni/tileserver-rs/commit/ac4b769e18e0d676454e95ff9a84c2901d450902))

## [2.23.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.22.0...v2.23.0) (2026-04-07)


### Features

* **cache:** add [cache] config section and wire into SourceManager + admin ([a735f3a](https://github.com/vinayakkulkarni/tileserver-rs/commit/a735f3ada2c7276476b7ba216200619716c14a55))
* **cache:** extract TileCache to src/cache.rs as always-available global cache ([8297359](https://github.com/vinayakkulkarni/tileserver-rs/commit/829735907e4d93ad39929e36237c44cafdac1f25))
* **render,cache:** polygon fill, GeoJSON overlays, and global tile cache ([#749](https://github.com/vinayakkulkarni/tileserver-rs/issues/749)) ([c83897c](https://github.com/vinayakkulkarni/tileserver-rs/commit/c83897c717e44d694d1b965d43fe087ef88cda4f))


### Bug Fixes

* **ci:** add --no-default-features to coverage workflow ([2f34527](https://github.com/vinayakkulkarni/tileserver-rs/commit/2f345272e691d5bd7f609df21bedc3bdfd3f7aea))
* **ci:** add setup-node@v4 with Node 24 for ESLint 10.2 compatibility ([e56a1d5](https://github.com/vinayakkulkarni/tileserver-rs/commit/e56a1d5bcb6a8103af6361b448146a9e9e4e0797))
* **ci:** eliminate race condition in release workflows ([#742](https://github.com/vinayakkulkarni/tileserver-rs/issues/742)) ([98697e0](https://github.com/vinayakkulkarni/tileserver-rs/commit/98697e08ebfba96c6a6a098d561d230cd1a36909))
* **ci:** remove nonexistent http feature and fix --ignore-filename-regex ([62ed0cf](https://github.com/vinayakkulkarni/tileserver-rs/commit/62ed0cf89671b8c4af2490b08c8621774407ae2e))
* **ci:** run coverage across all feature combinations ([4d99da9](https://github.com/vinayakkulkarni/tileserver-rs/commit/4d99da925226b23a35e61cce04936565edabfb6c))
* **client:** all issues from vue and nuxt skills resolved ([38f38d9](https://github.com/vinayakkulkarni/tileserver-rs/commit/38f38d90de79c9c6c156d3d369b77af563cd6700))
* **client:** externalize optional v-maplibre peer deps in rollup ([adec207](https://github.com/vinayakkulkarni/tileserver-rs/commit/adec207d5078d44159b138a55f9375dfd660040a))
* **deps:** pin [@luma](https://github.com/luma).gl/engine to ~9.2.6 (tilde) to match deck.gl peer ([8f2ac2b](https://github.com/vinayakkulkarni/tileserver-rs/commit/8f2ac2b3d990a3d97600fa4f292cba283f74d07b))
* **deps:** revert [@luma](https://github.com/luma).gl/engine to ^9.2.6 to match deck.gl peer dep ([cc604d9](https://github.com/vinayakkulkarni/tileserver-rs/commit/cc604d950ac1211dfa9974d1a8f4e6e2aefdab4a))
* **duckdb:** support all WKB geometry types, not just points ([c095bd0](https://github.com/vinayakkulkarni/tileserver-rs/commit/c095bd09a83c2b6b0314666454c1662dbf1f924c)), closes [#736](https://github.com/vinayakkulkarni/tileserver-rs/issues/736)
* resolve 13 code review issues + duckdb geometry support ([#759](https://github.com/vinayakkulkarni/tileserver-rs/issues/759)) ([f74a545](https://github.com/vinayakkulkarni/tileserver-rs/commit/f74a5451a17f6268b582a219e6ac980d76d1b80b))
* **sources:** handle TileType::Mlt from pmtiles 0.21.0, bump deps ([a77d8e1](https://github.com/vinayakkulkarni/tileserver-rs/commit/a77d8e173d87760dc61fdc04ac060c07e111f085))
* **test:** remove needless borrow in cli test (clippy) ([e6ce338](https://github.com/vinayakkulkarni/tileserver-rs/commit/e6ce3385c3f59a71f93ccc019921bde3cc942d2f))
* **test:** update error display tests to match lowercased messages ([4695f01](https://github.com/vinayakkulkarni/tileserver-rs/commit/4695f010fa28df830703d627d21cabdf338563af))


### Miscellaneous

* bump dependencies ✨ ([4f34b30](https://github.com/vinayakkulkarni/tileserver-rs/commit/4f34b30959ac43c1dcb4ad4569c1b192f2872a9e))
* **deps:** bump actions/setup-node from v4 to v6 ([65b00a5](https://github.com/vinayakkulkarni/tileserver-rs/commit/65b00a549432cbe58d00c875d112c8115cf9aed7))
* **deps:** bump oxlint ^1.59.0, oxfmt ^0.44.0, shadcn-nuxt ^2.5.3 ([3751370](https://github.com/vinayakkulkarni/tileserver-rs/commit/3751370dbded1770ccbd760a6e6ae5a47c0d39bb))
* **deps:** update docs lockfile ([997ce55](https://github.com/vinayakkulkarni/tileserver-rs/commit/997ce55ec0e212b285397aa38bac500e459bbeff))
* **docs:** bump dependencies ✨ ([dd6649d](https://github.com/vinayakkulkarni/tileserver-rs/commit/dd6649d3bbca0e88377c46bfffa5928cfc0ebe6b))
* remove unused catalog section from root package.json ([34ba91e](https://github.com/vinayakkulkarni/tileserver-rs/commit/34ba91e4c55987e4ddc1dfd55528452f7168e3d5))


### Code Refactoring

* fix 13 Oracle code review issues across src/ ([019cb60](https://github.com/vinayakkulkarni/tileserver-rs/commit/019cb60076fb41105a165ed4a999fd44435f78dc))
* **render,sources:** remove stale #[allow(dead_code)] annotations ([9587bfa](https://github.com/vinayakkulkarni/tileserver-rs/commit/9587bfa6f0fa6f31f215fc9fd217f048cb2405b8))

## [2.22.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.21.1...v2.22.0) (2026-04-03)


### Features

* **docker:** enable geoparquet feature in Docker builds ([f6c7688](https://github.com/vinayakkulkarni/tileserver-rs/commit/f6c7688325974b065d4c17d11b585e3958e41f6e))


### Miscellaneous

* **docker:** remove redundant default features from FEATURES arg ([fd7593d](https://github.com/vinayakkulkarni/tileserver-rs/commit/fd7593dc48f54105257c4c7d51f1e6c2a1afb776))
* **homebrew:** update formula to v2.21.1 ([678090b](https://github.com/vinayakkulkarni/tileserver-rs/commit/678090b51576d45c1c8f4acc12119bcd9b0c60c7))

## [2.21.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.21.0...v2.21.1) (2026-04-03)


### Bug Fixes

* **ci:** auto-build native libs on mbgl-sys tag and add download retries ([182ae29](https://github.com/vinayakkulkarni/tileserver-rs/commit/182ae291ecf4710cd4dec82ec49046c97d7c1385))
* **geoparquet:** correct polygon rendering at low zoom levels ([#738](https://github.com/vinayakkulkarni/tileserver-rs/issues/738)) ([b594685](https://github.com/vinayakkulkarni/tileserver-rs/commit/b594685f03a3e4997525fe120257add146317b23)), closes [#736](https://github.com/vinayakkulkarni/tileserver-rs/issues/736)


### Documentation

* **ai-chat:** update available models from Hermes to Qwen3 ([5e0b48f](https://github.com/vinayakkulkarni/tileserver-rs/commit/5e0b48f8bd82795a89fbe4cf09e6ac447c8366c4))

## [2.21.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.20.1...v2.21.0) (2026-04-01)


### Features

* **client:** make AI chat panel draggable with minimize/close ([a103299](https://github.com/vinayakkulkarni/tileserver-rs/commit/a103299fe6e89dabcac43b6d320b232ba2550528))
* **docs:** add Hot Reload, PostGIS, and GeoParquet features to landing page ([a405696](https://github.com/vinayakkulkarni/tileserver-rs/commit/a4056964aeb26131f8564c5a5a80692684a4a04f))
* **marketing:** add Configurable Caching to features grid ([a3d5503](https://github.com/vinayakkulkarni/tileserver-rs/commit/a3d5503e42aafbd3c7ce44f063ed4bdea21b4f21))


### Bug Fixes

* **ci:** add workflow_dispatch to release-please for manual mbgl-sys publish ([9ea2424](https://github.com/vinayakkulkarni/tileserver-rs/commit/9ea2424fe609b5d76ec51695ff1953c3d9e19e64))
* **ci:** allow publish-mbgl-sys to run when release-please fails ([274fe9f](https://github.com/vinayakkulkarni/tileserver-rs/commit/274fe9fd6f9c2e931f4c60bc055f43f428e6260e))
* **ci:** use official crates-io-auth-action for trusted publishing ([65d03bd](https://github.com/vinayakkulkarni/tileserver-rs/commit/65d03bde56dc88958bd16c5e1c276ba3fc63a0bd))
* **ci:** use POST with JSON body for crates.io OIDC token exchange ([c37eb11](https://github.com/vinayakkulkarni/tileserver-rs/commit/c37eb11cef591ad8d595326c2f95f4559b51fdad))
* **docs:** update lockfile after dependency upgrades ([2ee54d6](https://github.com/vinayakkulkarni/tileserver-rs/commit/2ee54d60e3743799a2ddce70cb37fbd2da5c0170))


### Documentation

* eslint-plugin-oxlint/oxlint ^1.58.0, reka-ui ^2.9.3, wrangler ^4.79.0 ([928235b](https://github.com/vinayakkulkarni/tileserver-rs/commit/928235b792200166ecde56619efaba2f292b376b))


### Miscellaneous

* **deps:** update Rust and frontend dependencies ([928235b](https://github.com/vinayakkulkarni/tileserver-rs/commit/928235b792200166ecde56619efaba2f292b376b))

## [2.20.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.20.0...v2.20.1) (2026-03-30)


### Bug Fixes

* **ci:** make build-mbgl-native workflow idempotent for re-runs ([41b0df2](https://github.com/vinayakkulkarni/tileserver-rs/commit/41b0df28254640e3e42c547ccf4310ace11020af))
* **ci:** use OIDC token exchange for crates.io trusted publishing ([def0bfb](https://github.com/vinayakkulkarni/tileserver-rs/commit/def0bfb65f2fd62ef2fd2e99180a184c102028f0))
* **deps:** bump dependabot/fetch-metadata from 2.5.0 to 3.0.0 ([5552388](https://github.com/vinayakkulkarni/tileserver-rs/commit/5552388b4810123250496e1e24b1d7bd3e8e81b4))
* **deps:** bump dependabot/fetch-metadata from 2.5.0 to 3.0.0 ([#725](https://github.com/vinayakkulkarni/tileserver-rs/issues/725)) ([5552388](https://github.com/vinayakkulkarni/tileserver-rs/commit/5552388b4810123250496e1e24b1d7bd3e8e81b4))

## [2.20.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.19.0...v2.20.0) (2026-03-30)


### Features

* **client:** replace Hermes models with Qwen3 lineup for LLM chat ([8defec7](https://github.com/vinayakkulkarni/tileserver-rs/commit/8defec760e6368b40663b9ff438d7c598ae8df64))
* **mbgl-sys:** add prebuilt feature and CI workflow for native library distribution ([6c0472d](https://github.com/vinayakkulkarni/tileserver-rs/commit/6c0472d698abc785b4383a5794b817be952183f6))
* **mlt:** upgrade mlt-core 0.4→0.6 with full API migration ([07206b6](https://github.com/vinayakkulkarni/tileserver-rs/commit/07206b68bdd8034e2a6a8aede6d1b5f15f375b62))


### Bug Fixes

* **ci:** add MapLibre link-time dependencies to Rust CI ([0d51bd7](https://github.com/vinayakkulkarni/tileserver-rs/commit/0d51bd766ed14461bf6aafbba74930f772581f2c))
* **ci:** increase Rust CI timeout to 90min to prevent test cancellation ([9ceedb8](https://github.com/vinayakkulkarni/tileserver-rs/commit/9ceedb8eefa2ecb6ca345b34a65c7a153a112f4e))
* **ci:** normalize YAML indentation in release workflows ([7108613](https://github.com/vinayakkulkarni/tileserver-rs/commit/710861309e5d4c0f0033eec71865181347fe0a77))
* **ci:** use per-package include-component-in-tag for release-please ([8b12b6a](https://github.com/vinayakkulkarni/tileserver-rs/commit/8b12b6a1cc0825a6e68ecb9b44bb1501905ac738))
* **sources:** update duckdb import path after module rename ([0ae937a](https://github.com/vinayakkulkarni/tileserver-rs/commit/0ae937a4af5ffe857fa32233cbaf3878c0969ce9))


### Performance Improvements

* **ci:** remove redundant cargo check and release build from CI ([9087e43](https://github.com/vinayakkulkarni/tileserver-rs/commit/9087e4323b073ed40c68c94616d38bb57b8db3ab))
* **ci:** use pre-built MapLibre Native libs instead of rebuilding ([27055b1](https://github.com/vinayakkulkarni/tileserver-rs/commit/27055b1240f2358f04d52702c4cea359e37abaa6))
* use Vec::with_capacity on hot allocation paths ([e853443](https://github.com/vinayakkulkarni/tileserver-rs/commit/e85344374fcf8c4875e4e0bf774b920af968783c))


### Documentation

* add crates.io badges and prebuilt feature docs to READMEs ([54c9b10](https://github.com/vinayakkulkarni/tileserver-rs/commit/54c9b107f39c1634992a56da454aa3bcd898d236))
* add module-level documentation to source files ([d8a91f1](https://github.com/vinayakkulkarni/tileserver-rs/commit/d8a91f1c97162cad8e6268c24fcde9954fff981f))
* update documentation for project restructure ([7e799ff](https://github.com/vinayakkulkarni/tileserver-rs/commit/7e799ff1929433b157c72cbbecc6ea5f0f433377))


### Miscellaneous

* **deps:** update all dependency versions ([908f12a](https://github.com/vinayakkulkarni/tileserver-rs/commit/908f12ae3057bd9b5daa36b891e87ad3bcec243a))
* **deps:** update all dependency versions ([7c699a1](https://github.com/vinayakkulkarni/tileserver-rs/commit/7c699a15e3cb937c01753635b0dba1efe61fdc69))
* **deps:** update all dependency versions in source files ([273f639](https://github.com/vinayakkulkarni/tileserver-rs/commit/273f639a8e56ec8ca3cc06f96b1cafe49d89bb16))
* **homebrew:** update formula to v2.19.0 ([f9bb9b5](https://github.com/vinayakkulkarni/tileserver-rs/commit/f9bb9b551ac5252dac779bc47064108e41488e3e))


### Code Refactoring

* add #[must_use] on public pure functions ([f939663](https://github.com/vinayakkulkarni/tileserver-rs/commit/f939663fac3add4b2cc3f9c2351e8a4654553e62))
* add #[non_exhaustive] on public enums ([f722303](https://github.com/vinayakkulkarni/tileserver-rs/commit/f722303d860a824661d568e844355a101df3af2f))
* **client:** fix Vue/Nuxt anti-patterns — cleanup timers, API layer, types ([8a184f2](https://github.com/vinayakkulkarni/tileserver-rs/commit/8a184f246231ebce807d77791d7d2eafa8ad376a))
* fix Rust anti-patterns — eliminate clones, add with_capacity, must_use ([e59f742](https://github.com/vinayakkulkarni/tileserver-rs/commit/e59f742bb474c1deb32b2dffc23f3f9ef7b720fa))
* rename crate to mbgl-sys and restructure project layout ([4d292a6](https://github.com/vinayakkulkarni/tileserver-rs/commit/4d292a63b256912c6f834a06de6e1f29cb399a4f))
* replace index loop with windows(2) in draw_path ([d82b0b3](https://github.com/vinayakkulkarni/tileserver-rs/commit/d82b0b3013b8de64f2f7e4dc5dc1ce5735c6bcc4))
* replace unwrap with expect for better panic messages ([495c85f](https://github.com/vinayakkulkarni/tileserver-rs/commit/495c85f1b0e17c0a9739f49edf71da0d487d47b4))
* **routes:** extract route handlers from main.rs into modules ([6b3f8cb](https://github.com/vinayakkulkarni/tileserver-rs/commit/6b3f8cbb6200aaaa587fb762b7feb3c7e0b55dcb))

## [2.19.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.18.0...v2.19.0) (2026-03-19)


### Features

* **geoparquet:** support directory of parquet files + duckdb bbox vars ([f1642b3](https://github.com/vinayakkulkarni/tileserver-rs/commit/f1642b3dca3ad31c3ac320b830eb03129f167273)), closes [#711](https://github.com/vinayakkulkarni/tileserver-rs/issues/711)


### Bug Fixes

* **configs:** use relative paths instead of absolute paths ([d06f275](https://github.com/vinayakkulkarni/tileserver-rs/commit/d06f275083ebb2c5b1fff98d66efcadf48afa5ef))
* **deps:** bump actions/setup-node from 4 to 6 ([#697](https://github.com/vinayakkulkarni/tileserver-rs/issues/697)) ([a98875e](https://github.com/vinayakkulkarni/tileserver-rs/commit/a98875ef46e61ef75a7c9767883e6893a0710f94))
* **deps:** bump marocchino/sticky-pull-request-comment from 2 to 3 ([#698](https://github.com/vinayakkulkarni/tileserver-rs/issues/698)) ([6c76753](https://github.com/vinayakkulkarni/tileserver-rs/commit/6c76753b949473d3846ca988dbccc5e7dbf8d67b))
* **deps:** update better-sqlite3 requirement from ^12.6.2 to ^12.8.0 ([#701](https://github.com/vinayakkulkarni/tileserver-rs/issues/701)) ([0726308](https://github.com/vinayakkulkarni/tileserver-rs/commit/0726308f73e21c8a040bc33731277d36f363c773))
* **duckdb:** read column names after query execution ([754c7bf](https://github.com/vinayakkulkarni/tileserver-rs/commit/754c7bf75db4cb77e6248590dae414f3ccf4a145))
* **marketing:** fix malformed DuckDB feature card from rebase ([aa46b9a](https://github.com/vinayakkulkarni/tileserver-rs/commit/aa46b9acfb1af2b9692ec2030ecdcbef087f1316))


### Miscellaneous

* add overture styles and fix geoparquet config ([366ac4b](https://github.com/vinayakkulkarni/tileserver-rs/commit/366ac4b0785522519dc1b4519a96001e19386d91))
* **configs:** add DuckDB test config with Overture data ([8e59515](https://github.com/vinayakkulkarni/tileserver-rs/commit/8e5951540dc81fc277cfb75fa775a4ede17b8b25))
* **docs:** bump doc dependencies ✨ ([4b1754c](https://github.com/vinayakkulkarni/tileserver-rs/commit/4b1754ceb3793c352c21f02e623fc75e2a0bd1ca))
* **homebrew:** update formula to v2.18.0 ([d1b631e](https://github.com/vinayakkulkarni/tileserver-rs/commit/d1b631e562319cb2ed32087c92929305b671b840))
* update docs lockfile ([540beda](https://github.com/vinayakkulkarni/tileserver-rs/commit/540bedae20c0090183a7e945d0f1c445a7b06e0a))
* update root lockfile ([db54797](https://github.com/vinayakkulkarni/tileserver-rs/commit/db5479714a2eb8b0ee826d6daadf53afc000bb75))


### Code Refactoring

* consolidate config files into configs/ directory ([028453b](https://github.com/vinayakkulkarni/tileserver-rs/commit/028453bb2047a06d9d05707782c2f38a051eb8e8))

## [2.18.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.17.3...v2.18.0) (2026-03-19)


### Features

* **sources:** add duckdb source for sql-driven tile generation ([#707](https://github.com/vinayakkulkarni/tileserver-rs/issues/707)) ([97b8997](https://github.com/vinayakkulkarni/tileserver-rs/commit/97b8997808d854f0ce6227ff95a724a80d3bfb16))
* **sources:** add geoparquet source for serving tiles from parquet files ([#706](https://github.com/vinayakkulkarni/tileserver-rs/issues/706)) ([7daa62d](https://github.com/vinayakkulkarni/tileserver-rs/commit/7daa62db73820540715ce8b5639fed254cd45f55))


### Bug Fixes

* allow payloadExtraction & clean up lockfile ([7170a7e](https://github.com/vinayakkulkarni/tileserver-rs/commit/7170a7e812384d4bcb5140a4a978f2bad273fac3))
* **ci:** use Node.js for nuxi generate to avoid JSC stack overflow ([95fbe68](https://github.com/vinayakkulkarni/tileserver-rs/commit/95fbe68fe21bc715ae663ce0e276216886bff42e))


### Performance Improvements

* **render:** use proper PNG compression and eliminate unnecessary buffer clones ([4b33776](https://github.com/vinayakkulkarni/tileserver-rs/commit/4b33776268963f994022ffa55ba9e535dd39da09))


### Documentation

* add render pool configuration section ([46b65c2](https://github.com/vinayakkulkarni/tileserver-rs/commit/46b65c2560c56036de33298ade63dee00300b3fc))


### Miscellaneous

* **benchmarks:** add raster tile benchmark infrastructure ([9b6d219](https://github.com/vinayakkulkarni/tileserver-rs/commit/9b6d219e39df295f40dde69493804daa483e45db))
* **deps:** update all dependencies ([3f4a3eb](https://github.com/vinayakkulkarni/tileserver-rs/commit/3f4a3eb252ed9a280e1bddd7e75a9fe3dbc72f10))
* **homebrew:** update formula to v2.17.3 ([fe6a5f7](https://github.com/vinayakkulkarni/tileserver-rs/commit/fe6a5f74d547a2aca1275237c473d7f828e5ddc4))

## [2.17.3](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.17.2...v2.17.3) (2026-03-13)


### Bug Fixes

* **render:** keep persistent NativeMap per worker to prevent EGL race condition ([8ccc184](https://github.com/vinayakkulkarni/tileserver-rs/commit/8ccc184c3a05c3a123e11a0ac82ae15cf8edc43e))

## [2.17.2](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.17.1...v2.17.2) (2026-03-13)


### Bug Fixes

* **ci:** fix ulimit subshell bug in release-linux and ci-rust workflows ([51201f3](https://github.com/vinayakkulkarni/tileserver-rs/commit/51201f3c4528b189deabc84a070db64d16e49c63))
* **ci:** resolve web-llm CJS resolver stack overflow in release builds ([89e1bd2](https://github.com/vinayakkulkarni/tileserver-rs/commit/89e1bd2fc93b2efc6a08f7a7aa341559bdf52a15))
* **render:** use per-thread RunLoop to fix concurrent rendering deadlock ([610bd1e](https://github.com/vinayakkulkarni/tileserver-rs/commit/610bd1e8ae0ac4ef416323519646ef9e6053c1fb)), closes [#692](https://github.com/vinayakkulkarni/tileserver-rs/issues/692)

## [2.17.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.17.0...v2.17.1) (2026-03-13)


### Bug Fixes

* **docs:** move public/ to root level and pre-bundle Vite deps ([9547ac8](https://github.com/vinayakkulkarni/tileserver-rs/commit/9547ac8ed841376e4c9a239ced76a4f8e309bd1b))
* **marketing:** mobile responsive layout and visual fixes ([08f7f7a](https://github.com/vinayakkulkarni/tileserver-rs/commit/08f7f7a4adc8870ffa46e9da462745d690909b1c))
* **render:** rewrite renderer pool with concurrent worker threads ([96ed510](https://github.com/vinayakkulkarni/tileserver-rs/commit/96ed5107edba128db053abfab649db6fb63b6d1e)), closes [#692](https://github.com/vinayakkulkarni/tileserver-rs/issues/692)


### Miscellaneous

* **deps:** update workspace catalog dependencies ([1687af3](https://github.com/vinayakkulkarni/tileserver-rs/commit/1687af312918045bf7f4ff8cf1fb995b0b8b3ed1))
* **docs:** update lockfile and dependency versions ([2c1b091](https://github.com/vinayakkulkarni/tileserver-rs/commit/2c1b091da2222fd6afdd0e79485f638a47186de1))

## [2.17.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.16.2...v2.17.0) (2026-03-13)


### Features

* **apps:** redesign marketing and docs with geometric grid layout ([856ec05](https://github.com/vinayakkulkarni/tileserver-rs/commit/856ec05397f14a93109e9b43f4852952bb3ce8bb))
* **marketing:** add geometric grid design to all sections ([ce724f1](https://github.com/vinayakkulkarni/tileserver-rs/commit/ce724f1daea51651f3cc08ea2f037a9b2f0a9a76))


### Bug Fixes

* add fade transition and fix left gutter sizing ([12f533a](https://github.com/vinayakkulkarni/tileserver-rs/commit/12f533a4319fa72b154f3944f11751c33619b797))
* **docs:** add trailing newlines ([f226add](https://github.com/vinayakkulkarni/tileserver-rs/commit/f226add3f5a571e576bf5341e2228af58a0561a4))
* pin oxfmt to 0.36.0 to avoid reformatting template strings ([d0b79fe](https://github.com/vinayakkulkarni/tileserver-rs/commit/d0b79fe2416823249a50d49d42d3f2cd11ebb12a))
* properly implement geometric design for docs and marketing ([bb8141f](https://github.com/vinayakkulkarni/tileserver-rs/commit/bb8141f75a1c3e9b7c5cee97150372c9f37b9f84))
* remove FadeContent wrappers to match geolith ([9892043](https://github.com/vinayakkulkarni/tileserver-rs/commit/989204326d146655305b3601e0fe6815ea0cf039))
* **render:** resolve static rendering failures on GPU-less servers ([2986d5d](https://github.com/vinayakkulkarni/tileserver-rs/commit/2986d5d891de2ebdf72f09193e2931a6e2a02601))


### Miscellaneous

* pin oxfmt cause of the bug in oxfmt ([a341b03](https://github.com/vinayakkulkarni/tileserver-rs/commit/a341b03df30be48e03905da063107ae9850b8b4d))

## [2.16.2](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.16.1...v2.16.2) (2026-03-11)


### Miscellaneous

* add 7 marketing skills from coreyhaines31/marketingskills ([57c26c0](https://github.com/vinayakkulkarni/tileserver-rs/commit/57c26c001fe710224a19cf3ce93ccb8363e1e03e))

## [2.16.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.16.0...v2.16.1) (2026-03-11)


### Bug Fixes

* **ci:** add retry logic for web-llm stack overflow in release builds ([05b8ce4](https://github.com/vinayakkulkarni/tileserver-rs/commit/05b8ce44e47fb114c03a0a6f111912a437000b51))

## [2.16.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.15.1...v2.16.0) (2026-03-11)


### Features

* **docs:** add Galaxy WebGL starfield background to landing page ([47a781f](https://github.com/vinayakkulkarni/tileserver-rs/commit/47a781f9cd42e164b3edb34d80e5d352083a284b))
* **docs:** add SEO, rename composables, and update navigation ([ed9daa3](https://github.com/vinayakkulkarni/tileserver-rs/commit/ed9daa356c8d8cb9eb9015ca56972979346baa21))
* **marketing:** add ogl dependency for Galaxy starfield background ([545d3cb](https://github.com/vinayakkulkarni/tileserver-rs/commit/545d3cbb5ba71f3cc6e7c3fb28da466bdab57cf6))
* **marketing:** add SEO, AI section, and rename composables ([a19b124](https://github.com/vinayakkulkarni/tileserver-rs/commit/a19b1240bbcf4457b7513984e3e3125c7bf65279))
* **marketing:** highlight OpenAPI spec in API section with link to live docs ([e02bea3](https://github.com/vinayakkulkarni/tileserver-rs/commit/e02bea3f6c9d045a9521ee6cdffacda290f4cb49))
* **marketing:** replace Squares with Galaxy WebGL starfield ([d5fdd8c](https://github.com/vinayakkulkarni/tileserver-rs/commit/d5fdd8c716e54d9314153f149fdb0331fb50fcd9))


### Bug Fixes

* **ci:** allow Release Please and Dependabot branches in lint-branch ([919363d](https://github.com/vinayakkulkarni/tileserver-rs/commit/919363d65fab5020e1f73fa1e2a189482ddab15e))
* **ci:** wrap ulimit -s unlimited with error handling on macOS ([7a3e7a3](https://github.com/vinayakkulkarni/tileserver-rs/commit/7a3e7a392ac9ae5fd561c1c4ed147fce9b4eb8a7))
* **docs:** resolve 211 eslint errors from stricter rule enforcement ([f19663c](https://github.com/vinayakkulkarni/tileserver-rs/commit/f19663c7c32dfe0ff7d8aa1e5a83015153cdb091))
* **marketing:** prevent GlareHover from blocking button clicks ([952e10f](https://github.com/vinayakkulkarni/tileserver-rs/commit/952e10f5e2dcc2e2a033e94411b80055593f9f48))
* **marketing:** remove max-w-6xl from nav for full-width consistency ([ca44459](https://github.com/vinayakkulkarni/tileserver-rs/commit/ca444595acc27632a0233f7ebc61df0d2812a552))


### Documentation

* **guides:** add browser-local AI chat guide ([c55c6db](https://github.com/vinayakkulkarni/tileserver-rs/commit/c55c6db206f2659d5d7935d8d0c01a0a0dbed1cb))


### Miscellaneous

* add project skills for marketing, SEO, and Vue/Nuxt ([3946a4f](https://github.com/vinayakkulkarni/tileserver-rs/commit/3946a4f1c2a75e8f009a0c29d306196e3f334ad4))
* **deps:** update all dependencies to latest versions ([cb8af56](https://github.com/vinayakkulkarni/tileserver-rs/commit/cb8af56eb3e5f99731bc23935a550df053cdca30))

## [2.15.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.15.0...v2.15.1) (2026-03-10)


### Bug Fixes

* **ci:** use node --stack-size for Nuxt builds to fix @mlc-ai/web-llm CJS resolver stack overflow ([6da1aaa](https://github.com/vinayakkulkarni/tileserver-rs/commit/6da1aaa2998f2be1c9625cf166c808bde0b149a5))
* **ci:** use ulimit -s unlimited with bun run instead of node --stack-size hack ([1fca05a](https://github.com/vinayakkulkarni/tileserver-rs/commit/1fca05ab60505dcf8d886a488babbe1f7b8dfa98))
* **docs:** add deploy and file-drop guides to sidebar navigation ([d76376c](https://github.com/vinayakkulkarni/tileserver-rs/commit/d76376c69881ea27ce657385d6135783c19762e5))
* **tests:** update tag count assertion for new Upload and Spatial tags ([909ae12](https://github.com/vinayakkulkarni/tileserver-rs/commit/909ae1281bf861e9405c71aabbe541e040202bc2))


### Documentation

* update docs, marketing, and openapi for v2.15.1 ([a7e2e5f](https://github.com/vinayakkulkarni/tileserver-rs/commit/a7e2e5f5d04a7a279234481f7b5d5027bbbba379))


### Miscellaneous

* **deps:** update all Cargo.toml version specifiers to latest, upgrade mlt-core 0.3→0.4 ([19da5d3](https://github.com/vinayakkulkarni/tileserver-rs/commit/19da5d389e7675383daa071ca4f5cb853a0be4ac))
* **deps:** update cargo and bun dependencies ([89ef82d](https://github.com/vinayakkulkarni/tileserver-rs/commit/89ef82d76f77b22749c44c831623911e290e9d89))

## [2.15.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.14.1...v2.15.0) (2026-03-09)


### Features

* **client:** add get_overlays tool for Map AI overlay awareness ([0a113f5](https://github.com/vinayakkulkarni/tileserver-rs/commit/0a113f55877362e828eb60b3305549eb1872a5d3))

## [2.14.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.14.0...v2.14.1) (2026-03-09)


### Bug Fixes

* **client:** reject standalone .shp files with clear error message ([a70349f](https://github.com/vinayakkulkarni/tileserver-rs/commit/a70349f69c0b69c099f80e731aad62d203af3060))

## [2.14.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.13.2...v2.14.0) (2026-03-09)


### Features

* **client:** drag-and-drop file visualization in the built-in viewer ([#672](https://github.com/vinayakkulkarni/tileserver-rs/issues/672)) ([6756add](https://github.com/vinayakkulkarni/tileserver-rs/commit/6756add236b8bdf977a4195e4fbae794c48d1fcb))

## [2.13.2](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.13.1...v2.13.2) (2026-03-09)


### Bug Fixes

* **bench:** replace deprecated criterion::black_box with std::hint::black_box ([3829ba1](https://github.com/vinayakkulkarni/tileserver-rs/commit/3829ba108c934c07890da0472ab4799e3aecda4e))


### Miscellaneous

* **deps:** upgrade mlt-core 0.3, toml 1.0, criterion 0.8 and bump npm deps ([2775425](https://github.com/vinayakkulkarni/tileserver-rs/commit/2775425612454b8a9111635e580040ade03c428e))
* **docs:** bump dependencies ✨ ([4cd63e5](https://github.com/vinayakkulkarni/tileserver-rs/commit/4cd63e5adaa47d0fd17b705a0c2eb5601c81ce52))
* **docs:** bump eslint and install better-sqlite3 ✨ ([e253660](https://github.com/vinayakkulkarni/tileserver-rs/commit/e25366079138194f58417d15c54f520e8ebc608d))


### Code Refactoring

* **client:** replace @maplibre/maplibre-gl-inspect with maplibre-gl-inspect ([b9e238a](https://github.com/vinayakkulkarni/tileserver-rs/commit/b9e238ac446393d9a33894b42326b59a745bae37))

## [2.13.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.13.0...v2.13.1) (2026-03-06)


### Miscellaneous

* **deps:** regenerate lockfiles with latest compatible versions ([b9760ef](https://github.com/vinayakkulkarni/tileserver-rs/commit/b9760efb6a83939c5dae2a3e0de68ba8637fe3f6))
* **deps:** restructure bun workspace catalogs to named-only ([#669](https://github.com/vinayakkulkarni/tileserver-rs/issues/669)) ([32bad16](https://github.com/vinayakkulkarni/tileserver-rs/commit/32bad16432afc616670a2b35c41389d0a44c6fe3))
* **deps:** update cargo dependencies and refresh lockfiles ([91683b9](https://github.com/vinayakkulkarni/tileserver-rs/commit/91683b98a07663290d0d4542441097a7b4e5184d))

## [2.13.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.12.1...v2.13.0) (2026-03-06)


### Features

* one-click deploy buttons for cloud platforms ([9950e0d](https://github.com/vinayakkulkarni/tileserver-rs/commit/9950e0d6dddb2234dd65bc54f0b5a56ee60a4f47))
* one-click deploy with automatic sample data download ([1a649d5](https://github.com/vinayakkulkarni/tileserver-rs/commit/1a649d57ee5d30daa9ebe0c00dbda35be6d3b2b3)), closes [#603](https://github.com/vinayakkulkarni/tileserver-rs/issues/603)
* **readme:** add Railway deploy button with live template URL ([0b05ade](https://github.com/vinayakkulkarni/tileserver-rs/commit/0b05ade676c1a402932736d098c5ebff6b4230c9))


### Bug Fixes

* **deps:** bump docker/build-push-action from 6 to 7 ([#664](https://github.com/vinayakkulkarni/tileserver-rs/issues/664)) ([ba07d53](https://github.com/vinayakkulkarni/tileserver-rs/commit/ba07d53ed3f101fa8adcc37ee65f8012481796bf))
* **deps:** bump docker/metadata-action from 5 to 6 ([239614e](https://github.com/vinayakkulkarni/tileserver-rs/commit/239614eaff03307f022c531b9404b2e2fcc2019b))
* **deps:** bump docker/metadata-action from 5 to 6 ([#663](https://github.com/vinayakkulkarni/tileserver-rs/issues/663)) ([239614e](https://github.com/vinayakkulkarni/tileserver-rs/commit/239614eaff03307f022c531b9404b2e2fcc2019b))
* **deps:** bump docker/setup-buildx-action from 3 to 4 ([#662](https://github.com/vinayakkulkarni/tileserver-rs/issues/662)) ([0a2d9e0](https://github.com/vinayakkulkarni/tileserver-rs/commit/0a2d9e018770115fb17c24aabf09be42bc347cb3))

## [2.12.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.12.0...v2.12.1) (2026-03-05)


### Bug Fixes

* **deps:** bump docker/login-action from 3 to 4 ([1120c4b](https://github.com/vinayakkulkarni/tileserver-rs/commit/1120c4b4331af6a24364c72606c050ba614eccbd))
* **deps:** bump docker/login-action from 3 to 4 ([505adca](https://github.com/vinayakkulkarni/tileserver-rs/commit/505adca151db9aff6720f75ddcaf3f5b8aa1e451))
* **deps:** bump lucide-vue-next from 0.576.0 to 0.577.0 ([c930440](https://github.com/vinayakkulkarni/tileserver-rs/commit/c9304407794c9493ef601f6e5407d1cfdef1e3a0))
* **deps:** bump lucide-vue-next from 0.576.0 to 0.577.0 ([3027cba](https://github.com/vinayakkulkarni/tileserver-rs/commit/3027cbaa628c613a81b922b397b7d05b5c8153c2))
* **server:** graceful MVT fallback on transcode errors ([eb2be93](https://github.com/vinayakkulkarni/tileserver-rs/commit/eb2be935d40d95319114488bbf0d0cf067e77716))
* **transcode:** upgrade mlt-core to 0.2.0 and adapt encoding API ([c17de25](https://github.com/vinayakkulkarni/tileserver-rs/commit/c17de25de0ec0c3fbda6274153de50ca9bf05591))


### Miscellaneous

* **deps:** upgrade zod v3 to v4, update outdated packages ([efb863b](https://github.com/vinayakkulkarni/tileserver-rs/commit/efb863b2b41d68ed123d2adb5af9e24072dded2c))
* **docs:** bump dependencies ✨ ([0a96dac](https://github.com/vinayakkulkarni/tileserver-rs/commit/0a96dac18a24dc3416134c11ef6103d1d5e1f80c))
* **marketing:** bump dependencies ✨ ([aa33191](https://github.com/vinayakkulkarni/tileserver-rs/commit/aa33191a450d0fbea3c66af296839914d84946c7))

## [2.12.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.11.3...v2.12.0) (2026-03-04)


### Features

* **api:** add spatial API endpoints for LLM tool integration ([a4e080e](https://github.com/vinayakkulkarni/tileserver-rs/commit/a4e080ed73bc0ff9d51f2f56d6c62952894726a8))
* **client:** add command palette UI and robust tool intent parser ([639c974](https://github.com/vinayakkulkarni/tileserver-rs/commit/639c974fc6bc250d4a86abafa0f8e61b93c1bfcc))
* **client:** add landing page redesign + browser-local LLM chat panel ([b7c5f9a](https://github.com/vinayakkulkarni/tileserver-rs/commit/b7c5f9a135fff165009043a80cb8a6c92b7daf19))
* **client:** add proper tool calling with TanStack AI toolDefinition ([bf44ee2](https://github.com/vinayakkulkarni/tileserver-rs/commit/bf44ee2090839841d9cfa15adf48899b22eb100e))
* **client:** add server tools, chat persistence, and model selection ([962e508](https://github.com/vinayakkulkarni/tileserver-rs/commit/962e5083f9d96b634669709dc372b8cd26685e61))
* **client:** browser-local LLM chat panel for map viewer ([aa5b3f7](https://github.com/vinayakkulkarni/tileserver-rs/commit/aa5b3f76123f9bddf5f66fa7a2d321996a524d3a))


### Bug Fixes

* **client:** enable scroll in command palette messages area ([994d120](https://github.com/vinayakkulkarni/tileserver-rs/commit/994d120ec646f4d60754a9bb13e9f3a30d9f2cfe))
* **client:** inject real map state when tool calling fails ([f0c7f77](https://github.com/vinayakkulkarni/tileserver-rs/commit/f0c7f77fb391a59a22289e90e7ff52fafd5a1a4e))
* **client:** polish LLM chat panel UI and fix X button overlap + duplicate messages ([4b7fb61](https://github.com/vinayakkulkarni/tileserver-rs/commit/4b7fb61c4afffda676bb484c6dc4820e422150b4))
* **client:** remove backdrop blur from command palette scrim ([d91dce3](https://github.com/vinayakkulkarni/tileserver-rs/commit/d91dce3669ed3a5d3bb60081f94966e3eb5c2faa))
* **client:** remove max-w-5xl container constraint for full-width layout ([2853fdb](https://github.com/vinayakkulkarni/tileserver-rs/commit/2853fdb2347fa68271113adf25d82e1c303510eb))
* **client:** restore scroll position when reopening command palette ([5ff58c3](https://github.com/vinayakkulkarni/tileserver-rs/commit/5ff58c39bee994766777750dd9efad70936d514d))
* **client:** suppress internal WebLLM errors from end users ([449f1d0](https://github.com/vinayakkulkarni/tileserver-rs/commit/449f1d0e16a24f71756a350ca01f653e5324e4d7))
* **client:** use catalog for reka-ui instead of hardcoded ^2.8.2 ([e9768d2](https://github.com/vinayakkulkarni/tileserver-rs/commit/e9768d2478420c2acef3c9c239a8878265d54f22))
* **client:** use native scroll for command palette messages ([2440553](https://github.com/vinayakkulkarni/tileserver-rs/commit/24405538bf46bf5b49573f55fba67339696957e0))


### Miscellaneous

* **homebrew:** update formula to v2.11.3 ([3490329](https://github.com/vinayakkulkarni/tileserver-rs/commit/3490329312f00e8e18e927b41acb765db1b9c05a))

## [2.11.3](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.11.2...v2.11.3) (2026-03-04)


### Bug Fixes

* **transcode:** catch mlt-core panics during MVT→MLT encoding ([194a6d4](https://github.com/vinayakkulkarni/tileserver-rs/commit/194a6d4c03dda72115ae814538b03227d5bd2e47)), closes [#651](https://github.com/vinayakkulkarni/tileserver-rs/issues/651)


### Miscellaneous

* **homebrew:** update formula to v2.11.2 ([620c28d](https://github.com/vinayakkulkarni/tileserver-rs/commit/620c28d088c1b40eb9524e85bb6a3134a8a96b11))

## [2.11.2](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.11.1...v2.11.2) (2026-03-04)


### Bug Fixes

* **docker:** add cmake to rust-builder for mlt-core/fastpfor build ([319059a](https://github.com/vinayakkulkarni/tileserver-rs/commit/319059a5ae59ef54e204027e89dcd9241a249920))

## [2.11.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.11.0...v2.11.1) (2026-03-04)


### Bug Fixes

* **build:** enable mlt feature by default in Cargo.toml and Dockerfile ([#647](https://github.com/vinayakkulkarni/tileserver-rs/issues/647)) ([7044b7c](https://github.com/vinayakkulkarni/tileserver-rs/commit/7044b7cfeeef0724aa695999a2a4e31f10f36b98)), closes [#646](https://github.com/vinayakkulkarni/tileserver-rs/issues/646)

## [2.11.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.10.0...v2.11.0) (2026-03-04)


### Features

* **marketing:** add responsive hamburger menu for mobile navigation ([b302791](https://github.com/vinayakkulkarni/tileserver-rs/commit/b30279117cb08caa4d4c07b6ec6c855d1491fa10))
* **transcode:** implement MVT to MLT encoding via mlt-core ([1e7ec77](https://github.com/vinayakkulkarni/tileserver-rs/commit/1e7ec77716cb81383e0d97c1507e7d1389812fdd)), closes [#641](https://github.com/vinayakkulkarni/tileserver-rs/issues/641)
* **transcode:** implement MVT to MLT encoding via mlt-core ([#644](https://github.com/vinayakkulkarni/tileserver-rs/issues/644)) ([27277f7](https://github.com/vinayakkulkarni/tileserver-rs/commit/27277f7e1c7fe499dd6cc3bde1fa443fb41c5876))


### Bug Fixes

* **serve:** return error on transcode failure instead of wrong-format data ([6788fc4](https://github.com/vinayakkulkarni/tileserver-rs/commit/6788fc4ef7bb6105ef9e42c6db881b3d10c09e0d))
* **transcode:** resolve clippy approx_constant and useless_vec warnings ([91d7138](https://github.com/vinayakkulkarni/tileserver-rs/commit/91d7138c1b082aa9610f84a02a8ce47da2e6a05d))


### Miscellaneous

* **homebrew:** update formula to v2.10.0 ([207cefe](https://github.com/vinayakkulkarni/tileserver-rs/commit/207cefe6cf169cfe8abbabb94551f5a5b3674dc6))

## [2.10.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.9.1...v2.10.0) (2026-03-03)


### Features

* add dark/light mode toggle to marketing and docs ([696cf73](https://github.com/vinayakkulkarni/tileserver-rs/commit/696cf73160eb645e86fe7d89a7860724a3a47013))
* **client:** add @maplibre/mlt dependency ([44d5fb5](https://github.com/vinayakkulkarni/tileserver-rs/commit/44d5fb51cc3a10ea77c6c2a71a9899dc0d7e3038))
* **docs:** add copy-to-clipboard button for code blocks ([8ef4b1c](https://github.com/vinayakkulkarni/tileserver-rs/commit/8ef4b1cc665db580d9cb6f69cf456da0f9c775b5))
* **docs:** add edit/report buttons and improved prev/next navigation ([34f0ba4](https://github.com/vinayakkulkarni/tileserver-rs/commit/34f0ba4b048c8fb37fd0ac7f245fe9478edc13e3))
* **docs:** migrate from Docus to @nuxt/content ([8301847](https://github.com/vinayakkulkarni/tileserver-rs/commit/83018474063cefa36aa90f5e82f06c1eca213c45))
* **marketing:** add GlareHover to Live Demo button, remove GradientText ([4d840cb](https://github.com/vinayakkulkarni/tileserver-rs/commit/4d840cb7b635b85d48f46941873d5b2a7801dd8d))
* **marketing:** add ShinyText component for hero heading ([d2d53df](https://github.com/vinayakkulkarni/tileserver-rs/commit/d2d53df9d0d71b43abb6f209cf43aa4becc92625))
* **marketing:** replace static bg-grid with animated Squares component ([41aa1a4](https://github.com/vinayakkulkarni/tileserver-rs/commit/41aa1a4a8032377a0ec67053ff64ca15a67998bb))


### Bug Fixes

* **ci:** add build retry for flaky font provider fetches ([c2a413c](https://github.com/vinayakkulkarni/tileserver-rs/commit/c2a413ca2c091fdc6ebfc1aab497c0a71a0e9e95))
* **client:** sharp design for style viewer and data inspector ([1789536](https://github.com/vinayakkulkarni/tileserver-rs/commit/178953623b27e9aedeb7fcc5ac433dd80ddae2b8))
* **client:** use min-h-dvh for dynamic viewport height ([517bd8c](https://github.com/vinayakkulkarni/tileserver-rs/commit/517bd8c85c7a24f621f9df9a2a03b99701277b5e))
* **docs:** add missing closing div in homepage nav ([5396d7d](https://github.com/vinayakkulkarni/tileserver-rs/commit/5396d7ddef5079e9ffeaf96e8727106b26658529))
* **docs:** add theme toggle to homepage (layout: false bypass) ([e18dbe8](https://github.com/vinayakkulkarni/tileserver-rs/commit/e18dbe80df36c1766eec8803870f1ce41d3eec06))
* **docs:** full-width homepage nav and refined scrollbar styles ([a90ec46](https://github.com/vinayakkulkarni/tileserver-rs/commit/a90ec46c333d46ed1cae47493c97e4f597f8deb7))
* **docs:** remove duplicate border above edit/report buttons ([4bee9b0](https://github.com/vinayakkulkarni/tileserver-rs/commit/4bee9b0382b136b6c782f562e7ab4654a3f73cbe))
* **docs:** resolve eslint/oxfmt conflicts and lint warnings ([a98ae1f](https://github.com/vinayakkulkarni/tileserver-rs/commit/a98ae1f25441bb94636146f65aa26b606af8aa46))
* **docs:** use min-h-dvh and prevent horizontal overflow ([2ffe3d3](https://github.com/vinayakkulkarni/tileserver-rs/commit/2ffe3d34e70507180090238c9a6d984c6764d4ac))
* **marketing:** use min-h-dvh, prevent horizontal overflow, fix code block overflow ([742bd35](https://github.com/vinayakkulkarni/tileserver-rs/commit/742bd35c495f9577fb71b92208ee026a79a00c15))
* **marketing:** use theme-aware colors for light mode support ([885d87d](https://github.com/vinayakkulkarni/tileserver-rs/commit/885d87dfbc822b5faa0f2d152ddf0c394f686b26))


### Miscellaneous

* update lockfiles ([d30621e](https://github.com/vinayakkulkarni/tileserver-rs/commit/d30621e89b5079e17f58fb8f7cbd61528919f24b))
* update lockfiles and dependencies ([44d8d3e](https://github.com/vinayakkulkarni/tileserver-rs/commit/44d8d3e64cf8de9b7afa7a024f34c90f10efe820))


### Code Refactoring

* **client:** extract composables and home sub-components ([cff2a00](https://github.com/vinayakkulkarni/tileserver-rs/commit/cff2a00cbf67fc8b33c51aaf5877d08387bce52e))
* **marketing:** extract types, composables, and sub-components ([6d8bbf3](https://github.com/vinayakkulkarni/tileserver-rs/commit/6d8bbf3dfa11277dd1f248df5bf725509427db0b))

## [2.9.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.9.0...v2.9.1) (2026-03-03)


### Bug Fixes

* **ci:** dispatch release-linux.yml instead of nonexistent per-arch files ([8efe293](https://github.com/vinayakkulkarni/tileserver-rs/commit/8efe2938cbcc74926717edbf0a105c187988850c))
* **sources:** apply serve_as format override in source loading ([#637](https://github.com/vinayakkulkarni/tileserver-rs/issues/637)) ([df0ab70](https://github.com/vinayakkulkarni/tileserver-rs/commit/df0ab7009478b700f52a34a201cca4954974a582))


### Documentation

* add TiTiler benchmarks, demo link, and performance highlights ([e603011](https://github.com/vinayakkulkarni/tileserver-rs/commit/e60301198a3c2165af3c4bdd7ce20596721fa480))


### Miscellaneous

* **docs:** update agents dep ✨ ([7fb0d8e](https://github.com/vinayakkulkarni/tileserver-rs/commit/7fb0d8ed7464fa6dcd289c54a0e6cc6450be9363))

## [2.9.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.8.0...v2.9.0) (2026-03-03)


### Features

* **marketing:** add MLT transcoding feature card and config example ([c679913](https://github.com/vinayakkulkarni/tileserver-rs/commit/c679913fa895fde3dbdf75a4abca66fbf607f9bd))
* **mlt:** add MLT transcoding error types ([75a091c](https://github.com/vinayakkulkarni/tileserver-rs/commit/75a091caf9a46f70a63ad77fd8589c8c4f6279cb))
* **mlt:** add MLT-to-MVT transcoding module ([b26782e](https://github.com/vinayakkulkarni/tileserver-rs/commit/b26782ec4dc92002c2c43c69b7eb6e8d4efd79dc))
* **mlt:** add serve_as config field for format transcoding ([02cdbc2](https://github.com/vinayakkulkarni/tileserver-rs/commit/02cdbc287123f9dd5635f118c39876ec0fbbc521))
* **mlt:** wire transcoding into lib exports and tile handler ([00ae7df](https://github.com/vinayakkulkarni/tileserver-rs/commit/00ae7df1be8cc8df427daddf791ff4196a2fff98))
* **styles:** inject MLT encoding hint in rewritten styles ([e57557f](https://github.com/vinayakkulkarni/tileserver-rs/commit/e57557f48e58465f4b50eea29ae3babe74f03912))


### Bug Fixes

* **ci:** prevent headless manifest from overwriting :latest tag ([#621](https://github.com/vinayakkulkarni/tileserver-rs/issues/621)) ([3603d25](https://github.com/vinayakkulkarni/tileserver-rs/commit/3603d25c7c82009d55424e8b97cd39791c809b14)), closes [#620](https://github.com/vinayakkulkarni/tileserver-rs/issues/620)
* **deps:** bump actions/download-artifact from 7 to 8 ([bcf45d8](https://github.com/vinayakkulkarni/tileserver-rs/commit/bcf45d8453f0edf90d0d834d754af5b27a804e26))
* **deps:** bump actions/download-artifact from 7 to 8 ([68ea5f7](https://github.com/vinayakkulkarni/tileserver-rs/commit/68ea5f713177e73a8e75e95bdeea677c25b6d251))
* **deps:** bump actions/upload-artifact from 6 to 7 ([10c6cb2](https://github.com/vinayakkulkarni/tileserver-rs/commit/10c6cb253cd1c5b11fb11c0b3698c5bf176f662a))
* **docker:** copy benches directory for Cargo manifest validation ([888ecb3](https://github.com/vinayakkulkarni/tileserver-rs/commit/888ecb38de6856a4d1b3853aa1557fc23d1c8661))


### Documentation

* add MLT transcoding feature card to docs landing page ([2518c0d](https://github.com/vinayakkulkarni/tileserver-rs/commit/2518c0d195c7d299633cd1908cc323e163039f67))
* **mlt:** add MLT guide with benchmark results and config examples ([8f6b93c](https://github.com/vinayakkulkarni/tileserver-rs/commit/8f6b93cf80c3b8b45e3766e4b815a69925b6893b))
* **site:** add /ping, /__admin/reload, zero-config, and admin_bind to docs ([a5373ba](https://github.com/vinayakkulkarni/tileserver-rs/commit/a5373bac40ca6687314f325c09916965388d90bb))
* update README, ARCHITECTURE, and CLAUDE.md with MLT support ([2aa6f2c](https://github.com/vinayakkulkarni/tileserver-rs/commit/2aa6f2c488dbfb9f1ed1016ac8507bda3e0af50f))
* update README, ARCHITECTURE, and CLAUDE.md with new endpoints and features ([b6a7dea](https://github.com/vinayakkulkarni/tileserver-rs/commit/b6a7deac12390479a0d5e3b6183af8ff3a25b808))


### Miscellaneous

* **deps-dev:** update wrangler requirement from ^4.67.0 to ^4.68.0 ([#628](https://github.com/vinayakkulkarni/tileserver-rs/issues/628)) ([3cd43c3](https://github.com/vinayakkulkarni/tileserver-rs/commit/3cd43c386bc00c3ebe63aaef485fc8d6e7671958))
* **deps:** update agents to v0.7.1 in docs ([0efa344](https://github.com/vinayakkulkarni/tileserver-rs/commit/0efa344ece11d46d6cf41f304928750040c6e149))
* **deps:** update cargo and bun dependencies ([06c71c9](https://github.com/vinayakkulkarni/tileserver-rs/commit/06c71c91b5d7dc0d0f2755f79240858205b6d128))
* **deps:** update oxfmt, lucide-vue-next, and @tanstack/vue-db ([becb1ed](https://github.com/vinayakkulkarni/tileserver-rs/commit/becb1edb0e0a92e530cefbd76bc88944f0fc1d53))


### Code Refactoring

* remove git_hash from ping response and delete build.rs ([442c692](https://github.com/vinayakkulkarni/tileserver-rs/commit/442c6929885c8bda99aad6df187a82381392b3e6))

## [2.8.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.7.1...v2.8.0) (2026-02-20)


### Features

* add MapLibre Tiles (MLT) format support ([88593fa](https://github.com/vinayakkulkarni/tileserver-rs/commit/88593faf49682cdeac14c5f900a40b15cd0621f8))
* **ci:** add headless build variants and consolidate Linux release workflows ([a55e281](https://github.com/vinayakkulkarni/tileserver-rs/commit/a55e281849b0b259c4dffee2bc350f2ab12fb2bd))
* **client:** add encoding field to Data type for MLT sources ([3597934](https://github.com/vinayakkulkarni/tileserver-rs/commit/3597934d074155660f45f3bec9dd0e45dcb7bb1f))
* **mbtiles:** detect MLT format from metadata ([cf92e8d](https://github.com/vinayakkulkarni/tileserver-rs/commit/cf92e8da8fb49dcff55a005d950b4373e41c226e))
* **pmtiles:** auto-detect MLT format for Unknown tile type ([ed25589](https://github.com/vinayakkulkarni/tileserver-rs/commit/ed255896879ec20e9b49870e5cb20db24c748e27))
* **server:** add zero-config auto-detect and hot-reload ([dab2b1f](https://github.com/vinayakkulkarni/tileserver-rs/commit/dab2b1fd4f8fdcc214e83cdda76c221b280b4df2)), closes [#593](https://github.com/vinayakkulkarni/tileserver-rs/issues/593)
* **server:** version footer with git hash and build hardening ([#618](https://github.com/vinayakkulkarni/tileserver-rs/issues/618)) ([18e16a4](https://github.com/vinayakkulkarni/tileserver-rs/commit/18e16a4d91591198c27653831f74cd42949ae132))
* **server:** zero-config auto-detect and hot-reload ([c51973a](https://github.com/vinayakkulkarni/tileserver-rs/commit/c51973ab38bdcc9bb78e052f2bd7746657e56a9d))
* **sources:** add MLT tile format variant with detection and TileJSON encoding ([e484fb2](https://github.com/vinayakkulkarni/tileserver-rs/commit/e484fb2087476cb3c87c7d1fd491f31d904712a4))
* **styles:** add MLT encoding to rewritten style sources ([d94bd5c](https://github.com/vinayakkulkarni/tileserver-rs/commit/d94bd5cedebfb9ca75cb3ec0fce38956906f6a36))
* **ui:** show version and git hash in footer via /ping endpoint ([a6f01cb](https://github.com/vinayakkulkarni/tileserver-rs/commit/a6f01cbf2e04f1c96783d1c6a6273139dec622ef))


### Bug Fixes

* **docker:** copy build.rs into rust-builder stage ([2123f4b](https://github.com/vinayakkulkarni/tileserver-rs/commit/2123f4b49ee5ec264f56eca9e59e486c4fc02b86))
* **docker:** remove duplicate -headless suffix in manifest inspect step ([aebc16a](https://github.com/vinayakkulkarni/tileserver-rs/commit/aebc16a84c8f18a6f3d5d450f29280c026e1077c))
* **docker:** skip frontend build for headless variant ([08d669e](https://github.com/vinayakkulkarni/tileserver-rs/commit/08d669eb69e887f7a976f603a5a92dee78b1663e))
* **server:** embed git hash at build time and fix OpenAPI version ([7134314](https://github.com/vinayakkulkarni/tileserver-rs/commit/71343144152696ac94324db16dba95697ec0cc59))
* **server:** use library crate imports in binary to fix clippy CI ([eb3e996](https://github.com/vinayakkulkarni/tileserver-rs/commit/eb3e996e9e0cc748c1f7c110a9453837604e219f))
* **src:** apply rust-skills best practice fixes ([22b605e](https://github.com/vinayakkulkarni/tileserver-rs/commit/22b605e2370807c000478555449ddcea5c0e541e))


### Performance Improvements

* **src:** add #[inline], #[non_exhaustive], and Vec::with_capacity for hot paths ([d767fe3](https://github.com/vinayakkulkarni/tileserver-rs/commit/d767fe363a2118f4e2418264ada35b9967853b48))


### Documentation

* add MLT format to API endpoints and feature comparison ([b504b48](https://github.com/vinayakkulkarni/tileserver-rs/commit/b504b489bb6a168b0cba72c96c8d8f7a28538feb))
* **benchmarks:** update comparison tables with latest benchmark results ([2c65e62](https://github.com/vinayakkulkarni/tileserver-rs/commit/2c65e62f93c6b8e31640c6250119cb76dfabf4b3))
* **guides:** add auto-detect and hot-reload documentation ([2ab3c79](https://github.com/vinayakkulkarni/tileserver-rs/commit/2ab3c79d414d1ad7ec1fc6aedd94bb04da4d995a))
* **guides:** add PostgreSQL reload behavior to hot-reload guide ([9663f2d](https://github.com/vinayakkulkarni/tileserver-rs/commit/9663f2d955a99a76087536647e5edf6d8c2f3dff))
* **marketing:** add MLT format to features and API endpoints ([93b2cc5](https://github.com/vinayakkulkarni/tileserver-rs/commit/93b2cc5adb9c4b87ccfb142a650f0377b7c44c97))


### Miscellaneous

* add GitHub issue templates for bug reports and feature requests ([984e349](https://github.com/vinayakkulkarni/tileserver-rs/commit/984e34972572a6c6268cbc1db5bcdff5c8129d37))
* add rust-skills submodule for Rust best practice enforcement ([7a62de8](https://github.com/vinayakkulkarni/tileserver-rs/commit/7a62de8a985bb9c4eb1014895d04b83ab2c60e35))
* **deps-dev:** bump agents from 0.4.1 to 0.5.0 ([f2161a6](https://github.com/vinayakkulkarni/tileserver-rs/commit/f2161a6b95be65c1bbe2175cae403f43fab6e2a1))
* **deps-dev:** bump agents from 0.4.1 to 0.5.0 ([48f7d17](https://github.com/vinayakkulkarni/tileserver-rs/commit/48f7d171b3a82dcfdaf4602c70c57b3e1a67506d))
* **deps-dev:** update wrangler requirement from ^4.65.0 to ^4.66.0 ([#607](https://github.com/vinayakkulkarni/tileserver-rs/issues/607)) ([da2b516](https://github.com/vinayakkulkarni/tileserver-rs/commit/da2b516ea2a12ec990dd59754e6615b69a72298a))
* **deps:** update cargo and bun dependencies ([16e0157](https://github.com/vinayakkulkarni/tileserver-rs/commit/16e01575d76cf7136247a996d4251215aaabc627))
* **deps:** update cargo and bun dependencies ([64fada4](https://github.com/vinayakkulkarni/tileserver-rs/commit/64fada44d6e201966efe7ecd19c00423efe2bb2d))
* **deps:** update cargo and bun dependencies ([5083d25](https://github.com/vinayakkulkarni/tileserver-rs/commit/5083d250fbbf4f53a62d313dac90e649f41b00b2))
* **deps:** update cargo and bun dependencies ([d464658](https://github.com/vinayakkulkarni/tileserver-rs/commit/d464658701e13bdedb1b823470ff08837cd9e4b1))
* **docs:** bump dependencies ✨ ([a86320b](https://github.com/vinayakkulkarni/tileserver-rs/commit/a86320b54a705554625da629015a7d779fe72576))
* **homebrew:** update formula to v2.7.1 ([51729de](https://github.com/vinayakkulkarni/tileserver-rs/commit/51729de46a699adc161951804607a1d86152a95e))

## [2.7.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.7.0...v2.7.1) (2026-02-11)


### Bug Fixes

* **build:** add `frontend` cargo feature to gate rust-embed dependency ([f8416ce](https://github.com/vinayakkulkarni/tileserver-rs/commit/f8416ce133939f86a2764055841d5b3fe9463d77)), closes [#582](https://github.com/vinayakkulkarni/tileserver-rs/issues/582)
* **build:** allow backend compilation without frontend via `frontend` feature flag ([#583](https://github.com/vinayakkulkarni/tileserver-rs/issues/583)) ([e5e1b67](https://github.com/vinayakkulkarni/tileserver-rs/commit/e5e1b672fd43f568325dc59d33ac13a37d91099e))
* **build:** gate embedded SPA behind `frontend` feature flag ([6251599](https://github.com/vinayakkulkarni/tileserver-rs/commit/6251599e031acf20744f49ef84c46b4d352424f3))
* **docker:** add --fix-missing to all apt-get install commands ([5003713](https://github.com/vinayakkulkarni/tileserver-rs/commit/50037139dff1f55d60f8bd512fcfcf48af4cf668))


### Documentation

* document `frontend` feature flag and cargo features ([e388489](https://github.com/vinayakkulkarni/tileserver-rs/commit/e38848917adcd16eb75f91d32fc9fe6013f309be))


### Miscellaneous

* bump internal devDeps ([abd70ac](https://github.com/vinayakkulkarni/tileserver-rs/commit/abd70aceaadf7977969d8d65e892423659c037cc))
* **deps-dev:** bump agents from 0.3.10 to 0.4.0 ([7be30b0](https://github.com/vinayakkulkarni/tileserver-rs/commit/7be30b0a6195c6c92722981c7c12ecdf5d3d68de))
* **deps-dev:** bump agents from 0.3.10 to 0.4.0 ([4cc7df5](https://github.com/vinayakkulkarni/tileserver-rs/commit/4cc7df50b4fed3195575ff32550737ab625f1f37))
* **deps:** update cargo, node, and benchmark dependencies ([e9ad6ac](https://github.com/vinayakkulkarni/tileserver-rs/commit/e9ad6ac2b10daabbc5e4ad141cc1121932f5e0cb))
* **homebrew:** update formula to v2.7.0 ([b4d1619](https://github.com/vinayakkulkarni/tileserver-rs/commit/b4d16191a7f99f9a81c5f19d855c98e44d54e930))

## [2.7.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.6.1...v2.7.0) (2026-02-05)


### Features

* **client:** add layer visibility toggle to data inspector ([4398edb](https://github.com/vinayakkulkarni/tileserver-rs/commit/4398edb29c9176ee700361f5de6651f58b5c3f99))


### Bug Fixes

* **client:** clear URL hash on back navigation ([4d27e87](https://github.com/vinayakkulkarni/tileserver-rs/commit/4d27e87bea32c6442ae7e5176a78a2fd8d6ddf12))


### Miscellaneous

* **homebrew:** update formula to v2.6.1 ([478c065](https://github.com/vinayakkulkarni/tileserver-rs/commit/478c0651d0c000cc7328a07828d32e8db4185106))

## [2.6.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.6.0...v2.6.1) (2026-02-05)


### Bug Fixes

* **ci:** remove duplicate dependabot config for docs ([6b076df](https://github.com/vinayakkulkarni/tileserver-rs/commit/6b076df99a1539f152a7d26f89a3fc9fe9ca5999))
* **client:** remove hardcoded hash from navigation links ([c5b9270](https://github.com/vinayakkulkarni/tileserver-rs/commit/c5b9270ffc9a32fb7853d61fee4aec6d14aa52a1))


### Miscellaneous

* **deps:** bump agents to v0.3.10 ([531d450](https://github.com/vinayakkulkarni/tileserver-rs/commit/531d4502bfc1c5544b1207582e2ef1f3ba54e80c))
* **homebrew:** update formula to v2.6.0 ([69a2aec](https://github.com/vinayakkulkarni/tileserver-rs/commit/69a2aec66793d0f6a3cb0c8619d89f242c79a166))
* **homebrew:** update formula to v2.6.0 ([db2ffe3](https://github.com/vinayakkulkarni/tileserver-rs/commit/db2ffe3c7829ffc4964adfb11c39a94ebbbad321))

## [2.6.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.5.1...v2.6.0) (2026-02-04)


### Features

* **client:** add motion-v with collapsible layers panel ([c665459](https://github.com/vinayakkulkarni/tileserver-rs/commit/c665459c1bc03130fd0c53a0d31713f7109456ed))


### Bug Fixes

* **client:** clear map hash before back navigation ([0219311](https://github.com/vinayakkulkarni/tileserver-rs/commit/02193114ee5761dad976455025c03e9b10884261))


### Documentation

* document thin-template rule for Vue components ([5640e71](https://github.com/vinayakkulkarni/tileserver-rs/commit/5640e71cdc7f34f8614b51b305fb2e081307cf92))


### Miscellaneous

* **deps:** add motion-v and bump workspace dependencies ([0e116b3](https://github.com/vinayakkulkarni/tileserver-rs/commit/0e116b39af389aea3f2e28e3a1df984656a1fa1f))
* **deps:** bump docs dependencies ([5081bbc](https://github.com/vinayakkulkarni/tileserver-rs/commit/5081bbc0264b5a3f4e9a43a581108d0d4c8d4d59))
* **deps:** bump workspace catalog dependencies ([698ee1a](https://github.com/vinayakkulkarni/tileserver-rs/commit/698ee1a11b7d4fe4a820a3ae226441f8740401a4))
* **homebrew:** update formula to v2.5.1 ([41fc252](https://github.com/vinayakkulkarni/tileserver-rs/commit/41fc252eb725ff1b3d8eaed4e3227446c7d1c74d))
* **homebrew:** update formula to v2.5.1 ([c6be3ca](https://github.com/vinayakkulkarni/tileserver-rs/commit/c6be3cac06041f2319e177c0357f05018cb2417c))
* ignore dirty content in maplibre-native submodule ([fa7d91f](https://github.com/vinayakkulkarni/tileserver-rs/commit/fa7d91f7c3fb1bf165f3969eccaa97254203974c))


### Code Refactoring

* **client:** move logic from components to composables ([53e5930](https://github.com/vinayakkulkarni/tileserver-rs/commit/53e593021d9e7e1acd20e304c71ed7d5b206266e))

## [2.5.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.5.0...v2.5.1) (2026-02-01)


### Bug Fixes

* always start Xvfb and return empty PBF for missing font ranges ([0104bfd](https://github.com/vinayakkulkarni/tileserver-rs/commit/0104bfd7be02316b6f90f4363decc97ccfb8c8b3))


### Miscellaneous

* **homebrew:** update formula to v2.5.0 ([94e278c](https://github.com/vinayakkulkarni/tileserver-rs/commit/94e278cc133e8ca853fd6a35df2c380737874fde))

## [2.5.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.4.2...v2.5.0) (2026-01-31)


### Features

* **telemetry:** add metrics config fields to TelemetryConfig ([4dd9aeb](https://github.com/vinayakkulkarni/tileserver-rs/commit/4dd9aebf196f57a13cd12ecccaa64c5565b4b27a))
* **telemetry:** add OTEL metrics pipeline with OTLP gRPC export ([5da8475](https://github.com/vinayakkulkarni/tileserver-rs/commit/5da84759b4fca5dafb397bd280bfb36c52edd0ea))
* **telemetry:** record HTTP metrics in request logging middleware ([7867daa](https://github.com/vinayakkulkarni/tileserver-rs/commit/7867daaf0dd1b521b7fdb70a8f00215a2aa6aad2))


### Bug Fixes

* **client:** match tileserver-gl data inspector style ([fb74d7b](https://github.com/vinayakkulkarni/tileserver-rs/commit/fb74d7be4cb8fe47b948eea1da40b4f98f91f5cb))
* **client:** resolve route param type errors in dynamic pages ([fcb97ad](https://github.com/vinayakkulkarni/tileserver-rs/commit/fcb97addd67da567850c86b682c78aedde0b766f))
* **deps:** bump @tanstack/vue-db from 0.0.93 to 0.0.94 ([76aec48](https://github.com/vinayakkulkarni/tileserver-rs/commit/76aec48f56425a10fcb1927532a22dd4f5dd9ea6))
* **deps:** bump @tanstack/vue-db from 0.0.93 to 0.0.94 ([009fb82](https://github.com/vinayakkulkarni/tileserver-rs/commit/009fb82f3cff9d83c17fcb18d8fe5789e9f9ab7b))
* **deps:** bump @tanstack/vue-db from 0.0.96 to 0.0.97 ([f32adb1](https://github.com/vinayakkulkarni/tileserver-rs/commit/f32adb1b144a933653ed95d0dd58490dd1b98c3d))
* **deps:** bump @tanstack/vue-db from 0.0.96 to 0.0.97 ([fffeadf](https://github.com/vinayakkulkarni/tileserver-rs/commit/fffeadf45729838b6d98301b6362ae34906431cd))
* **docker:** drop --frozen-lockfile with --filter ([3285e76](https://github.com/vinayakkulkarni/tileserver-rs/commit/3285e76d7f43550e9c2a969c04930edad3a4687c))
* **docker:** use --filter to install only client workspace deps ([3ac69e0](https://github.com/vinayakkulkarni/tileserver-rs/commit/3ac69e02635373ac0c51681ef839e5ba7213d0f8))
* **docs:** simplify Nitro config to fix CI build ([24cd3ee](https://github.com/vinayakkulkarni/tileserver-rs/commit/24cd3ee575e47a8adda17a9b98aca902ed35bed7))


### Documentation

* **config:** document telemetry configuration options ([e665606](https://github.com/vinayakkulkarni/tileserver-rs/commit/e6656066f71b0ce4c211725c9a29a77b512bc8a5))
* **guides:** add telemetry and observability guide ([9ae348c](https://github.com/vinayakkulkarni/tileserver-rs/commit/9ae348ca697f4483e78b0554d245b3bcf7a50651))


### Miscellaneous

* add VS Code workspace settings for OXC toolchain ([c619919](https://github.com/vinayakkulkarni/tileserver-rs/commit/c619919a04af6e14ecc5c6cc24323f601b755b54))
* adopt Bun workspace catalogs for dependency management ([f03c458](https://github.com/vinayakkulkarni/tileserver-rs/commit/f03c458dab63a1a79eb579187812f595be7ec581))
* bump dependencies 🤷‍♂️ ([2d7dad3](https://github.com/vinayakkulkarni/tileserver-rs/commit/2d7dad3c70e72abcb5bf5f226dbce2ae77ded80d))
* **client:** clean up static assets ([1d56598](https://github.com/vinayakkulkarni/tileserver-rs/commit/1d56598349bb35e52a83d37cb310df2a46c7aeda))
* **deps:** upgrade Cargo dependencies with breaking changes ([6f349a3](https://github.com/vinayakkulkarni/tileserver-rs/commit/6f349a3dc7c4b27b4b158a8c532d4905ac12823d))
* enable-beta-ecosystems for latest deps w/ lockfiles ([2b69535](https://github.com/vinayakkulkarni/tileserver-rs/commit/2b69535750c0e1b8e25efc9f99f1eebeff0f1aca))
* **homebrew:** update formula to v2.4.2 ([cd9d0ed](https://github.com/vinayakkulkarni/tileserver-rs/commit/cd9d0ed3fbd31586b959b03d734200cc9a5d4b14))
* ignore sisyphus ([c3ea773](https://github.com/vinayakkulkarni/tileserver-rs/commit/c3ea7734765a69166f80d097fae1e1e0b34bca6d))
* update dependabot config for Bun catalog workflow ([c799ca5](https://github.com/vinayakkulkarni/tileserver-rs/commit/c799ca5653ea81919e1e154073f0034fcf644e0f))


### Code Refactoring

* **client:** migrate from Prettier to OXC toolchain ([0b14adb](https://github.com/vinayakkulkarni/tileserver-rs/commit/0b14adb722771282f5bfdee9329b535b5e031bd2))
* **marketing:** migrate from Prettier/stylistic to OXC toolchain ([b9f92f5](https://github.com/vinayakkulkarni/tileserver-rs/commit/b9f92f58653bc54cdcef33faca1bde4a47ff2bd1))

## [2.4.2](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.4.1...v2.4.2) (2026-01-09)


### Bug Fixes

* **ci:** add major version tag to Docker releases ([f5b013d](https://github.com/vinayakkulkarni/tileserver-rs/commit/f5b013de901e6b8079433352e8e9bf53654485fb))


### Miscellaneous

* **homebrew:** update formula to v2.4.1 ([3dabab1](https://github.com/vinayakkulkarni/tileserver-rs/commit/3dabab1412b60b876919abe17b816de84762e0a8))

## [2.4.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.4.0...v2.4.1) (2026-01-09)


### Bug Fixes

* **client:** hide Inspect and Services for non-vector data sources ([097e815](https://github.com/vinayakkulkarni/tileserver-rs/commit/097e8157b5d8de562234e79bf80c4bbef8e7e2dc))


### Miscellaneous

* **deps:** bump dependencies ([b6877ed](https://github.com/vinayakkulkarni/tileserver-rs/commit/b6877edcfb1a6d91d13185bcf43704c8398d5e2c))

## [2.4.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/tileserver-rs-v2.3.0...tileserver-rs-v2.4.0) (2026-01-08)


### Features

* **raster:** add rescale_mode="none" for discrete class colormaps ([c675b80](https://github.com/vinayakkulkarni/tileserver-rs/commit/c675b809b4030fbe0df72406ce39d73d14b18b5a)), closes [#514](https://github.com/vinayakkulkarni/tileserver-rs/issues/514)
* workflows updated 🤷‍♂️ ([82f9e12](https://github.com/vinayakkulkarni/tileserver-rs/commit/82f9e12fcdd8fdf576138f8530e2366995286c50))


### Bug Fixes

* add public_url config for Docker port mapping and fix docs build ([aa90e5c](https://github.com/vinayakkulkarni/tileserver-rs/commit/aa90e5c1469b4b4751038e88a913742c4ae2c0e8))
* **deps:** Bump @tanstack/vue-db from 0.0.92 to 0.0.93 ([8991b31](https://github.com/vinayakkulkarni/tileserver-rs/commit/8991b31945bdde4e9b12669bf67221bc71335b6a))
* **deps:** Bump @tanstack/vue-db from 0.0.92 to 0.0.93 ([7a44b47](https://github.com/vinayakkulkarni/tileserver-rs/commit/7a44b47a4502aaf9693afc930b441987190fa3b0))
* **deps:** Bump actions/download-artifact from 4 to 7 ([#507](https://github.com/vinayakkulkarni/tileserver-rs/issues/507)) ([3fc7de6](https://github.com/vinayakkulkarni/tileserver-rs/commit/3fc7de64763a5113ada862d22a750dfe3a929ce2))
* **deps:** Bump actions/upload-artifact from 4 to 6 ([#508](https://github.com/vinayakkulkarni/tileserver-rs/issues/508)) ([6dd9168](https://github.com/vinayakkulkarni/tileserver-rs/commit/6dd91680b45e2e5d69081a9177c8da97f73335db))
* **docs:** prerendering causign bugs ([f05d6b3](https://github.com/vinayakkulkarni/tileserver-rs/commit/f05d6b36b1a8c1e12cfee99257dcd069ea76c61a))
* update nuxt config ([de045d2](https://github.com/vinayakkulkarni/tileserver-rs/commit/de045d2c61f940e2157927bc1708c3f7a6de73d5))


### Miscellaneous

* lockfile updated ([90d4e74](https://github.com/vinayakkulkarni/tileserver-rs/commit/90d4e746a6ee63d52697b9dcd26fc7b8198936f8))
* update docs repo ([756ec67](https://github.com/vinayakkulkarni/tileserver-rs/commit/756ec673cde0895b4f58abb2c1df5bbee7534317))
* update root lockfile for agents dependency ([f4d7752](https://github.com/vinayakkulkarni/tileserver-rs/commit/f4d775201f91ceff13aa7d05567dcf57373f3e76))


### Code Refactoring

* move docs/ and marketing/ into apps/ directory ([261e2f9](https://github.com/vinayakkulkarni/tileserver-rs/commit/261e2f9a500f982ef8a55b87107f425e7f5c6376))

## [2.3.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/tileserver-rs-v2.2.0...tileserver-rs-v2.3.0) (2026-01-07)


### Features

* **postgres:** support dynamic rescaling from PostgreSQL function return values ([a787906](https://github.com/vinayakkulkarni/tileserver-rs/commit/a787906a2e6837b7e455848f0c23db7dd451b4a7)), closes [#506](https://github.com/vinayakkulkarni/tileserver-rs/issues/506)


### Bug Fixes

* **openapi:** consolidate duplicate tile endpoints into single documented path ([657d2c4](https://github.com/vinayakkulkarni/tileserver-rs/commit/657d2c4fad40849810ee7d8eeb9f59ccb16e8e80))


### Miscellaneous

* **ci:** rename docker workflow to release-docker.yml for consistency ([d676741](https://github.com/vinayakkulkarni/tileserver-rs/commit/d676741d0861ff002969889f1836b11e769f21ff))

## [2.2.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/tileserver-rs-v2.1.1...tileserver-rs-v2.2.0) (2026-01-07)


### Features

* **docker:** build multi-arch images for linux/amd64 and linux/arm64 ([cac14d3](https://github.com/vinayakkulkarni/tileserver-rs/commit/cac14d3157c8d43de389f5a81f99768ec7032353))


### Bug Fixes

* **ci:** enable Linux ARM64 in Release Please trigger matrix ([21d2bc9](https://github.com/vinayakkulkarni/tileserver-rs/commit/21d2bc9f0289d74626652c4314935c4545dabb00))
* **postgres:** route function sources to vector tile handler when raster enabled ([32a5c4e](https://github.com/vinayakkulkarni/tileserver-rs/commit/32a5c4efb2f933ff1e6c08843ceeca52da6958f1))


### Miscellaneous

* **homebrew:** update formula to v2.1.1 ([cebcb19](https://github.com/vinayakkulkarni/tileserver-rs/commit/cebcb19b6e6b43cad3b54bb2b6229a371e118423))
* **homebrew:** update formula to v2.1.1 ([76cf7ad](https://github.com/vinayakkulkarni/tileserver-rs/commit/76cf7adb7832c0a28efc4b31260aae241399e5c7))

## [2.1.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/tileserver-rs-v2.1.0...tileserver-rs-v2.1.1) (2026-01-07)


### Bug Fixes

* **postgres:** pass query params to function sources when raster feature enabled ([346c603](https://github.com/vinayakkulkarni/tileserver-rs/commit/346c603988aeb2cfb4d6291c80f353fd87a98fa4))


### Miscellaneous

* **main:** release tileserver-rs 2.1.0 ([fc6603a](https://github.com/vinayakkulkarni/tileserver-rs/commit/fc6603ae2651505236e988fda129c8a931c6a032))
* **main:** release tileserver-rs 2.1.0 ([3baa666](https://github.com/vinayakkulkarni/tileserver-rs/commit/3baa666b7f20d82427b60a7faa509c5e85359d9f))

## [2.1.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/tileserver-rs-v2.0.0...tileserver-rs-v2.1.0) (2026-01-07)


### Features

* **ci:** enable Linux ARM64 release workflow ([b634eb1](https://github.com/vinayakkulkarni/tileserver-rs/commit/b634eb142fdbe42d90c867cc05ed828d364e98a3))


### Bug Fixes

* **build:** detect build-linux-opengl directory for MapLibre Native ([c57d7b9](https://github.com/vinayakkulkarni/tileserver-rs/commit/c57d7b9f4fedb77d2717730ec86ba4ed85325c2a))
* **ci:** use correct ARM64 runner label ubuntu-24.04-arm ([be615f5](https://github.com/vinayakkulkarni/tileserver-rs/commit/be615f5ad3cc1c5c51a4d02210c2948e96440a2e))

## [2.0.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/tileserver-rs-v1.0.0...tileserver-rs-v2.0.0) (2026-01-07)


### ⚠ BREAKING CHANGES

* **raster:** Requires 'raster' feature flag to enable COG support

### Features

* enable postgres and raster features by default ([7c545e6](https://github.com/vinayakkulkarni/tileserver-rs/commit/7c545e6fa9c5a003b4dc41ab33907053b8dc3153))
* **postgres:** add out-of-database raster source support ([b6068c2](https://github.com/vinayakkulkarni/tileserver-rs/commit/b6068c2aa2ea7ef3433641cafbd8fbca9871aaa5))
* **raster:** add Cloud Optimized GeoTIFF (COG) tile serving ([c529c25](https://github.com/vinayakkulkarni/tileserver-rs/commit/c529c25557ddf70e14f6700bfe75e25656a7fe55))


### Bug Fixes

* **ci:** add GDAL dependency for raster feature in CI and Docker ([609e58e](https://github.com/vinayakkulkarni/tileserver-rs/commit/609e58e213da6b613c8b371d69362618da15e96f))
* **ci:** add libclang-dev and pkg-config to all workflows ([930f13b](https://github.com/vinayakkulkarni/tileserver-rs/commit/930f13bd2f0aba8f92053f0c01bb2ac006c66a53))
* **deps:** Bump dependabot/fetch-metadata from 2.4.0 to 2.5.0 ([a79c2cf](https://github.com/vinayakkulkarni/tileserver-rs/commit/a79c2cf12a43e0a87db56ba941a9d42e3e6f331a))
* **deps:** Bump dependabot/fetch-metadata from 2.4.0 to 2.5.0 ([e90f004](https://github.com/vinayakkulkarni/tileserver-rs/commit/e90f00444efae12bae26300a11ccd0e132e5ee3f))
* **docker:** add libclang-dev for gdal-sys bindgen ([2e07a00](https://github.com/vinayakkulkarni/tileserver-rs/commit/2e07a00a0f16d25ae9d49e673699296e8b070b41))
* **docker:** add pkg-config for gdal-sys header discovery ([838d234](https://github.com/vinayakkulkarni/tileserver-rs/commit/838d2344c631adc97f662da74cbb9237d5391e71))
* **postgres:** forward URL query parameters to PostgreSQL function sources ([f348757](https://github.com/vinayakkulkarni/tileserver-rs/commit/f348757d522794866b66baab5c0871ecbc24aa88))
* resolve all clippy warnings in tests and postgres sources ([ad4b4a2](https://github.com/vinayakkulkarni/tileserver-rs/commit/ad4b4a23e0883b43f0fc03a630d8390403efebf0))
* update postgres test signatures and resolve clippy warnings ([ce311d6](https://github.com/vinayakkulkarni/tileserver-rs/commit/ce311d615a9ef66d9c917ea3eaef59c2e532e621))
* use derive macro for ColorMapType default and remove unused colormap method ([04ee086](https://github.com/vinayakkulkarni/tileserver-rs/commit/04ee086ef0eb10de30aee480c36df0077b029805))


### Performance Improvements

* only serialize query params to JSON for PostgreSQL sources ([f2913f0](https://github.com/vinayakkulkarni/tileserver-rs/commit/f2913f0adf1582efe8bb2243e62deb14c455630a))


### Miscellaneous

* **deps-dev:** Update @commitlint/cli requirement from ^20.2.0 to ^20.3.0 ([#487](https://github.com/vinayakkulkarni/tileserver-rs/issues/487)) ([ba200d8](https://github.com/vinayakkulkarni/tileserver-rs/commit/ba200d821450659f507b022aa161982bf81e4d2e))
* **deps-dev:** Update @commitlint/config-conventional requirement from ^20.2.0 to ^20.3.0 ([#486](https://github.com/vinayakkulkarni/tileserver-rs/issues/486)) ([c3eeb40](https://github.com/vinayakkulkarni/tileserver-rs/commit/c3eeb409d83210b208fd14998073158f065e893d))
* **deps:** update frontend dependencies ([f8dfd52](https://github.com/vinayakkulkarni/tileserver-rs/commit/f8dfd52437ce2bfd9e66a2c7247452248e0750f1))
* **homebrew:** update formula to v1.0.0 ([58f5914](https://github.com/vinayakkulkarni/tileserver-rs/commit/58f5914d14355f8211702863c3ce3015d9277b49))
* **homebrew:** update formula to v1.0.0 ([949fedb](https://github.com/vinayakkulkarni/tileserver-rs/commit/949fedbacd0f7ea5bc52dd709dfddf54843b1100))
* **homebrew:** update formula to v1.0.0 ([1f11ec4](https://github.com/vinayakkulkarni/tileserver-rs/commit/1f11ec498c67d85af5b166ccc8c90e438b6dcdc0))
* **homebrew:** update formula to v1.0.0 ([912fb18](https://github.com/vinayakkulkarni/tileserver-rs/commit/912fb18c60e9d2bbd3cdc4439c32662f35df53c4))

## [1.0.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/tileserver-rs-v0.2.1...tileserver-rs-v1.0.0) (2025-12-31)


### Features

* add HTTP request logging and OpenAPI 3.1 spec ([f5efd37](https://github.com/vinayakkulkarni/tileserver-rs/commit/f5efd370102c5a47599191abf0e61507e17b39c4))
* **api:** add ?key query parameter support for API key passthrough ([b8b0f39](https://github.com/vinayakkulkarni/tileserver-rs/commit/b8b0f39779d7a8202e4850efb8333bef95b5f302))
* **api:** add comprehensive test suite and Swagger UI documentation ([18fa5a4](https://github.com/vinayakkulkarni/tileserver-rs/commit/18fa5a40c650e4ab6f26346985ccbf7187035cd5))
* **benchmarks:** add comprehensive benchmark suite with correct tile coordinates ([a3f421c](https://github.com/vinayakkulkarni/tileserver-rs/commit/a3f421ca68cf7f29c552d9cc34d797bdc24ccd91))
* **benchmarks:** add tileserver-gl PMTiles support and update results ([95c8303](https://github.com/vinayakkulkarni/tileserver-rs/commit/95c8303d903e42465c45b1633e9f27e5a02601ed))
* **benchmarks:** fair Docker-to-Docker ARM64 benchmark comparison ([b4eba14](https://github.com/vinayakkulkarni/tileserver-rs/commit/b4eba14aae17a76435f6996f03afccd53e5152a5))
* **marketing:** add landing page with Nuxt 4 + shadcn-vue ([63920e4](https://github.com/vinayakkulkarni/tileserver-rs/commit/63920e40c39e71be3a580437622fa0694b4d3189))
* **marketing:** add PostgreSQL/PostGIS support to landing page ([ed1187d](https://github.com/vinayakkulkarni/tileserver-rs/commit/ed1187d4849ab91da6e3f6e06fb504e5f1fc21cc))
* **mbtiles:** implement full MBTiles 1.3 support with SQLite ([c2b125e](https://github.com/vinayakkulkarni/tileserver-rs/commit/c2b125e0cc71b2eff567ce052e98310ca4914ee5))
* **postgres:** add PostgreSQL/PostGIS tile source support with performance optimizations ([24a88ae](https://github.com/vinayakkulkarni/tileserver-rs/commit/24a88aeecbaac8bda335f111dcc525dd593fc8e1))
* suppress MapLibre logs, move OpenAPI to /_openapi, simplify UI ([d42118e](https://github.com/vinayakkulkarni/tileserver-rs/commit/d42118e52aebbd6e6fc055f43907d8003b4bbacb))


### Bug Fixes

* **build:** detect MapLibre Native library changes for proper rebuild ([28f8c09](https://github.com/vinayakkulkarni/tileserver-rs/commit/28f8c094c39019c9d3deac23fcf77d09eafb1df0))
* **ci:** disable macOS AMD64 build (Intel runners retired) ([61f6487](https://github.com/vinayakkulkarni/tileserver-rs/commit/61f648775555096b29d1a122d4b16b3c0d0ed91e))
* **ci:** fix prerelease detection for tileserver-rs-v* tags ([06867ee](https://github.com/vinayakkulkarni/tileserver-rs/commit/06867ee8f5fc62fc21a96ce8066b0d6e0b49b6ed))
* **deps:** Bump actions/cache from 4 to 5 ([#469](https://github.com/vinayakkulkarni/tileserver-rs/issues/469)) ([3333f35](https://github.com/vinayakkulkarni/tileserver-rs/commit/3333f3529ea151338b5161a68b31e8fe31b04c8b))
* **deps:** Bump actions/checkout from 4 to 6 ([#467](https://github.com/vinayakkulkarni/tileserver-rs/issues/467)) ([97f0a00](https://github.com/vinayakkulkarni/tileserver-rs/commit/97f0a00aaf172fe64cffd98a7427a9a6d069991b))
* **deps:** Bump actions/github-script from 7 to 8 ([#468](https://github.com/vinayakkulkarni/tileserver-rs/issues/468)) ([c4d3cf2](https://github.com/vinayakkulkarni/tileserver-rs/commit/c4d3cf2caa8d883d7c3b669b7affeb52bc680715))
* **deps:** Bump actions/upload-artifact from 4 to 6 ([#470](https://github.com/vinayakkulkarni/tileserver-rs/issues/470)) ([47e16f6](https://github.com/vinayakkulkarni/tileserver-rs/commit/47e16f602673f0a0d4f536ea4ef5d3f1cca26e51))
* **deps:** Update @tanstack/vue-query requirement ([#472](https://github.com/vinayakkulkarni/tileserver-rs/issues/472)) ([3e3e338](https://github.com/vinayakkulkarni/tileserver-rs/commit/3e3e3387497dd3cdef2a020b782809272f19e090))
* **deps:** Update @tanstack/vue-query requirement ([#476](https://github.com/vinayakkulkarni/tileserver-rs/issues/476)) ([885582e](https://github.com/vinayakkulkarni/tileserver-rs/commit/885582e937002a1421706f80b2a35280615a4167))
* **deps:** Update @tanstack/vue-query requirement ([#483](https://github.com/vinayakkulkarni/tileserver-rs/issues/483)) ([5eb855a](https://github.com/vinayakkulkarni/tileserver-rs/commit/5eb855ac8413bdf2f8d3a85e161ae2c2af9407db))
* **deps:** Update @tanstack/vue-query requirement in /apps/client ([#474](https://github.com/vinayakkulkarni/tileserver-rs/issues/474)) ([144ebd7](https://github.com/vinayakkulkarni/tileserver-rs/commit/144ebd7c6dbb6f2a6d10377469f2c38aedb2bd90))
* **deps:** Update @tanstack/vue-query requirement in /apps/client ([#480](https://github.com/vinayakkulkarni/tileserver-rs/issues/480)) ([107eee7](https://github.com/vinayakkulkarni/tileserver-rs/commit/107eee796938fd6bbc438aa8ce923b01d56f6609))
* **deps:** Update @tanstack/vue-query requirement in /apps/client ([#484](https://github.com/vinayakkulkarni/tileserver-rs/issues/484)) ([d12816c](https://github.com/vinayakkulkarni/tileserver-rs/commit/d12816ccdaf41c1836dd44b201bc591fc820f492))
* **docker:** add version tags from release-please format ([7a72e91](https://github.com/vinayakkulkarni/tileserver-rs/commit/7a72e919c3290f21a7e56506aa1ca27a01d9edcc))
* **docker:** add workflow_dispatch for manual builds ([be21765](https://github.com/vinayakkulkarni/tileserver-rs/commit/be217657a6680c4139f136ad4a93e349eddff192))
* **docs:** martin supports PMTiles ([2abaf79](https://github.com/vinayakkulkarni/tileserver-rs/commit/2abaf794fe89e398e13aa2cfa82f08e7eceaded9))
* **docs:** remove duplicate h1 header in benchmarks page ([86f3e72](https://github.com/vinayakkulkarni/tileserver-rs/commit/86f3e728744305efbf8f7601e40908b296ba577b))
* **docs:** remove duplicate h1 headers from all doc pages ([700bcb6](https://github.com/vinayakkulkarni/tileserver-rs/commit/700bcb68ee4d8e97bd17fc884d2ec5a7d5653560))
* make OpenAPI docs work offline without external dependencies ([15b44ea](https://github.com/vinayakkulkarni/tileserver-rs/commit/15b44eadf72a61424f772e7dbab6a29548248dfa))


### Performance Improvements

* **postgres:** add connection pool pre-warming and prepared statement caching ([429aa6e](https://github.com/vinayakkulkarni/tileserver-rs/commit/429aa6e8d7f3ada57ff0bcf8dfe282b657948859))


### Documentation

* add crates.io publishing plan ([c961745](https://github.com/vinayakkulkarni/tileserver-rs/commit/c9617456e66959250a457a1d5aa1c801bacc8307))
* add quickstart guide, static images guide, and MapLibre integration ([e8e426f](https://github.com/vinayakkulkarni/tileserver-rs/commit/e8e426fcd3d3d2391a2adb0aff5729abe3293bdd))
* add vector tiles guide and Docker deployment guide ([adb9a14](https://github.com/vinayakkulkarni/tileserver-rs/commit/adb9a1499b1de57007085cb9757e5952c41dc6ab))
* **benchmarks:** update comparison table with actual benchmark results ([e6159a9](https://github.com/vinayakkulkarni/tileserver-rs/commit/e6159a94c11b6d4f9488e98b22c0ea6b85f74ca9))
* **benchmarks:** update PostgreSQL results with isolated benchmark data ([1ac4f31](https://github.com/vinayakkulkarni/tileserver-rs/commit/1ac4f31b0dbfe703053fc4562cb9af8676eee1d8))


### Miscellaneous

* **data:** add zuric mbtiles ([7b95e79](https://github.com/vinayakkulkarni/tileserver-rs/commit/7b95e79634bd2776525257deeaba5c5bb6140fcf))
* **deps-dev:** Update eslint-plugin-better-tailwindcss requirement ([#471](https://github.com/vinayakkulkarni/tileserver-rs/issues/471)) ([26b03e2](https://github.com/vinayakkulkarni/tileserver-rs/commit/26b03e286bffc311e592a078d37f3faacce861e1))
* **deps-dev:** Update eslint-plugin-better-tailwindcss requirement ([#473](https://github.com/vinayakkulkarni/tileserver-rs/issues/473)) ([47e4df7](https://github.com/vinayakkulkarni/tileserver-rs/commit/47e4df727009169bb60457d1be9f178dc249947e))
* **deps-dev:** Update eslint-plugin-oxlint requirement ([#475](https://github.com/vinayakkulkarni/tileserver-rs/issues/475)) ([5b743c8](https://github.com/vinayakkulkarni/tileserver-rs/commit/5b743c8b28788d44ef9ef850027b4bcb96a55f96))
* **deps-dev:** Update eslint-plugin-oxlint requirement in /docs ([#479](https://github.com/vinayakkulkarni/tileserver-rs/issues/479)) ([8db60d9](https://github.com/vinayakkulkarni/tileserver-rs/commit/8db60d94d91353ecba6b41b5c5b96c38d43cdedc))
* **deps-dev:** Update oxlint requirement from ^1.35.0 to ^1.36.0 ([#477](https://github.com/vinayakkulkarni/tileserver-rs/issues/477)) ([07fe55c](https://github.com/vinayakkulkarni/tileserver-rs/commit/07fe55c91d8d5c7da730225e28c167928df0fd14))
* **deps-dev:** Update oxlint requirement in /apps/client ([#478](https://github.com/vinayakkulkarni/tileserver-rs/issues/478)) ([69a7b87](https://github.com/vinayakkulkarni/tileserver-rs/commit/69a7b879bb392f271c048ecd70bdc7d7bd892dc0))
* **deps-dev:** Update oxlint requirement in /docs ([#481](https://github.com/vinayakkulkarni/tileserver-rs/issues/481)) ([5761821](https://github.com/vinayakkulkarni/tileserver-rs/commit/5761821a408b85808d0b2288c779349de82f5e9f))
* **homebrew:** update formula to v0.2.1 ([4c3459c](https://github.com/vinayakkulkarni/tileserver-rs/commit/4c3459c1940cd3f3062254f747dfac7dca759290))
* release 1.0.0 ([eee2124](https://github.com/vinayakkulkarni/tileserver-rs/commit/eee21240d4e678568b3d33b52c066210463ff07a))
* remove redundant config.benchmark.toml ([10af658](https://github.com/vinayakkulkarni/tileserver-rs/commit/10af6586db07523f2332d87dbb5d62d5334cd4ab))
* remove TODO.md (all features implemented) ([7767498](https://github.com/vinayakkulkarni/tileserver-rs/commit/77674985ddd3a78c05b58807879b6ec1e65f3e6d))
* update bun.lock ([4d19ee9](https://github.com/vinayakkulkarni/tileserver-rs/commit/4d19ee9ce4e318ace7a59ce2ebc1eec4dbf460b0))

## [0.2.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/tileserver-rs-v0.2.0...tileserver-rs-v0.2.1) (2025-12-28)


### Features

* **ci:** add multi-platform release workflows ([a08edf8](https://github.com/vinayakkulkarni/tileserver-rs/commit/a08edf808e3351eebdc9f422ee28611d4833bfc7))


### Bug Fixes

* **ci:** add actions:write permission to trigger builds ([b8e4a9a](https://github.com/vinayakkulkarni/tileserver-rs/commit/b8e4a9acf16bad83fdf9891685710ebe84777aca))
* **ci:** use macos-15-large for Intel, disable Linux ARM64 for now ([2ebb092](https://github.com/vinayakkulkarni/tileserver-rs/commit/2ebb0929ca3145f4bd6f3d886678d98b14aa39d4))


### Documentation

* add release process documentation to README and CONTRIBUTING ([247ac46](https://github.com/vinayakkulkarni/tileserver-rs/commit/247ac46d3850adba089e25f1cb0945cd0e0ac6b3))


### Code Refactoring

* **ci:** use standard artifact naming and separate homebrew workflow ([4fced3f](https://github.com/vinayakkulkarni/tileserver-rs/commit/4fced3fffabb18ef704e09050eb21938285f29ee))

## [0.2.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/tileserver-rs-v0.1.0...tileserver-rs-v0.2.0) (2025-12-28)


### ⚠ BREAKING CHANGES

* move fonts in `data` directory
* pmmtiles support added
* docs
* init 6 ✨

### Features

* add native MapLibre rendering and local PMTiles support ([ed46a3d](https://github.com/vinayakkulkarni/tileserver-rs/commit/ed46a3d655a8dea80e6fbcc8e8551536f2dd4df8))
* add otel support ([013d451](https://github.com/vinayakkulkarni/tileserver-rs/commit/013d451cd3dc79625bdaf29be20bee62ea51086c))
* **api:** add static file serving, path/marker overlays, and auto-fit ([6b86148](https://github.com/vinayakkulkarni/tileserver-rs/commit/6b861485d8fd2e9f4556a46feb3baa02b71094d1))
* **ci:** add release-please for automated releases ([0bd4b2c](https://github.com/vinayakkulkarni/tileserver-rs/commit/0bd4b2c069eac540981ce038de8eea231744b807))
* **docker:** add multi-arch support (amd64 + arm64) ([1b1c39a](https://github.com/vinayakkulkarni/tileserver-rs/commit/1b1c39a8f23b5ab025f6d91125b4be9f91b890f3))
* **docs:** add analytics ✨ ([fce4861](https://github.com/vinayakkulkarni/tileserver-rs/commit/fce486137c558c0deac584b86889595f0d07cacd))
* initial setup for raster maps ([777d1a6](https://github.com/vinayakkulkarni/tileserver-rs/commit/777d1a6b49b155b7839950712be24714427c9fc2))
* pmmtiles support added ([37425e5](https://github.com/vinayakkulkarni/tileserver-rs/commit/37425e55925318123897c213aa22ddb7c44ade34))
* **release:** add homebrew formula and update release workflow ([f55dfad](https://github.com/vinayakkulkarni/tileserver-rs/commit/f55dfadee2bc56d0ba6bbac9d9fa3fc93db2f80c))
* some more changes done ✅ ([f0f444c](https://github.com/vinayakkulkarni/tileserver-rs/commit/f0f444c5d3d4a413b3c9e0e987fcd0214bffac83))


### Bug Fixes

* **ci:** use CMake presets for MapLibre Native builds ([bdb3920](https://github.com/vinayakkulkarni/tileserver-rs/commit/bdb3920a506a3b147af0cca3213157a8e112c86c))
* **ci:** use HTTPS URL for submodule (required for GitHub Actions) ([2d9c45a](https://github.com/vinayakkulkarni/tileserver-rs/commit/2d9c45aab731903a4bd9005e5d52018f2d4548c2))
* code & workflows clean up ([20f4299](https://github.com/vinayakkulkarni/tileserver-rs/commit/20f4299bf62bdb08bb1344ca4df37dbab9a6b677))
* command ([37779cc](https://github.com/vinayakkulkarni/tileserver-rs/commit/37779ccc9d91c9c2ba38529e96f9923a54815703))
* **deps:** bump @antfu/utils from 0.7.2 to 0.7.4 in /client ([6c593f8](https://github.com/vinayakkulkarni/tileserver-rs/commit/6c593f8e0e98302e46db563278edd770e2b2695a))
* **deps:** bump @antfu/utils from 0.7.2 to 0.7.4 in /client ([33f2341](https://github.com/vinayakkulkarni/tileserver-rs/commit/33f2341e10f41b52c1fd234f8dc1b200a1c1aade))
* **deps:** Bump @tanstack/vue-db from 0.0.91 to 0.0.92 ([39a7ae8](https://github.com/vinayakkulkarni/tileserver-rs/commit/39a7ae85143d4e65c37c16383e1939a9d1617d27))
* **deps:** Bump @tanstack/vue-db from 0.0.91 to 0.0.92 ([ce50e75](https://github.com/vinayakkulkarni/tileserver-rs/commit/ce50e75f2a68dbbbd3c15dfafab0dbe0c134a924))
* **deps:** bump actions/cache from 3.3.1 to 3.3.2 ([f851956](https://github.com/vinayakkulkarni/tileserver-rs/commit/f8519562ef0f6c2a781479bdd28f2648447fcdc2))
* **deps:** bump actions/cache from 3.3.2 to 3.3.3 ([2cb7321](https://github.com/vinayakkulkarni/tileserver-rs/commit/2cb732172745135793f22b313c2cdb542f3fead6))
* **deps:** bump actions/cache from 3.3.3 to 4.0.0 ([9a6dc50](https://github.com/vinayakkulkarni/tileserver-rs/commit/9a6dc50ff13d362637c55ee95361b1b5bc642697))
* **deps:** bump actions/checkout from 3 to 4 ([40cc212](https://github.com/vinayakkulkarni/tileserver-rs/commit/40cc21291fd5ca082fd1dd93ba795c9d7c3292a6))
* **deps:** Bump actions/checkout from 4 to 6 ([#463](https://github.com/vinayakkulkarni/tileserver-rs/issues/463)) ([e8593a2](https://github.com/vinayakkulkarni/tileserver-rs/commit/e8593a24faa6e8f4745239b6c3a2eac16d9eb1fd))
* **deps:** bump actions/setup-node from 3 to 4 ([9ffa2bc](https://github.com/vinayakkulkarni/tileserver-rs/commit/9ffa2bc760b3422b69bf9f1482d4911f7446dd65))
* **deps:** bump actions/upload-artifact from 4 to 6 ([6a82db6](https://github.com/vinayakkulkarni/tileserver-rs/commit/6a82db6a39b89d034fc8f38ef13897b281fcc441))
* **deps:** bump dependabot/fetch-metadata from 1.4.0 to 1.5.0 ([48ede1e](https://github.com/vinayakkulkarni/tileserver-rs/commit/48ede1eceade566f630fd57f410f971193d8ae82))
* **deps:** bump dependabot/fetch-metadata from 1.4.0 to 1.5.0 ([16ad0a9](https://github.com/vinayakkulkarni/tileserver-rs/commit/16ad0a9917c05e7fac99e3d6b428752ab7756daf))
* **deps:** bump dependabot/fetch-metadata from 1.5.0 to 1.5.1 ([a6b9aef](https://github.com/vinayakkulkarni/tileserver-rs/commit/a6b9aef6252ee445e14dd3e76ece7e30e2993d11))
* **deps:** bump dependabot/fetch-metadata from 1.5.0 to 1.5.1 ([de1eea9](https://github.com/vinayakkulkarni/tileserver-rs/commit/de1eea9dbd584c4e5eec14a94af9e950e3b6a65a))
* **deps:** bump dependabot/fetch-metadata from 1.5.1 to 1.6.0 ([2421c92](https://github.com/vinayakkulkarni/tileserver-rs/commit/2421c926879442bd87bceb1a13a5854c8d657c2d))
* **deps:** bump dependabot/fetch-metadata from 1.5.1 to 1.6.0 ([b3bec91](https://github.com/vinayakkulkarni/tileserver-rs/commit/b3bec9134886c317e119558bbbaad534f3745465))
* **deps:** bump dependabot/fetch-metadata from 1.6.0 to 2.0.0 ([210bf8a](https://github.com/vinayakkulkarni/tileserver-rs/commit/210bf8aef4d9180efd69aa153e6be9cd060fdac7))
* **deps:** bump dependabot/fetch-metadata from 2.0.0 to 2.4.0 ([6eab1e2](https://github.com/vinayakkulkarni/tileserver-rs/commit/6eab1e2fa48fdffe155440dce347458f88936c0d))
* **deps:** bump dependabot/fetch-metadata from 2.0.0 to 2.4.0 ([ac2418d](https://github.com/vinayakkulkarni/tileserver-rs/commit/ac2418ded249dbd0987a109757628f2d5677f98a))
* **deps:** bump semver from 5.7.1 to 5.7.2 ([9462d29](https://github.com/vinayakkulkarni/tileserver-rs/commit/9462d29f01bc939226369fa4c48ce63f52504a83))
* **deps:** bump semver from 5.7.1 to 5.7.2 ([005b7a2](https://github.com/vinayakkulkarni/tileserver-rs/commit/005b7a2f90e26a6226a1081758386197d0340684))
* **deps:** bump semver from 5.7.1 to 5.7.2 in /client ([a587b10](https://github.com/vinayakkulkarni/tileserver-rs/commit/a587b10165d6f1945db607582f07d62b77cb8186))
* **deps:** bump semver from 5.7.1 to 5.7.2 in /client ([f2a267f](https://github.com/vinayakkulkarni/tileserver-rs/commit/f2a267fbbfe036031889f00b851d596f4c350aff))
* **docker:** revert to amd64-only (ARM runners need public repo) ([50baf31](https://github.com/vinayakkulkarni/tileserver-rs/commit/50baf31879ab5ce023712428fb7b3374056fc6e1))
* **docker:** use Rust 1.85 for Edition 2024 support (pxfm crate) ([5974985](https://github.com/vinayakkulkarni/tileserver-rs/commit/5974985587ce0a618a44951eb96fce75242bbcf0))
* **docker:** use Rust 1.92 (latest stable) ([e43672f](https://github.com/vinayakkulkarni/tileserver-rs/commit/e43672f7b93ad0877c6cd0a8f5a2b209f0e4e862))
* layer vis in map ([1057844](https://github.com/vinayakkulkarni/tileserver-rs/commit/10578442aac96994645af35ece9083ff70367225))
* lockfile issues ([db4fa13](https://github.com/vinayakkulkarni/tileserver-rs/commit/db4fa13901ac33c51ebe1d0c50a85431408af164))
* resolve clippy warnings in maplibre-native-sys build.rs ([5ca8316](https://github.com/vinayakkulkarni/tileserver-rs/commit/5ca83162e19b8aeb02479e2c5d90f180d7306ea4))
* **security:** add path traversal protection and DoS limits ([cff7457](https://github.com/vinayakkulkarni/tileserver-rs/commit/cff745711f1e597285311ca2e53e77b55468bb96))
* static bundle ([2617dcf](https://github.com/vinayakkulkarni/tileserver-rs/commit/2617dcf7490c2a7f1465a2fbe672a575f8d8ddd8))
* style loading issues ([ea0511e](https://github.com/vinayakkulkarni/tileserver-rs/commit/ea0511e254dce825e78b6d32bedabe99341e59ee))
* tiptap issue https://github.com/nuxt/ui/issues/5710#issuecomment-3675516020 ([707fba9](https://github.com/vinayakkulkarni/tileserver-rs/commit/707fba9c0c5ef6316f9c42d50637641fda93dbcf))


### Documentation

* add CONTRIBUTING.md and setup maplibre-native as submodule ([0334000](https://github.com/vinayakkulkarni/tileserver-rs/commit/033400072e64d06fe0cf098fabf235b15b86c9fe))
* add fonts configuration and new API endpoints ([d8f01b7](https://github.com/vinayakkulkarni/tileserver-rs/commit/d8f01b7d933f4ad82e288fdc3930c3e53fb4cad0))
* add instructions to clear cargo cache after building MapLibre Native ([4f87fbf](https://github.com/vinayakkulkarni/tileserver-rs/commit/4f87fbfaa650c47e2bf2291ee66f4e15b0868edb))
* improve submodule instructions in README and CONTRIBUTING ([3ec4acf](https://github.com/vinayakkulkarni/tileserver-rs/commit/3ec4acf92b25252bebafd813ed80efa817d9ac23))
* **installation:** add homebrew and pre-built binary instructions ([2ea51eb](https://github.com/vinayakkulkarni/tileserver-rs/commit/2ea51ebef1f40211743a14ca2aa05e6c026f57b4))
* update README, CONTRIBUTING, and configuration docs ([d3a53bb](https://github.com/vinayakkulkarni/tileserver-rs/commit/d3a53bbb3f62e48ba541647131c50d0c90ce8009))


### Miscellaneous

* add maplibre-native submodule reference ([507fdb1](https://github.com/vinayakkulkarni/tileserver-rs/commit/507fdb1abebb47ffd0aeab830cedc0bf161cdc46))
* add sample pmtiles ([f6f3e1f](https://github.com/vinayakkulkarni/tileserver-rs/commit/f6f3e1fcae98b393007935bcdce0d689f4dbb1e6))
* bump bun to 1.3.5 ([c0ed0b2](https://github.com/vinayakkulkarni/tileserver-rs/commit/c0ed0b2053237a8a863bf9bc26e71fe214532f53))
* bump dep ([56d2fdc](https://github.com/vinayakkulkarni/tileserver-rs/commit/56d2fdc886a85127199fa1de33a9b6caa8e71862))
* bump deps ([437bde6](https://github.com/vinayakkulkarni/tileserver-rs/commit/437bde6570295c4c3bed3c3dc4fb4dae349ef4bb))
* bump deps ([6d60de6](https://github.com/vinayakkulkarni/tileserver-rs/commit/6d60de6faffd35d17499e657079c509909655f0e))
* **ci:** update workflows and use actions/checkout@v6 ([4df2aab](https://github.com/vinayakkulkarni/tileserver-rs/commit/4df2aab885c6d732a86843dce51c4eab3ede3525))
* dependabot config updated ([32bbcca](https://github.com/vinayakkulkarni/tileserver-rs/commit/32bbccaac4be4ba64e043f0234c9cdbc22e760fc))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.13 to 8.9.15 in /client ([bc0cd23](https://github.com/vinayakkulkarni/tileserver-rs/commit/bc0cd23745b9cd53022eb6f33f54eb4506e15ed6))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.13 to 8.9.15 in /client ([bcd614e](https://github.com/vinayakkulkarni/tileserver-rs/commit/bcd614e0cc62e7c71742f330204292c5fe777812))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.15 to 8.9.16 in /client ([8b6e191](https://github.com/vinayakkulkarni/tileserver-rs/commit/8b6e191b76bae699d954f466b260473e69e5099d))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.15 to 8.9.16 in /client ([ae00e4e](https://github.com/vinayakkulkarni/tileserver-rs/commit/ae00e4ef9ae9b824a63ed1a6b063c5f981bce03b))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.16 to 8.9.17 in /client ([1f16e30](https://github.com/vinayakkulkarni/tileserver-rs/commit/1f16e308ae0cca3eb1592047fc6676250515ee96))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.16 to 8.9.17 in /client ([da17fc1](https://github.com/vinayakkulkarni/tileserver-rs/commit/da17fc1bb0262cd7638c34315e1d8f934de2146a))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.17 to 8.9.18 in /client ([ae2c20b](https://github.com/vinayakkulkarni/tileserver-rs/commit/ae2c20b2a7fc6b6ee298dfa1070ae1c768fb62a7))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.17 to 8.9.18 in /client ([055dfce](https://github.com/vinayakkulkarni/tileserver-rs/commit/055dfce3ec2f39cae6666a7bf430d3e6e0b14e2e))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.18 to 8.9.19 in /client ([239eb0b](https://github.com/vinayakkulkarni/tileserver-rs/commit/239eb0b0049d041ab7b7ed86d73b3aa0a8c6f599))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.18 to 8.9.19 in /client ([f1eb66b](https://github.com/vinayakkulkarni/tileserver-rs/commit/f1eb66b2c33f6889784a86bf965d99772aa12e0b))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.19 to 8.9.20 in /client ([8ea36de](https://github.com/vinayakkulkarni/tileserver-rs/commit/8ea36de81becdb08788746945fc290ead023e998))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.19 to 8.9.20 in /client ([00a3c7f](https://github.com/vinayakkulkarni/tileserver-rs/commit/00a3c7f9537bb6f5678a1f1125d46c30c080941b))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.20 to 8.9.21 in /client ([2db0f98](https://github.com/vinayakkulkarni/tileserver-rs/commit/2db0f988adfc31bcb7fa7214b1fbd74cc863b129))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.20 to 8.9.21 in /client ([6719367](https://github.com/vinayakkulkarni/tileserver-rs/commit/6719367d81675939817025171b8f730d981ef380))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.21 to 8.9.22 in /client ([aa84398](https://github.com/vinayakkulkarni/tileserver-rs/commit/aa8439882d84b49c4a339749b1683ff661c04651))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.21 to 8.9.22 in /client ([16cd2e9](https://github.com/vinayakkulkarni/tileserver-rs/commit/16cd2e9aaaeaa94c036de91bd77c96bf51daafeb))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.22 to 8.9.23 in /client ([3658d36](https://github.com/vinayakkulkarni/tileserver-rs/commit/3658d3665885cd33a7e0ec40dd2e307395803205))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.22 to 8.9.23 in /client ([dafaff0](https://github.com/vinayakkulkarni/tileserver-rs/commit/dafaff0b94d901b4013aeb20df08ccabc22bdfad))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.23 to 8.9.25 in /client ([b98fba3](https://github.com/vinayakkulkarni/tileserver-rs/commit/b98fba3247c1f3b3c744c47fd0d6987f7cfd069f))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.23 to 8.9.25 in /client ([7f885a8](https://github.com/vinayakkulkarni/tileserver-rs/commit/7f885a8c764cac7938cef02517b3d4639a43385a))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.25 to 8.9.26 in /client ([cde9b6f](https://github.com/vinayakkulkarni/tileserver-rs/commit/cde9b6f0b16fdd5c832851f6fc0ec8ba399c1323))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.25 to 8.9.26 in /client ([ca99044](https://github.com/vinayakkulkarni/tileserver-rs/commit/ca9904433b466aa8b56fb01005b47306699955ef))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.26 to 8.9.27 in /client ([8d55b1c](https://github.com/vinayakkulkarni/tileserver-rs/commit/8d55b1c01211fcfe75f0b0bdf9c75d8cd221763f))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.26 to 8.9.27 in /client ([3373e3d](https://github.com/vinayakkulkarni/tileserver-rs/commit/3373e3d0c665f93410d153c213ef343e98b856b8))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.27 to 8.9.28 in /client ([96b95a0](https://github.com/vinayakkulkarni/tileserver-rs/commit/96b95a0abae97262f6d30d967c0d3e20748ccd2d))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.27 to 8.9.28 in /client ([ee61707](https://github.com/vinayakkulkarni/tileserver-rs/commit/ee61707677db6500f400bba5a8a08b0c5d6c7cd1))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.28 to 8.9.29 in /client ([2a9dd55](https://github.com/vinayakkulkarni/tileserver-rs/commit/2a9dd55c8fe07828adf51654eab17aa15751652b))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.28 to 8.9.29 in /client ([496f5ca](https://github.com/vinayakkulkarni/tileserver-rs/commit/496f5cad88a4334164917132cfeb4971fa584a34))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.29 to 8.9.30 in /client ([6347fb2](https://github.com/vinayakkulkarni/tileserver-rs/commit/6347fb212f7f596eaf8706d9a969c16477f01770))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.29 to 8.9.30 in /client ([96149b2](https://github.com/vinayakkulkarni/tileserver-rs/commit/96149b27a86d20ebe4d2e407527b9af437154503))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.30 to 8.9.31 in /client ([c748dfc](https://github.com/vinayakkulkarni/tileserver-rs/commit/c748dfca1600914ff48455fbe761e198235cab4f))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.30 to 8.9.31 in /client ([950a795](https://github.com/vinayakkulkarni/tileserver-rs/commit/950a795bf6790d9ca2c77bee4f3b36ae64c51af0))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.31 to 8.9.32 in /client ([5e18684](https://github.com/vinayakkulkarni/tileserver-rs/commit/5e186846e3d5138cf2bb59eafbf53d6f6b064dbe))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/layers from 8.9.31 to 8.9.32 in /client ([15813a2](https://github.com/vinayakkulkarni/tileserver-rs/commit/15813a213818ddd07492906db8bda2ad5e735a91))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.13 to 8.9.15 in /client ([09ddbdd](https://github.com/vinayakkulkarni/tileserver-rs/commit/09ddbdd5857cb118c7d1ca1c8b201ca123433811))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.13 to 8.9.15 in /client ([8ffa2b3](https://github.com/vinayakkulkarni/tileserver-rs/commit/8ffa2b349020bf0d6103d71de8b3dad7a1c9b44f))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.15 to 8.9.16 in /client ([00fc063](https://github.com/vinayakkulkarni/tileserver-rs/commit/00fc0633f595ab95ec895b671e5b9c87e97c772f))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.15 to 8.9.16 in /client ([d598987](https://github.com/vinayakkulkarni/tileserver-rs/commit/d598987a2dce4e3b38d2e7210f8db2598760449b))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.16 to 8.9.17 in /client ([e3938ec](https://github.com/vinayakkulkarni/tileserver-rs/commit/e3938ec9397c557fdd152d55401e3e1f5abc7546))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.16 to 8.9.17 in /client ([237cc49](https://github.com/vinayakkulkarni/tileserver-rs/commit/237cc492003bec3cfe8cf6a658c6e32c0d215720))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.17 to 8.9.18 in /client ([6830b05](https://github.com/vinayakkulkarni/tileserver-rs/commit/6830b0560b621079f772b30a6c29180975fd0971))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.17 to 8.9.18 in /client ([01437ea](https://github.com/vinayakkulkarni/tileserver-rs/commit/01437eadc766d6eef2d7800321c8a0e2fca217db))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.18 to 8.9.19 in /client ([2b88140](https://github.com/vinayakkulkarni/tileserver-rs/commit/2b88140d9df7feb8b472aa1ec118d8c1ffb714f4))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.18 to 8.9.19 in /client ([8206568](https://github.com/vinayakkulkarni/tileserver-rs/commit/820656805c0a479acea8961a68da20667d5eb77c))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.19 to 8.9.20 in /client ([0596fb1](https://github.com/vinayakkulkarni/tileserver-rs/commit/0596fb140d7472c365726ea656814b5193e58a47))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.19 to 8.9.20 in /client ([55be2cb](https://github.com/vinayakkulkarni/tileserver-rs/commit/55be2cbd30afa733244e930db5e37722c8f2e8ed))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.20 to 8.9.21 in /client ([35b352b](https://github.com/vinayakkulkarni/tileserver-rs/commit/35b352bcc9c4b7c6537208d930e836c3c4594038))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.20 to 8.9.21 in /client ([4fbee8f](https://github.com/vinayakkulkarni/tileserver-rs/commit/4fbee8fcac127620bc178c69712c1a3cb70fb62c))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.21 to 8.9.22 in /client ([06a387d](https://github.com/vinayakkulkarni/tileserver-rs/commit/06a387d261faa501917135650483d46d6659dcb3))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.21 to 8.9.22 in /client ([974e749](https://github.com/vinayakkulkarni/tileserver-rs/commit/974e7491902645a679cc58e4e5d095017ef23d61))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.22 to 8.9.23 in /client ([1d3f366](https://github.com/vinayakkulkarni/tileserver-rs/commit/1d3f3669738dd167c6edb82667bc596ff93990dc))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.22 to 8.9.23 in /client ([95fa952](https://github.com/vinayakkulkarni/tileserver-rs/commit/95fa952d38c43e1ddf44425a2a515cbc92acc0dd))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.23 to 8.9.25 in /client ([08c206e](https://github.com/vinayakkulkarni/tileserver-rs/commit/08c206eeaa043c6ed4ca922f63b9b2912da04c0b))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.23 to 8.9.25 in /client ([f49f8a9](https://github.com/vinayakkulkarni/tileserver-rs/commit/f49f8a98286ab00df10b9f891e5a37ccd3e0ebca))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.25 to 8.9.26 in /client ([40f6092](https://github.com/vinayakkulkarni/tileserver-rs/commit/40f60924fe56f01ba6dda41cf73e65b80833c94a))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.25 to 8.9.26 in /client ([96c2071](https://github.com/vinayakkulkarni/tileserver-rs/commit/96c2071e898679fe04e7ffc7cba0cbd63dcc9278))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.26 to 8.9.27 in /client ([c32ac84](https://github.com/vinayakkulkarni/tileserver-rs/commit/c32ac84b86e1090ad13ab99512a6fa318f776c05))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.26 to 8.9.27 in /client ([eec7615](https://github.com/vinayakkulkarni/tileserver-rs/commit/eec76153389f903c85a0cbff5fa070d671c0e6bb))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.27 to 8.9.28 in /client ([478cdd8](https://github.com/vinayakkulkarni/tileserver-rs/commit/478cdd8c8ef678864743415255d72b3c1b971802))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.27 to 8.9.28 in /client ([12aa821](https://github.com/vinayakkulkarni/tileserver-rs/commit/12aa821c4984da385cadfae65c348bb84bd2a99d))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.28 to 8.9.29 in /client ([c1a2992](https://github.com/vinayakkulkarni/tileserver-rs/commit/c1a299261ee78ebe6df32df76ac5d1a3b2a3055c))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.28 to 8.9.29 in /client ([f99d3fb](https://github.com/vinayakkulkarni/tileserver-rs/commit/f99d3fb18c57a65aee347c1aded16e939e137201))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.29 to 8.9.30 in /client ([9ee5848](https://github.com/vinayakkulkarni/tileserver-rs/commit/9ee5848125e6964fe62a3dfa2be21f998543b1f7))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.29 to 8.9.30 in /client ([466691a](https://github.com/vinayakkulkarni/tileserver-rs/commit/466691a526b739389f66fbeb863758c1472393fe))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.30 to 8.9.31 in /client ([7b4a444](https://github.com/vinayakkulkarni/tileserver-rs/commit/7b4a44455f9261d83dc4fe3d475796713c738bee))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.30 to 8.9.31 in /client ([e0f4fd5](https://github.com/vinayakkulkarni/tileserver-rs/commit/e0f4fd55bffd895cda3907201ceaac69464e227e))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.31 to 8.9.32 in /client ([8bfa133](https://github.com/vinayakkulkarni/tileserver-rs/commit/8bfa13380e3ed1267fb6749468832613ee39814d))
* **deps-dev:** bump [@deck](https://github.com/deck).gl/mapbox from 8.9.31 to 8.9.32 in /client ([ae40bf7](https://github.com/vinayakkulkarni/tileserver-rs/commit/ae40bf725230d2a3ef0484d9626a1c7f81867b96))
* **deps-dev:** bump @babel/traverse from 7.22.11 to 7.23.2 in /client ([fc285b3](https://github.com/vinayakkulkarni/tileserver-rs/commit/fc285b3275e187271b9c4aeef5eb39fac78c76b2))
* **deps-dev:** bump @babel/traverse from 7.22.11 to 7.23.2 in /client ([b52582e](https://github.com/vinayakkulkarni/tileserver-rs/commit/b52582e50f28aa3809eec5ab552a85562c9555a1))
* **deps-dev:** bump @commitlint/cli from 17.6.3 to 17.6.5 ([3a0efa5](https://github.com/vinayakkulkarni/tileserver-rs/commit/3a0efa50617f529e38da2bad974b461ca26631e6))
* **deps-dev:** bump @commitlint/cli from 17.6.3 to 17.6.5 ([bcd3008](https://github.com/vinayakkulkarni/tileserver-rs/commit/bcd3008cc3f2665c856af8aefc34f420c3e1d2be))
* **deps-dev:** bump @commitlint/cli from 17.6.5 to 17.6.6 ([a8ea25c](https://github.com/vinayakkulkarni/tileserver-rs/commit/a8ea25c93fdfbb0eef662a089b457c375a226c2b))
* **deps-dev:** bump @commitlint/cli from 17.6.5 to 17.6.6 ([32347bc](https://github.com/vinayakkulkarni/tileserver-rs/commit/32347bc0a081b39695b407160bc8b5a9267a8848))
* **deps-dev:** bump @commitlint/cli from 17.6.6 to 17.6.7 ([d699f09](https://github.com/vinayakkulkarni/tileserver-rs/commit/d699f09ac00b20257f1fd2abf5049317a88cd892))
* **deps-dev:** bump @commitlint/cli from 17.6.6 to 17.6.7 ([049ab45](https://github.com/vinayakkulkarni/tileserver-rs/commit/049ab452cb546cc7596b41e43c9aead067f1d470))
* **deps-dev:** bump @commitlint/cli from 17.6.7 to 17.7.0 ([9ad54ea](https://github.com/vinayakkulkarni/tileserver-rs/commit/9ad54eaf5ae70ec0d7820615614f7dbd2f04334f))
* **deps-dev:** bump @commitlint/cli from 17.6.7 to 17.7.0 ([7fe1437](https://github.com/vinayakkulkarni/tileserver-rs/commit/7fe1437cec2d6aae4fcff322fd323784e674170b))
* **deps-dev:** bump @commitlint/cli from 17.7.0 to 17.7.1 ([1b90cc8](https://github.com/vinayakkulkarni/tileserver-rs/commit/1b90cc8c123e4c5ebb59bc18ea2b0f96a184f969))
* **deps-dev:** bump @commitlint/cli from 17.7.0 to 17.7.1 ([9a12b89](https://github.com/vinayakkulkarni/tileserver-rs/commit/9a12b89ac08eb178c9101344b75107f400f553bd))
* **deps-dev:** bump @commitlint/cli from 17.7.1 to 17.7.2 ([5e088c1](https://github.com/vinayakkulkarni/tileserver-rs/commit/5e088c1dec8e693a3cb44bfdfe315cdd537d0b0c))
* **deps-dev:** bump @commitlint/cli from 17.7.1 to 17.7.2 ([3eeb541](https://github.com/vinayakkulkarni/tileserver-rs/commit/3eeb5417f793d86d5d7849b39ba67cafbf6f2695))
* **deps-dev:** bump @commitlint/cli from 17.7.2 to 17.8.0 ([ead83c8](https://github.com/vinayakkulkarni/tileserver-rs/commit/ead83c888dfa214106faa7e032c1f9a738c1788a))
* **deps-dev:** bump @commitlint/cli from 17.7.2 to 17.8.0 ([31b838b](https://github.com/vinayakkulkarni/tileserver-rs/commit/31b838b89e59742b3cb15cd6f0cffdf40589156f))
* **deps-dev:** bump @commitlint/cli from 17.8.0 to 18.4.2 ([2fda7a9](https://github.com/vinayakkulkarni/tileserver-rs/commit/2fda7a918d9abb1ff4f35037b81b90489c4483f3))
* **deps-dev:** bump @commitlint/cli from 18.4.2 to 18.4.3 ([958e820](https://github.com/vinayakkulkarni/tileserver-rs/commit/958e820c2de07953e2f3a19ef5cd3b7dd1e19559))
* **deps-dev:** bump @commitlint/cli from 18.4.2 to 18.4.3 ([2500609](https://github.com/vinayakkulkarni/tileserver-rs/commit/25006093b649cd7f92aba3e52972bf6a0fc57d0c))
* **deps-dev:** bump @commitlint/cli from 18.4.3 to 18.6.0 ([e03282c](https://github.com/vinayakkulkarni/tileserver-rs/commit/e03282c3fe4f6da006078eef0e272bd4b2acda98))
* **deps-dev:** bump @commitlint/config-conventional ([6d467ce](https://github.com/vinayakkulkarni/tileserver-rs/commit/6d467ce42b1f13ebde4906940a5bfadcfcc80f60))
* **deps-dev:** bump @commitlint/config-conventional ([e1ad555](https://github.com/vinayakkulkarni/tileserver-rs/commit/e1ad5552a1934a62cea01a27e597a3afb42bf0f2))
* **deps-dev:** bump @commitlint/config-conventional ([12d6fdd](https://github.com/vinayakkulkarni/tileserver-rs/commit/12d6fdd4e72b4214519adbb3fe3b8b55e5470825))
* **deps-dev:** bump @commitlint/config-conventional ([c18a301](https://github.com/vinayakkulkarni/tileserver-rs/commit/c18a3016aa5ad0434e3b5b3f2329ea347003a10b))
* **deps-dev:** bump @commitlint/config-conventional ([d8bab3a](https://github.com/vinayakkulkarni/tileserver-rs/commit/d8bab3a7aaca2b050a466422f1f0b82560c1ac98))
* **deps-dev:** bump @commitlint/config-conventional ([261a86c](https://github.com/vinayakkulkarni/tileserver-rs/commit/261a86c8079320028a7737f00162750c76706a58))
* **deps-dev:** bump @commitlint/config-conventional ([2a66f6e](https://github.com/vinayakkulkarni/tileserver-rs/commit/2a66f6e8eec7d47036a9aa2679dafae945f40b82))
* **deps-dev:** bump @commitlint/config-conventional ([8018587](https://github.com/vinayakkulkarni/tileserver-rs/commit/801858739fe5dad1858ce83636408213942827e9))
* **deps-dev:** bump @commitlint/config-conventional from 17.6.3 to 17.6.5 ([d4e9076](https://github.com/vinayakkulkarni/tileserver-rs/commit/d4e9076852661cd91706767a9ae01c136f39905d))
* **deps-dev:** bump @commitlint/config-conventional from 17.6.5 to 17.6.6 ([ce8e22b](https://github.com/vinayakkulkarni/tileserver-rs/commit/ce8e22b5f29f38476f220eb305808b7fe4f788fb))
* **deps-dev:** bump @commitlint/config-conventional from 17.6.6 to 17.6.7 ([2ec7778](https://github.com/vinayakkulkarni/tileserver-rs/commit/2ec7778ac1b78d21f8966975a5ba0f0a876b11d7))
* **deps-dev:** bump @commitlint/config-conventional from 17.6.7 to 17.7.0 ([f86e339](https://github.com/vinayakkulkarni/tileserver-rs/commit/f86e33951250fb7c91da33e8fe06b6d56ccb391b))
* **deps-dev:** bump @commitlint/config-conventional from 17.7.0 to 17.8.0 ([f7fc730](https://github.com/vinayakkulkarni/tileserver-rs/commit/f7fc73039c7a7aae1821440e89f07a0035ef8d66))
* **deps-dev:** bump @commitlint/config-conventional from 18.4.2 to 18.4.3 ([af12810](https://github.com/vinayakkulkarni/tileserver-rs/commit/af12810f67691f73d0d606ca5c5dc82510771bf5))
* **deps-dev:** bump @nuxtjs/plausible from 0.2.0 to 0.2.1 in /client ([33afe62](https://github.com/vinayakkulkarni/tileserver-rs/commit/33afe62dcc1d9d3699f3fe371658366ec3b1148f))
* **deps-dev:** bump @nuxtjs/plausible from 0.2.1 to 0.2.3 in /client ([a3ad25e](https://github.com/vinayakkulkarni/tileserver-rs/commit/a3ad25e8e8853be91713526443d1f7f981884d40))
* **deps-dev:** bump @types/d3-dsv from 3.0.1 to 3.0.2 in /client ([80b06a1](https://github.com/vinayakkulkarni/tileserver-rs/commit/80b06a1b19ec1e6485d30a11dc5a3f3a6bd8f0f5))
* **deps-dev:** bump @types/d3-dsv from 3.0.1 to 3.0.2 in /client ([0d0776e](https://github.com/vinayakkulkarni/tileserver-rs/commit/0d0776e94895eaf39b53842616b8c109988ec47a))
* **deps-dev:** bump @types/d3-dsv from 3.0.2 to 3.0.4 in /client ([1a1aa42](https://github.com/vinayakkulkarni/tileserver-rs/commit/1a1aa4286dbcf97e27e061efd3de7c097119fd13))
* **deps-dev:** bump @types/d3-dsv from 3.0.2 to 3.0.4 in /client ([977bbc7](https://github.com/vinayakkulkarni/tileserver-rs/commit/977bbc77532ab3d0fe34100d6d1a9c48467d2308))
* **deps-dev:** bump @types/d3-dsv from 3.0.4 to 3.0.5 in /client ([3fe4391](https://github.com/vinayakkulkarni/tileserver-rs/commit/3fe43911abb84aff350a0ffebfb20e50603eac2a))
* **deps-dev:** bump @types/d3-dsv from 3.0.4 to 3.0.5 in /client ([f4608ae](https://github.com/vinayakkulkarni/tileserver-rs/commit/f4608ae9340a99d85c17b7e1278e075b015157d3))
* **deps-dev:** bump @types/d3-dsv from 3.0.5 to 3.0.6 in /client ([4ac1e80](https://github.com/vinayakkulkarni/tileserver-rs/commit/4ac1e8041a917dbeea59f4a9d7faa1bf85513a71))
* **deps-dev:** bump @types/d3-dsv from 3.0.5 to 3.0.6 in /client ([e41caa7](https://github.com/vinayakkulkarni/tileserver-rs/commit/e41caa7c61a5fc6c936b9609e9597ef09b827ca1))
* **deps-dev:** bump @types/d3-dsv from 3.0.6 to 3.0.7 in /client ([fa0a713](https://github.com/vinayakkulkarni/tileserver-rs/commit/fa0a71364bd10077664159af24231679bb2b9057))
* **deps-dev:** bump @types/d3-dsv from 3.0.6 to 3.0.7 in /client ([b22d4ae](https://github.com/vinayakkulkarni/tileserver-rs/commit/b22d4ae54ecaf00291c01662ab2b96c8e2a482af))
* **deps-dev:** bump @types/geojson from 7946.0.10 to 7946.0.11 in /client ([796e0c4](https://github.com/vinayakkulkarni/tileserver-rs/commit/796e0c45dd2c190953134c9eb428f8f4549af768))
* **deps-dev:** bump @types/geojson from 7946.0.11 to 7946.0.12 in /client ([bf1813c](https://github.com/vinayakkulkarni/tileserver-rs/commit/bf1813c245971bbe0826adca8c1d5b9b0269d67b))
* **deps-dev:** bump @types/geojson from 7946.0.12 to 7946.0.13 in /client ([af7f4fd](https://github.com/vinayakkulkarni/tileserver-rs/commit/af7f4fdfe95e316829c39fa346d6ef457c4d7f73))
* **deps-dev:** bump @types/geojson in /client ([597b7d9](https://github.com/vinayakkulkarni/tileserver-rs/commit/597b7d92394cab392045f0e01af5dd3381ed4c93))
* **deps-dev:** bump @types/geojson in /client ([2fff04d](https://github.com/vinayakkulkarni/tileserver-rs/commit/2fff04d22b4d293646b3980bb25d6304b4c4f0c3))
* **deps-dev:** bump @types/geojson in /client ([8e69516](https://github.com/vinayakkulkarni/tileserver-rs/commit/8e69516bf15e16a7f098d5eb2590eaefb53a5bcb))
* **deps-dev:** bump @types/uuid from 9.0.1 to 9.0.2 in /client ([06dd571](https://github.com/vinayakkulkarni/tileserver-rs/commit/06dd571805e159fda6c7e03de3e0543b9241b9d9))
* **deps-dev:** bump @types/uuid from 9.0.1 to 9.0.2 in /client ([8f52649](https://github.com/vinayakkulkarni/tileserver-rs/commit/8f5264996304f45bcdaef20c2cb3f587aaf7edb4))
* **deps-dev:** bump @types/uuid from 9.0.2 to 9.0.3 in /client ([5aca243](https://github.com/vinayakkulkarni/tileserver-rs/commit/5aca2438e882a2f14e2d5e68153deab7806a71be))
* **deps-dev:** bump @types/uuid from 9.0.2 to 9.0.3 in /client ([631b611](https://github.com/vinayakkulkarni/tileserver-rs/commit/631b611f7133733d1bc230405590c60fec51df98))
* **deps-dev:** bump @types/uuid from 9.0.3 to 9.0.4 in /client ([c2f9d95](https://github.com/vinayakkulkarni/tileserver-rs/commit/c2f9d95ae29aa43a7f7943865b781028fa6c478f))
* **deps-dev:** bump @types/uuid from 9.0.3 to 9.0.4 in /client ([f1ecc0f](https://github.com/vinayakkulkarni/tileserver-rs/commit/f1ecc0fd4bb2f4265a3d7492549caea4a8cc79b4))
* **deps-dev:** bump @types/uuid from 9.0.4 to 9.0.5 in /client ([27c9509](https://github.com/vinayakkulkarni/tileserver-rs/commit/27c9509b3f9304cc52748f223f84128ff5b8d7ea))
* **deps-dev:** bump @types/uuid from 9.0.4 to 9.0.5 in /client ([51dd4bb](https://github.com/vinayakkulkarni/tileserver-rs/commit/51dd4bb367c7e5ed01762e0a09ac98f63bc4f9b2))
* **deps-dev:** bump @types/uuid from 9.0.5 to 9.0.6 in /client ([c768f6a](https://github.com/vinayakkulkarni/tileserver-rs/commit/c768f6ad040f921cabf5ac086b9e4a6928776676))
* **deps-dev:** bump @types/uuid from 9.0.5 to 9.0.6 in /client ([683537a](https://github.com/vinayakkulkarni/tileserver-rs/commit/683537add8ca9fbc6d7119b2d03a17302d460c21))
* **deps-dev:** bump @types/uuid from 9.0.6 to 9.0.7 in /client ([3362f01](https://github.com/vinayakkulkarni/tileserver-rs/commit/3362f013216c14672a2fe5c0d4591c7f3cbb9fe0))
* **deps-dev:** bump @types/uuid from 9.0.6 to 9.0.7 in /client ([e0fcdcd](https://github.com/vinayakkulkarni/tileserver-rs/commit/e0fcdcd6c8e30bda827dd7fa087dc113d135d7de))
* **deps-dev:** bump @typescript-eslint/eslint-plugin from 5.59.11 to 5.60.0 in /client ([c52f5bc](https://github.com/vinayakkulkarni/tileserver-rs/commit/c52f5bc7f8cba61842b0a05d18e3adde7c0e28f2))
* **deps-dev:** bump @typescript-eslint/eslint-plugin from 5.59.2 to 5.59.5 in /client ([525fb98](https://github.com/vinayakkulkarni/tileserver-rs/commit/525fb9870d750cf516d8601dac171f2dee41a248))
* **deps-dev:** bump @typescript-eslint/eslint-plugin from 5.59.5 to 5.59.6 in /client ([ab785d0](https://github.com/vinayakkulkarni/tileserver-rs/commit/ab785d0085694b8eb50e7c86213b471a182c0911))
* **deps-dev:** bump @typescript-eslint/eslint-plugin from 5.59.6 to 5.59.7 in /client ([36ecc25](https://github.com/vinayakkulkarni/tileserver-rs/commit/36ecc25b22cfee450a01e522a146e14c3185f567))
* **deps-dev:** bump @typescript-eslint/eslint-plugin from 5.59.7 to 5.59.8 in /client ([1716fd2](https://github.com/vinayakkulkarni/tileserver-rs/commit/1716fd2bdad5f15f326d91dadb02dddd2e47ffd4))
* **deps-dev:** bump @typescript-eslint/eslint-plugin from 5.59.8 to 5.59.9 in /client ([ef3b0e8](https://github.com/vinayakkulkarni/tileserver-rs/commit/ef3b0e89454d1c75c6074295a1e549e48699b067))
* **deps-dev:** bump @typescript-eslint/eslint-plugin from 5.59.9 to 5.59.11 in /client ([ddac856](https://github.com/vinayakkulkarni/tileserver-rs/commit/ddac856a40ed4fa6a92362bb564a771c06eea519))
* **deps-dev:** bump @typescript-eslint/eslint-plugin from 5.60.0 to 5.60.1 in /client ([2f9c30c](https://github.com/vinayakkulkarni/tileserver-rs/commit/2f9c30c9c130f5de88352c145d81319b4fbb7915))
* **deps-dev:** bump @typescript-eslint/eslint-plugin from 5.60.1 to 5.61.0 in /client ([0525ef9](https://github.com/vinayakkulkarni/tileserver-rs/commit/0525ef939637f41d9ffb3945483925f434eef317))
* **deps-dev:** bump @typescript-eslint/eslint-plugin in /client ([287ed49](https://github.com/vinayakkulkarni/tileserver-rs/commit/287ed494568b0e352f26a00c58530285ff9b2d73))
* **deps-dev:** bump @typescript-eslint/eslint-plugin in /client ([43f7fda](https://github.com/vinayakkulkarni/tileserver-rs/commit/43f7fda13a4ed38ca9daf72be9b47444bcdaf0d1))
* **deps-dev:** bump @typescript-eslint/eslint-plugin in /client ([856a071](https://github.com/vinayakkulkarni/tileserver-rs/commit/856a071cc49e26b57b939b25f08a31b20f052de6))
* **deps-dev:** bump @typescript-eslint/eslint-plugin in /client ([b1a02d7](https://github.com/vinayakkulkarni/tileserver-rs/commit/b1a02d73b7b6b80fc2325d696cbf4c3ebd05c48a))
* **deps-dev:** bump @typescript-eslint/eslint-plugin in /client ([7955899](https://github.com/vinayakkulkarni/tileserver-rs/commit/79558994b2f30628846b51c361a5aa70c206d7d9))
* **deps-dev:** bump @typescript-eslint/eslint-plugin in /client ([74534e7](https://github.com/vinayakkulkarni/tileserver-rs/commit/74534e75b11fd413baea012ec5d4b86436f72525))
* **deps-dev:** bump @typescript-eslint/eslint-plugin in /client ([df13c78](https://github.com/vinayakkulkarni/tileserver-rs/commit/df13c780d6c8bba08b78d6b7376e3017d049cf36))
* **deps-dev:** bump @typescript-eslint/eslint-plugin in /client ([ce3cd45](https://github.com/vinayakkulkarni/tileserver-rs/commit/ce3cd454e58cffded7a3792f4023f007154cfa61))
* **deps-dev:** bump @typescript-eslint/eslint-plugin in /client ([5b16940](https://github.com/vinayakkulkarni/tileserver-rs/commit/5b16940e449b3f594f56f5e0a97decad76e0a291))
* **deps-dev:** bump @typescript-eslint/parser from 5.59.11 to 5.60.0 in /client ([4229195](https://github.com/vinayakkulkarni/tileserver-rs/commit/422919504771e7b726b5d06aa9eef0053fafc57e))
* **deps-dev:** bump @typescript-eslint/parser from 5.59.2 to 5.59.5 in /client ([84a0200](https://github.com/vinayakkulkarni/tileserver-rs/commit/84a0200e6ab3bb94d64eb3e3f3c110a96ff35a6d))
* **deps-dev:** bump @typescript-eslint/parser from 5.59.5 to 5.59.6 in /client ([93372df](https://github.com/vinayakkulkarni/tileserver-rs/commit/93372df27ffab593bfd0eabe44ee63fa6b2d66b8))
* **deps-dev:** bump @typescript-eslint/parser from 5.59.6 to 5.59.7 in /client ([9cd6fd4](https://github.com/vinayakkulkarni/tileserver-rs/commit/9cd6fd428244688ff03490a172e46062594758c2))
* **deps-dev:** bump @typescript-eslint/parser from 5.59.7 to 5.59.8 in /client ([5c6a719](https://github.com/vinayakkulkarni/tileserver-rs/commit/5c6a719909008a646fd7d6170ded1079e9cc4abf))
* **deps-dev:** bump @typescript-eslint/parser from 5.59.8 to 5.59.9 in /client ([7e103fe](https://github.com/vinayakkulkarni/tileserver-rs/commit/7e103fe0e5df3c7b498c3fcbfa28ed825d6534fe))
* **deps-dev:** bump @typescript-eslint/parser from 5.59.9 to 5.59.11 in /client ([8c39626](https://github.com/vinayakkulkarni/tileserver-rs/commit/8c39626388d00a7820f8c0bb5608cd2c6fc3e9cf))
* **deps-dev:** bump @typescript-eslint/parser from 5.60.0 to 5.60.1 in /client ([45687b0](https://github.com/vinayakkulkarni/tileserver-rs/commit/45687b0815d52a2a69a6fae56b23bb9f41f6c0d4))
* **deps-dev:** bump @typescript-eslint/parser from 5.60.1 to 5.61.0 in /client ([23dd343](https://github.com/vinayakkulkarni/tileserver-rs/commit/23dd3434bdcd0d6ad451c4f8bcd7433e8b3ee111))
* **deps-dev:** bump @typescript-eslint/parser from 5.61.0 to 5.62.0 in /client ([7fe347a](https://github.com/vinayakkulkarni/tileserver-rs/commit/7fe347afb8661cbfe98c7c01eb9039359abdb124))
* **deps-dev:** bump @typescript-eslint/parser in /client ([3f5ebc4](https://github.com/vinayakkulkarni/tileserver-rs/commit/3f5ebc420516edf38e40f29760c3c473dc72a920))
* **deps-dev:** bump @typescript-eslint/parser in /client ([b176274](https://github.com/vinayakkulkarni/tileserver-rs/commit/b1762745a96bb3657e19785e11c287564da950a8))
* **deps-dev:** bump @typescript-eslint/parser in /client ([0e98aa5](https://github.com/vinayakkulkarni/tileserver-rs/commit/0e98aa5280181911f26388bd2f74c93df07e5105))
* **deps-dev:** bump @typescript-eslint/parser in /client ([cbdf405](https://github.com/vinayakkulkarni/tileserver-rs/commit/cbdf40587edbf9eb1a039e0c3489a7ec6db0a166))
* **deps-dev:** bump @typescript-eslint/parser in /client ([6014154](https://github.com/vinayakkulkarni/tileserver-rs/commit/601415494406f96c19ae64c2c6217ea0fb2bf818))
* **deps-dev:** bump @typescript-eslint/parser in /client ([8ec6528](https://github.com/vinayakkulkarni/tileserver-rs/commit/8ec65282dd265454149d68f0ab6e2a08b581ceb8))
* **deps-dev:** bump @typescript-eslint/parser in /client ([2af4e1a](https://github.com/vinayakkulkarni/tileserver-rs/commit/2af4e1abb66a54f4e8a587a0ceb0f800e1d70ed4))
* **deps-dev:** bump @typescript-eslint/parser in /client ([74d5548](https://github.com/vinayakkulkarni/tileserver-rs/commit/74d554815979eb34ddf729361ad996b2162a9467))
* **deps-dev:** bump @typescript-eslint/parser in /client ([eaadefd](https://github.com/vinayakkulkarni/tileserver-rs/commit/eaadefd73a6015e35501bdfe856a03342f98223b))
* **deps-dev:** bump @typescript-eslint/parser in /client ([00e0b6e](https://github.com/vinayakkulkarni/tileserver-rs/commit/00e0b6eabaa14497442baf5b7de938e7b58fa8d6))
* **deps-dev:** bump @vueuse/core from 10.3.0 to 10.4.0 in /client ([09b1dda](https://github.com/vinayakkulkarni/tileserver-rs/commit/09b1dda1588eef882c6a7ae0f25c373cb907ee79))
* **deps-dev:** bump @vueuse/core from 10.3.0 to 10.4.0 in /client ([9b5745f](https://github.com/vinayakkulkarni/tileserver-rs/commit/9b5745f161887e494673b491a73368f8da0882ca))
* **deps-dev:** bump @vueuse/core from 10.4.1 to 10.5.0 in /client ([4d38673](https://github.com/vinayakkulkarni/tileserver-rs/commit/4d38673263d414f680d12d88cac3d9828e5cf398))
* **deps-dev:** bump @vueuse/core from 10.4.1 to 10.5.0 in /client ([f9a6398](https://github.com/vinayakkulkarni/tileserver-rs/commit/f9a6398a1e8a820eff2f0574e75f128fbcee97d0))
* **deps-dev:** bump @vueuse/core from 10.6.1 to 10.7.2 in /client ([993eac9](https://github.com/vinayakkulkarni/tileserver-rs/commit/993eac9416bef9e58fb446ad353bba45a195d211))
* **deps-dev:** bump @vueuse/nuxt from 10.1.2 to 10.2.0 in /client ([f50ef61](https://github.com/vinayakkulkarni/tileserver-rs/commit/f50ef6138c2fea678dc460e793703b342d8b1983))
* **deps-dev:** bump @vueuse/nuxt from 10.1.2 to 10.2.0 in /client ([723443a](https://github.com/vinayakkulkarni/tileserver-rs/commit/723443aaff4513f720b890806822c9f1476c63c4))
* **deps-dev:** bump @vueuse/nuxt from 10.2.0 to 10.2.1 in /client ([c597d30](https://github.com/vinayakkulkarni/tileserver-rs/commit/c597d30a67630c31d1bd9061faef2d66289194b0))
* **deps-dev:** bump @vueuse/nuxt from 10.2.0 to 10.2.1 in /client ([07d6f07](https://github.com/vinayakkulkarni/tileserver-rs/commit/07d6f0787c7a3ef1511521dd72b2c1daaf59c2a9))
* **deps-dev:** bump @vueuse/nuxt from 10.2.1 to 10.3.0 in /client ([0b0aea2](https://github.com/vinayakkulkarni/tileserver-rs/commit/0b0aea27490325eea624e57baca8492b7c619f15))
* **deps-dev:** bump @vueuse/nuxt from 10.2.1 to 10.3.0 in /client ([de8db30](https://github.com/vinayakkulkarni/tileserver-rs/commit/de8db30173e9c66000eb3312428cba0a8ad46995))
* **deps-dev:** bump @vueuse/nuxt from 10.3.0 to 10.4.1 in /client ([dfeab6c](https://github.com/vinayakkulkarni/tileserver-rs/commit/dfeab6ccc8e6690263a1612a7f192807c961911d))
* **deps-dev:** bump @vueuse/nuxt from 10.3.0 to 10.4.1 in /client ([7cb3900](https://github.com/vinayakkulkarni/tileserver-rs/commit/7cb3900bc74cd54d2339ee348c630d85f87ff392))
* **deps-dev:** bump @vueuse/nuxt from 10.4.1 to 10.5.0 in /client ([272a249](https://github.com/vinayakkulkarni/tileserver-rs/commit/272a249f07ce55b112c2b761f167ab28922421f4))
* **deps-dev:** bump @vueuse/nuxt from 10.4.1 to 10.5.0 in /client ([30d9a22](https://github.com/vinayakkulkarni/tileserver-rs/commit/30d9a22caf2e3bd41ef545b54804d76108256e9d))
* **deps-dev:** bump @vueuse/nuxt from 10.5.0 to 10.6.0 in /client ([9f1a678](https://github.com/vinayakkulkarni/tileserver-rs/commit/9f1a678a243b6ed7ec8dac3f9be42d31b24e39ac))
* **deps-dev:** bump @vueuse/nuxt from 10.5.0 to 10.6.0 in /client ([14a8754](https://github.com/vinayakkulkarni/tileserver-rs/commit/14a87549f7753e645c9360d3d1229cbd495d06fb))
* **deps-dev:** bump @vueuse/nuxt from 10.6.0 to 10.6.1 in /client ([2e5e103](https://github.com/vinayakkulkarni/tileserver-rs/commit/2e5e103d3ba9b253781d51b0923c58378f86c8c3))
* **deps-dev:** bump @vueuse/nuxt from 10.6.0 to 10.6.1 in /client ([24baada](https://github.com/vinayakkulkarni/tileserver-rs/commit/24baada09afa91552016f68a8d5f3157b35a2b54))
* **deps-dev:** bump eslint from 8.40.0 to 8.41.0 in /client ([6f656be](https://github.com/vinayakkulkarni/tileserver-rs/commit/6f656be03e74cc4139a751b8785a00e12acf9563))
* **deps-dev:** bump eslint from 8.40.0 to 8.41.0 in /client ([8343e5b](https://github.com/vinayakkulkarni/tileserver-rs/commit/8343e5b2606eb2a39efe9922823963a60a8c57cc))
* **deps-dev:** bump eslint from 8.41.0 to 8.42.0 in /client ([32901a8](https://github.com/vinayakkulkarni/tileserver-rs/commit/32901a847c69f6bd116195b6b280b39d854c7f27))
* **deps-dev:** bump eslint from 8.41.0 to 8.42.0 in /client ([ecedaf3](https://github.com/vinayakkulkarni/tileserver-rs/commit/ecedaf36c0e7533070c8aec1e7e599848e287f49))
* **deps-dev:** bump eslint from 8.42.0 to 8.43.0 in /client ([9ba7ebb](https://github.com/vinayakkulkarni/tileserver-rs/commit/9ba7ebb1c5e9df1942bf6c35a3f27379269278e9))
* **deps-dev:** bump eslint from 8.42.0 to 8.43.0 in /client ([8cf811f](https://github.com/vinayakkulkarni/tileserver-rs/commit/8cf811f76899dda57de80082eacbfdfff5a464f4))
* **deps-dev:** bump eslint from 8.43.0 to 8.44.0 in /client ([b2c4b4e](https://github.com/vinayakkulkarni/tileserver-rs/commit/b2c4b4e5be8f44093680e2bdd9bf3a5885f84051))
* **deps-dev:** bump eslint from 8.43.0 to 8.44.0 in /client ([cbffe52](https://github.com/vinayakkulkarni/tileserver-rs/commit/cbffe52ba3a45b9ed5f8f2078193960199589e6b))
* **deps-dev:** bump eslint from 8.44.0 to 8.45.0 in /client ([a7b58d1](https://github.com/vinayakkulkarni/tileserver-rs/commit/a7b58d112f87a851095a42906285a837e96f3afe))
* **deps-dev:** bump eslint from 8.44.0 to 8.45.0 in /client ([dca371f](https://github.com/vinayakkulkarni/tileserver-rs/commit/dca371f9ebd7d77b265ed829210a737936ca6a4f))
* **deps-dev:** bump eslint from 8.45.0 to 8.46.0 in /client ([e77ac45](https://github.com/vinayakkulkarni/tileserver-rs/commit/e77ac459d543a6204ba3b829ef499278ce5167bd))
* **deps-dev:** bump eslint from 8.45.0 to 8.46.0 in /client ([5c02709](https://github.com/vinayakkulkarni/tileserver-rs/commit/5c02709be42177326443d4fd0992d0f0245a1a5c))
* **deps-dev:** bump eslint from 8.46.0 to 8.47.0 in /client ([75b2534](https://github.com/vinayakkulkarni/tileserver-rs/commit/75b2534a221d3d1e2cc76ae7dd89ccfb2d4af968))
* **deps-dev:** bump eslint from 8.46.0 to 8.47.0 in /client ([85e2757](https://github.com/vinayakkulkarni/tileserver-rs/commit/85e275746e76c0561d8a915d34dc22ccafaa351f))
* **deps-dev:** bump eslint from 8.47.0 to 8.48.0 in /client ([1d53567](https://github.com/vinayakkulkarni/tileserver-rs/commit/1d53567963b496c5759ce7ae7c18eba0ef3d291c))
* **deps-dev:** bump eslint from 8.47.0 to 8.48.0 in /client ([f0e65e5](https://github.com/vinayakkulkarni/tileserver-rs/commit/f0e65e570b906b02204b1e1c59f52a1219b3fa4a))
* **deps-dev:** bump eslint from 8.48.0 to 8.49.0 in /client ([c1776fa](https://github.com/vinayakkulkarni/tileserver-rs/commit/c1776fa8c29dfa1504487da8ec5fc8feb2e8fee6))
* **deps-dev:** bump eslint from 8.48.0 to 8.49.0 in /client ([0887932](https://github.com/vinayakkulkarni/tileserver-rs/commit/0887932e1ddc23f32fdd49fae2dc8b8fd8482e8d))
* **deps-dev:** bump eslint from 8.49.0 to 8.50.0 in /client ([87ceaa8](https://github.com/vinayakkulkarni/tileserver-rs/commit/87ceaa83c7951b067b5773921f737ffdee1c3b3d))
* **deps-dev:** bump eslint from 8.49.0 to 8.50.0 in /client ([2cf0c60](https://github.com/vinayakkulkarni/tileserver-rs/commit/2cf0c600d71065ae427c0a7f68a89624a4a5a290))
* **deps-dev:** bump eslint from 8.50.0 to 8.51.0 in /client ([65c7ab9](https://github.com/vinayakkulkarni/tileserver-rs/commit/65c7ab9b1f4822d677ff5474dafabc2c3a98f62d))
* **deps-dev:** bump eslint from 8.50.0 to 8.51.0 in /client ([d05726e](https://github.com/vinayakkulkarni/tileserver-rs/commit/d05726e24a5eff2da3ea79dae9eb51e19835896f))
* **deps-dev:** bump eslint from 8.51.0 to 8.52.0 in /client ([b95206b](https://github.com/vinayakkulkarni/tileserver-rs/commit/b95206bb1a96095ad8d9d08ae4a58e8e531fbec7))
* **deps-dev:** bump eslint from 8.51.0 to 8.52.0 in /client ([c635b82](https://github.com/vinayakkulkarni/tileserver-rs/commit/c635b8298abe3ba1ca6f8dccf10bc793d7588c3c))
* **deps-dev:** bump eslint from 8.52.0 to 8.53.0 in /client ([941fd8d](https://github.com/vinayakkulkarni/tileserver-rs/commit/941fd8d8cdf5b5b72026761dee70c35e92ca060b))
* **deps-dev:** bump eslint from 8.52.0 to 8.53.0 in /client ([d85b2e1](https://github.com/vinayakkulkarni/tileserver-rs/commit/d85b2e11525ffb07cf10288648c3d5b5ef5714ca))
* **deps-dev:** bump eslint from 8.53.0 to 8.54.0 in /client ([1d20451](https://github.com/vinayakkulkarni/tileserver-rs/commit/1d20451267f5ea8400a416fbd0ad2a9a9866d5d1))
* **deps-dev:** bump eslint from 8.53.0 to 8.54.0 in /client ([d54eda8](https://github.com/vinayakkulkarni/tileserver-rs/commit/d54eda891993ead21cf09fc266bb73de285c64d3))
* **deps-dev:** bump eslint from 8.54.0 to 8.56.0 in /client ([9557044](https://github.com/vinayakkulkarni/tileserver-rs/commit/9557044780a959790703addef56e9f33836612ac))
* **deps-dev:** bump eslint-config-prettier from 8.8.0 to 8.9.0 in /client ([e0c3acd](https://github.com/vinayakkulkarni/tileserver-rs/commit/e0c3acd36a209c491a727bc77db36939143a9dc4))
* **deps-dev:** bump eslint-config-prettier from 8.9.0 to 8.10.0 in /client ([ca858ef](https://github.com/vinayakkulkarni/tileserver-rs/commit/ca858ef444434235af8824269ac94f243e126260))
* **deps-dev:** bump eslint-config-prettier in /client ([46b5d16](https://github.com/vinayakkulkarni/tileserver-rs/commit/46b5d164a040a729333c5557845c67951680d44c))
* **deps-dev:** bump eslint-config-prettier in /client ([082b7d2](https://github.com/vinayakkulkarni/tileserver-rs/commit/082b7d2955b18b5bf5c49e319be5d578e741da0e))
* **deps-dev:** bump eslint-config-prettier in /client ([5ffca50](https://github.com/vinayakkulkarni/tileserver-rs/commit/5ffca5035cdccc5a48a52fb51122995afdc119cc))
* **deps-dev:** bump eslint-plugin-jsdoc from 44.0.1 to 44.2.3 in /client ([d5aea3d](https://github.com/vinayakkulkarni/tileserver-rs/commit/d5aea3db91a3c6b17490d38a196d86ce182571d8))
* **deps-dev:** bump eslint-plugin-jsdoc from 44.2.3 to 44.2.4 in /client ([6227239](https://github.com/vinayakkulkarni/tileserver-rs/commit/62272390e138f1eeaaf03c120e03743acb08f97e))
* **deps-dev:** bump eslint-plugin-jsdoc from 44.2.4 to 44.2.5 in /client ([4d7f127](https://github.com/vinayakkulkarni/tileserver-rs/commit/4d7f127ea825cff193a1ce971e3954fd48d06db6))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.1.0 to 46.2.4 in /client ([e68e715](https://github.com/vinayakkulkarni/tileserver-rs/commit/e68e715fa0c1ffed8598a75de7eb9b599651ef4c))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.2.4 to 46.2.5 in /client ([ed4393d](https://github.com/vinayakkulkarni/tileserver-rs/commit/ed4393dc30285d25e566a2bd9df637c52f594742))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.2.5 to 46.2.6 in /client ([0c1046d](https://github.com/vinayakkulkarni/tileserver-rs/commit/0c1046d7aecd581d7765aadf7febadaebbcae1e2))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.2.6 to 46.4.0 in /client ([1686542](https://github.com/vinayakkulkarni/tileserver-rs/commit/1686542ed0628fd54983e9356b2ab234445d7d9f))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.4.0 to 46.4.2 in /client ([ce6b118](https://github.com/vinayakkulkarni/tileserver-rs/commit/ce6b11866b8181f0525f9cf2a6a3e0dd81d940eb))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.4.2 to 46.4.3 in /client ([fc354f0](https://github.com/vinayakkulkarni/tileserver-rs/commit/fc354f058d7e4aabd704866912475be221b2ee3b))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.4.3 to 46.4.4 in /client ([be68c21](https://github.com/vinayakkulkarni/tileserver-rs/commit/be68c21d62087a1ebd1579a0a6e6d93478da7d52))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.4.4 to 46.4.5 in /client ([c917b13](https://github.com/vinayakkulkarni/tileserver-rs/commit/c917b13a6d2fe0544d907092c5a7b93748d1afc8))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.4.5 to 46.4.6 in /client ([9cf7109](https://github.com/vinayakkulkarni/tileserver-rs/commit/9cf7109c9e274d42b71b8212fe12b18cb22990d4))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.4.6 to 46.5.0 in /client ([9e4d278](https://github.com/vinayakkulkarni/tileserver-rs/commit/9e4d2780901093c54426d6e78c0f3c29003285af))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.5.0 to 46.5.1 in /client ([63e3020](https://github.com/vinayakkulkarni/tileserver-rs/commit/63e3020a89d67f5fec836621a9368938aa95d5d6))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.5.1 to 46.6.0 in /client ([5a47669](https://github.com/vinayakkulkarni/tileserver-rs/commit/5a47669a0916dad8d28ab54b1bd61e1e4fcb0980))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.6.0 to 46.7.0 in /client ([82632e3](https://github.com/vinayakkulkarni/tileserver-rs/commit/82632e3a1d908a3fa31bd3810311bfb44680acf3))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.7.0 to 46.8.0 in /client ([8f95387](https://github.com/vinayakkulkarni/tileserver-rs/commit/8f953879e4448cbde4ad0458afd337a1e481a88c))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.8.0 to 46.8.1 in /client ([87abf40](https://github.com/vinayakkulkarni/tileserver-rs/commit/87abf40090b786eda97b11136945be5329f00a04))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.8.1 to 46.8.2 in /client ([6569716](https://github.com/vinayakkulkarni/tileserver-rs/commit/6569716760a35c4c9fa167f134f554a9a6dd0d6a))
* **deps-dev:** bump eslint-plugin-jsdoc from 46.8.2 to 46.9.0 in /client ([52891d0](https://github.com/vinayakkulkarni/tileserver-rs/commit/52891d07c27c10fe80c8adbdf7b2990e2066de5b))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([210f148](https://github.com/vinayakkulkarni/tileserver-rs/commit/210f148faeabed74276a63ed08b971d8e78f1adb))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([0c68109](https://github.com/vinayakkulkarni/tileserver-rs/commit/0c6810987d2b71d48d240636d269f3b4d7f06f75))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([47758d4](https://github.com/vinayakkulkarni/tileserver-rs/commit/47758d4ede45067ed2c214d4fe18dfcaed03dcb4))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([a43dda4](https://github.com/vinayakkulkarni/tileserver-rs/commit/a43dda47c61359a78f573e84544720ad167af6c3))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([163d04f](https://github.com/vinayakkulkarni/tileserver-rs/commit/163d04f12b1b0614382e6a34ad3c1ee0fc8db8b3))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([51aebdd](https://github.com/vinayakkulkarni/tileserver-rs/commit/51aebddb160eda45a14eb84c56450befaff1430c))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([f1d08e0](https://github.com/vinayakkulkarni/tileserver-rs/commit/f1d08e038624b531eb8c68a3ceb15319c94b9325))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([2affe28](https://github.com/vinayakkulkarni/tileserver-rs/commit/2affe28a88873b60d84d3a3058f3910e6f745047))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([c992263](https://github.com/vinayakkulkarni/tileserver-rs/commit/c992263f27546db83c2ee2fe511900979a6004a8))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([574c332](https://github.com/vinayakkulkarni/tileserver-rs/commit/574c33299f647bd377fcba469286554a2671fdeb))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([e881ae9](https://github.com/vinayakkulkarni/tileserver-rs/commit/e881ae97c40dc8c777e85ca0ec0093cfc1e36e26))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([b24ada1](https://github.com/vinayakkulkarni/tileserver-rs/commit/b24ada106a7062139136511cdb7ffe099c1cdc98))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([a2da8bc](https://github.com/vinayakkulkarni/tileserver-rs/commit/a2da8bc0a2393a005fed63f034290e1f93510a4b))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([32b7587](https://github.com/vinayakkulkarni/tileserver-rs/commit/32b758759c79c04591682a4ac79ddf89d3635c03))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([704df89](https://github.com/vinayakkulkarni/tileserver-rs/commit/704df89852d2f87faa8f411e0253a03727893ea8))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([35d79e4](https://github.com/vinayakkulkarni/tileserver-rs/commit/35d79e486f48afcc593cff08ec24a60ff985d7d9))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([d4fb402](https://github.com/vinayakkulkarni/tileserver-rs/commit/d4fb40264127b3266c9e3360f2561800cae8e282))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([fb56b40](https://github.com/vinayakkulkarni/tileserver-rs/commit/fb56b40367415d313b524716e75bb37572898279))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([d2bfc80](https://github.com/vinayakkulkarni/tileserver-rs/commit/d2bfc8006fb6af065ec81114d8f44b33846ff4b9))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([65e4165](https://github.com/vinayakkulkarni/tileserver-rs/commit/65e4165fd230ed04078f8dca488aeed0bae03edc))
* **deps-dev:** bump eslint-plugin-jsdoc in /client ([7d0389f](https://github.com/vinayakkulkarni/tileserver-rs/commit/7d0389fe05cb756a93c06f06fdb4b7d1a72f3a5d))
* **deps-dev:** bump eslint-plugin-vue from 9.11.1 to 9.12.0 in /client ([5500d89](https://github.com/vinayakkulkarni/tileserver-rs/commit/5500d8974c172fa9e14c1f2589557001f1fe16de))
* **deps-dev:** bump eslint-plugin-vue from 9.11.1 to 9.12.0 in /client ([ef6dbe5](https://github.com/vinayakkulkarni/tileserver-rs/commit/ef6dbe533980b7a54a78dd91ae00ea10cd51ac12))
* **deps-dev:** bump eslint-plugin-vue from 9.12.0 to 9.13.0 in /client ([43e3263](https://github.com/vinayakkulkarni/tileserver-rs/commit/43e32638617c33d916e09cf123fc0c95dd3497c6))
* **deps-dev:** bump eslint-plugin-vue from 9.12.0 to 9.13.0 in /client ([5054e01](https://github.com/vinayakkulkarni/tileserver-rs/commit/5054e014816a5b83504cc9a0887114c09d5ff64e))
* **deps-dev:** bump eslint-plugin-vue from 9.13.0 to 9.14.0 in /client ([6214d8a](https://github.com/vinayakkulkarni/tileserver-rs/commit/6214d8ae4020991bd46793c542306fd8cd4206a6))
* **deps-dev:** bump eslint-plugin-vue from 9.13.0 to 9.14.0 in /client ([1ce9d16](https://github.com/vinayakkulkarni/tileserver-rs/commit/1ce9d166cdef0016faa83457af24ee03440f49cc))
* **deps-dev:** bump eslint-plugin-vue from 9.14.0 to 9.14.1 in /client ([e342d18](https://github.com/vinayakkulkarni/tileserver-rs/commit/e342d183a84d19c44a532be60d236e6733f0d7b1))
* **deps-dev:** bump eslint-plugin-vue from 9.14.0 to 9.14.1 in /client ([7df6a88](https://github.com/vinayakkulkarni/tileserver-rs/commit/7df6a886f5bcedc8ea83389384ad28c01c72aa25))
* **deps-dev:** bump eslint-plugin-vue from 9.14.1 to 9.15.0 in /client ([ede638c](https://github.com/vinayakkulkarni/tileserver-rs/commit/ede638c0ed41ecf261f697b87ae636e674343892))
* **deps-dev:** bump eslint-plugin-vue from 9.14.1 to 9.15.0 in /client ([7b879f2](https://github.com/vinayakkulkarni/tileserver-rs/commit/7b879f23370dff2903a636bea9596dcdda1ac2d1))
* **deps-dev:** bump eslint-plugin-vue from 9.15.0 to 9.15.1 in /client ([2fcc5e6](https://github.com/vinayakkulkarni/tileserver-rs/commit/2fcc5e604fbe77eb714cfcd3f61d994a7cfd291b))
* **deps-dev:** bump eslint-plugin-vue from 9.15.0 to 9.15.1 in /client ([0f7c49f](https://github.com/vinayakkulkarni/tileserver-rs/commit/0f7c49f184aa02c23dbe5ed22d6181640dfdb0e5))
* **deps-dev:** bump eslint-plugin-vue from 9.15.1 to 9.16.1 in /client ([d26ca48](https://github.com/vinayakkulkarni/tileserver-rs/commit/d26ca484dc32e2a9d759571ef56b982322231c39))
* **deps-dev:** bump eslint-plugin-vue from 9.15.1 to 9.16.1 in /client ([19f3aa4](https://github.com/vinayakkulkarni/tileserver-rs/commit/19f3aa4c622dcc579bfb4fc2bdc3f0ef1a7f8585))
* **deps-dev:** bump eslint-plugin-vue from 9.16.1 to 9.17.0 in /client ([99c9da8](https://github.com/vinayakkulkarni/tileserver-rs/commit/99c9da82a3190eec1fec561d34628f690dfb2801))
* **deps-dev:** bump eslint-plugin-vue from 9.16.1 to 9.17.0 in /client ([a2ddeae](https://github.com/vinayakkulkarni/tileserver-rs/commit/a2ddeae821149a54f6c1e704192debebcbc3c2eb))
* **deps-dev:** bump eslint-plugin-vue from 9.17.0 to 9.18.0 in /client ([8888719](https://github.com/vinayakkulkarni/tileserver-rs/commit/8888719289f1be1456a269e62d733b5a6da802cc))
* **deps-dev:** bump eslint-plugin-vue from 9.17.0 to 9.18.0 in /client ([3cb8497](https://github.com/vinayakkulkarni/tileserver-rs/commit/3cb84976aca7ea7052b3b2cb24ecf6ab6b62fd9c))
* **deps-dev:** bump eslint-plugin-vue from 9.18.0 to 9.18.1 in /client ([039687a](https://github.com/vinayakkulkarni/tileserver-rs/commit/039687af51a6324abf94cd54d7376f52bb5dde9c))
* **deps-dev:** bump eslint-plugin-vue from 9.18.0 to 9.18.1 in /client ([47030ec](https://github.com/vinayakkulkarni/tileserver-rs/commit/47030ec4197194f53f87d4c3536f3f322cb17036))
* **deps-dev:** bump eslint-plugin-vue from 9.18.1 to 9.19.2 in /client ([2334a3d](https://github.com/vinayakkulkarni/tileserver-rs/commit/2334a3dbaa57af2507100048e6d5633b028af2e8))
* **deps-dev:** bump eslint-plugin-vue from 9.18.1 to 9.19.2 in /client ([a11c621](https://github.com/vinayakkulkarni/tileserver-rs/commit/a11c621cc44aeed36cf05b02ff38a0eb9d3bb289))
* **deps-dev:** bump follow-redirects from 1.15.2 to 1.15.4 in /client ([1a52961](https://github.com/vinayakkulkarni/tileserver-rs/commit/1a529614202cc030f88fd688959087dbf84d06be))
* **deps-dev:** bump follow-redirects from 1.15.4 to 1.15.6 in /client ([a7b22e2](https://github.com/vinayakkulkarni/tileserver-rs/commit/a7b22e28f2032bfb83c5c4e3f5582d6b322f32f9))
* **deps-dev:** bump lint-staged from 13.2.2 to 13.2.3 ([ee4f005](https://github.com/vinayakkulkarni/tileserver-rs/commit/ee4f0056de77bf81147dc0ac59d384e92865c772))
* **deps-dev:** bump lint-staged from 13.2.2 to 13.2.3 ([cd39a2f](https://github.com/vinayakkulkarni/tileserver-rs/commit/cd39a2fcfbe3a5dc8d8dd98938a99ce3c48c9e21))
* **deps-dev:** bump lint-staged from 13.2.3 to 14.0.1 ([b29aea7](https://github.com/vinayakkulkarni/tileserver-rs/commit/b29aea7bf2484f6bbb3d1d51f4fd5541e93bed28))
* **deps-dev:** bump lint-staged from 14.0.1 to 15.1.0 ([32416ee](https://github.com/vinayakkulkarni/tileserver-rs/commit/32416ee9769621edf3db97cd6b2c42e13102ba5a))
* **deps-dev:** bump lint-staged from 15.1.0 to 15.2.0 ([3f8a22a](https://github.com/vinayakkulkarni/tileserver-rs/commit/3f8a22ac1838aefa20bdd666bd7e649984e9847d))
* **deps-dev:** bump lint-staged from 15.2.0 to 15.2.5 ([ece119f](https://github.com/vinayakkulkarni/tileserver-rs/commit/ece119f98648ed7ca2edcac1a666d6c25bebb359))
* **deps-dev:** bump nuxt from 3.4.3 to 3.5.0 in /client ([0b77f90](https://github.com/vinayakkulkarni/tileserver-rs/commit/0b77f900463e1e7de665b4a71a853741fa6efdd1))
* **deps-dev:** bump nuxt from 3.4.3 to 3.5.0 in /client ([21ce4eb](https://github.com/vinayakkulkarni/tileserver-rs/commit/21ce4eb5030b6e3c30209305e1dd110c0eebe217))
* **deps-dev:** bump nuxt from 3.5.0 to 3.5.1 in /client ([5eb0346](https://github.com/vinayakkulkarni/tileserver-rs/commit/5eb0346d70ea912bb1f97125b5022c4b9eab5183))
* **deps-dev:** bump nuxt from 3.5.0 to 3.5.1 in /client ([33640e6](https://github.com/vinayakkulkarni/tileserver-rs/commit/33640e65ca07378b8380970abc677067f334cf3f))
* **deps-dev:** bump nuxt from 3.5.1 to 3.5.2 in /client ([3dec7e0](https://github.com/vinayakkulkarni/tileserver-rs/commit/3dec7e0c8a9f920f88fc18358df47fb732de7206))
* **deps-dev:** bump nuxt from 3.5.1 to 3.5.2 in /client ([212cfc4](https://github.com/vinayakkulkarni/tileserver-rs/commit/212cfc4138335479b89938a39cd73183af3b4dc1))
* **deps-dev:** bump nuxt from 3.5.2 to 3.5.3 in /client ([d31b812](https://github.com/vinayakkulkarni/tileserver-rs/commit/d31b8121451641e9e4641d7cf34e7b8f0434e505))
* **deps-dev:** bump nuxt from 3.5.2 to 3.5.3 in /client ([9b5fb98](https://github.com/vinayakkulkarni/tileserver-rs/commit/9b5fb988fc8b2caf085d4da0c9fd7773b859fb7e))
* **deps-dev:** bump nuxt from 3.5.3 to 3.6.1 in /client ([3dd08ee](https://github.com/vinayakkulkarni/tileserver-rs/commit/3dd08eeb29cff13313af6f4af21d95eafec03493))
* **deps-dev:** bump nuxt from 3.5.3 to 3.6.1 in /client ([12f248d](https://github.com/vinayakkulkarni/tileserver-rs/commit/12f248d52f8363bc12a9e327066c681a674409cd))
* **deps-dev:** bump nuxt from 3.6.1 to 3.6.2 in /client ([49ae5f0](https://github.com/vinayakkulkarni/tileserver-rs/commit/49ae5f0dc2f378c4917491b827b96c77b7d8611d))
* **deps-dev:** bump nuxt from 3.6.1 to 3.6.2 in /client ([f447d5c](https://github.com/vinayakkulkarni/tileserver-rs/commit/f447d5c0ee7ae537c3627f29efbfc35dccfd4321))
* **deps-dev:** bump nuxt from 3.6.2 to 3.6.3 in /client ([2de07cc](https://github.com/vinayakkulkarni/tileserver-rs/commit/2de07cc0c22a4900bf6141a7708733b2eef7238c))
* **deps-dev:** bump nuxt from 3.6.2 to 3.6.3 in /client ([9d63643](https://github.com/vinayakkulkarni/tileserver-rs/commit/9d636431aa9f96cdc76d9eaf4104674076c9dd24))
* **deps-dev:** bump nuxt from 3.6.3 to 3.6.5 in /client ([82e8af5](https://github.com/vinayakkulkarni/tileserver-rs/commit/82e8af59f6284ff203edd01dd2667b697e7fea36))
* **deps-dev:** bump nuxt from 3.6.3 to 3.6.5 in /client ([9d0be26](https://github.com/vinayakkulkarni/tileserver-rs/commit/9d0be26e4f44c0de11d084e9c30067a7f767d825))
* **deps-dev:** bump nuxt from 3.6.5 to 3.7.0 in /client ([7735ce6](https://github.com/vinayakkulkarni/tileserver-rs/commit/7735ce61bd71a2c50cc8a004385a3c5d7790a8c9))
* **deps-dev:** bump nuxt from 3.6.5 to 3.7.0 in /client ([17160c6](https://github.com/vinayakkulkarni/tileserver-rs/commit/17160c645a43f0c60b9ded96b3133d0662f2667a))
* **deps-dev:** bump nuxt from 3.7.0 to 3.7.1 in /client ([353d919](https://github.com/vinayakkulkarni/tileserver-rs/commit/353d9194e166a2e8c40e5cf930407150ae830765))
* **deps-dev:** bump nuxt from 3.7.0 to 3.7.1 in /client ([13c9561](https://github.com/vinayakkulkarni/tileserver-rs/commit/13c95611854163ad2af85e9c15a1620c598666e6))
* **deps-dev:** bump nuxt from 3.7.1 to 3.7.2 in /client ([6c0982a](https://github.com/vinayakkulkarni/tileserver-rs/commit/6c0982a9dfc3bf871e88c280eee25315217dd274))
* **deps-dev:** bump nuxt from 3.7.1 to 3.7.2 in /client ([6282967](https://github.com/vinayakkulkarni/tileserver-rs/commit/6282967eba9a1f7eccdb2385ec20fb9ac9552d10))
* **deps-dev:** bump nuxt from 3.7.2 to 3.7.3 in /client ([76854de](https://github.com/vinayakkulkarni/tileserver-rs/commit/76854dee42f4a49dbfd6ae19b490858fd62392ee))
* **deps-dev:** bump nuxt from 3.7.2 to 3.7.3 in /client ([6adb03c](https://github.com/vinayakkulkarni/tileserver-rs/commit/6adb03c414ce80831e9040bf9d18076e3cd60468))
* **deps-dev:** bump nuxt from 3.7.3 to 3.7.4 in /client ([5e5272a](https://github.com/vinayakkulkarni/tileserver-rs/commit/5e5272a85e39e0e0317bbb87202acebe970b1161))
* **deps-dev:** bump nuxt from 3.7.3 to 3.7.4 in /client ([db6b57c](https://github.com/vinayakkulkarni/tileserver-rs/commit/db6b57c5238ad72c953a5e329553522f5c4f597c))
* **deps-dev:** bump nuxt from 3.7.4 to 3.8.0 in /client ([64bde0b](https://github.com/vinayakkulkarni/tileserver-rs/commit/64bde0bbe8efe3b357fde3abc3a9f14faac795f5))
* **deps-dev:** bump nuxt from 3.7.4 to 3.8.0 in /client ([3c62669](https://github.com/vinayakkulkarni/tileserver-rs/commit/3c62669f8c3c5342b555edcb9d5d328fe5bff3df))
* **deps-dev:** bump nuxt from 3.8.0 to 3.8.1 in /client ([02fd68d](https://github.com/vinayakkulkarni/tileserver-rs/commit/02fd68d78e6045a0064d292e9181aa40b9ff7948))
* **deps-dev:** bump nuxt from 3.8.0 to 3.8.1 in /client ([17c851d](https://github.com/vinayakkulkarni/tileserver-rs/commit/17c851d0a7c7efea2f074c9ef1fe4df62b7a2430))
* **deps-dev:** bump nuxt from 3.8.1 to 3.8.2 in /client ([764a4b4](https://github.com/vinayakkulkarni/tileserver-rs/commit/764a4b493fd18bd6a0d46e8d2dd2be2148e279cf))
* **deps-dev:** bump postcss from 8.4.30 to 8.4.31 in /client ([55273cd](https://github.com/vinayakkulkarni/tileserver-rs/commit/55273cd3dc9b11ad96071e175e562889647c6175))
* **deps-dev:** bump postcss from 8.4.30 to 8.4.31 in /client ([99d9544](https://github.com/vinayakkulkarni/tileserver-rs/commit/99d95440e864e6659724bbb94b1e310a4ab43868))
* **deps-dev:** bump sass from 1.62.1 to 1.63.2 in /client ([f98f264](https://github.com/vinayakkulkarni/tileserver-rs/commit/f98f2640ed5956dcfad10b9eae50bc17225e7447))
* **deps-dev:** bump sass from 1.62.1 to 1.63.2 in /client ([7eb2eef](https://github.com/vinayakkulkarni/tileserver-rs/commit/7eb2eefcd24b6e0eb54ab45c4e9f5427ca46a927))
* **deps-dev:** bump sass from 1.63.2 to 1.63.3 in /client ([5c6cf7a](https://github.com/vinayakkulkarni/tileserver-rs/commit/5c6cf7a9ffc0b9d82a303f6a9fdc70d2a4e2937c))
* **deps-dev:** bump sass from 1.63.2 to 1.63.3 in /client ([371c4c1](https://github.com/vinayakkulkarni/tileserver-rs/commit/371c4c13848e7607f97e424b79e62e953ce67aa1))
* **deps-dev:** bump sass from 1.63.3 to 1.63.4 in /client ([d5b2dcd](https://github.com/vinayakkulkarni/tileserver-rs/commit/d5b2dcdc13cb62b6c09be7bc2e04d55cd9c10469))
* **deps-dev:** bump sass from 1.63.3 to 1.63.4 in /client ([2ef9de5](https://github.com/vinayakkulkarni/tileserver-rs/commit/2ef9de518a5ba8e617554917bbc0d4949349c306))
* **deps-dev:** bump sass from 1.63.4 to 1.63.6 in /client ([3e4fe5a](https://github.com/vinayakkulkarni/tileserver-rs/commit/3e4fe5a97c2a8ee322bc7440e65d0d59ffb71734))
* **deps-dev:** bump sass from 1.63.4 to 1.63.6 in /client ([557ffa4](https://github.com/vinayakkulkarni/tileserver-rs/commit/557ffa4ce36cc9c35f239d6adc1cf62747f71dcc))
* **deps-dev:** bump sass from 1.63.6 to 1.64.0 in /client ([c8f0c43](https://github.com/vinayakkulkarni/tileserver-rs/commit/c8f0c439282a6976be21261e20d9ecaac16197cd))
* **deps-dev:** bump sass from 1.63.6 to 1.64.0 in /client ([b819e07](https://github.com/vinayakkulkarni/tileserver-rs/commit/b819e0781b9273713366e2632fe3367d2f7f2072))
* **deps-dev:** bump sass from 1.64.0 to 1.64.1 in /client ([80ec939](https://github.com/vinayakkulkarni/tileserver-rs/commit/80ec939ec3fc47561d72e913951ec93ea654ca99))
* **deps-dev:** bump sass from 1.64.0 to 1.64.1 in /client ([f0c13d5](https://github.com/vinayakkulkarni/tileserver-rs/commit/f0c13d587e6fe667215cfc7f0575c9eaa2581f6b))
* **deps-dev:** bump sass from 1.64.1 to 1.64.2 in /client ([6b5e4c6](https://github.com/vinayakkulkarni/tileserver-rs/commit/6b5e4c6debde3822b9a57c99c4daf52ad827d625))
* **deps-dev:** bump sass from 1.64.1 to 1.64.2 in /client ([22f8a7f](https://github.com/vinayakkulkarni/tileserver-rs/commit/22f8a7f9403b4b9a82a0928ccb2e95f4551bc36d))
* **deps-dev:** bump sass from 1.64.2 to 1.65.1 in /client ([d1e69bb](https://github.com/vinayakkulkarni/tileserver-rs/commit/d1e69bb8372f4165cd79d7a550445a644939755e))
* **deps-dev:** bump sass from 1.64.2 to 1.65.1 in /client ([63c59c5](https://github.com/vinayakkulkarni/tileserver-rs/commit/63c59c5851f621aaad8a72883df000320420e897))
* **deps-dev:** bump sass from 1.65.1 to 1.66.0 in /client ([50b5a83](https://github.com/vinayakkulkarni/tileserver-rs/commit/50b5a833b64426321ba7fcde71b780bcc512f5dc))
* **deps-dev:** bump sass from 1.65.1 to 1.66.0 in /client ([b4ed2e8](https://github.com/vinayakkulkarni/tileserver-rs/commit/b4ed2e89689af8a58cdf65087c022e010b1c27c8))
* **deps-dev:** bump sass from 1.66.0 to 1.66.1 in /client ([bab1df2](https://github.com/vinayakkulkarni/tileserver-rs/commit/bab1df28faff273c5c5c600d2b0c08331ef150bc))
* **deps-dev:** bump sass from 1.66.0 to 1.66.1 in /client ([15a4897](https://github.com/vinayakkulkarni/tileserver-rs/commit/15a48975fae67d7d953d9c7398de212cc8e0f363))
* **deps-dev:** bump sass from 1.66.1 to 1.67.0 in /client ([329dbda](https://github.com/vinayakkulkarni/tileserver-rs/commit/329dbda09fc57efdf7874f9fbc80842eebac9902))
* **deps-dev:** bump sass from 1.66.1 to 1.67.0 in /client ([994dba0](https://github.com/vinayakkulkarni/tileserver-rs/commit/994dba01588182254e5712f222d74df40719d729))
* **deps-dev:** bump sass from 1.67.0 to 1.68.0 in /client ([f4a3b4c](https://github.com/vinayakkulkarni/tileserver-rs/commit/f4a3b4c99c633d7a733443647b8bd5c36606abb7))
* **deps-dev:** bump sass from 1.67.0 to 1.68.0 in /client ([a428bb9](https://github.com/vinayakkulkarni/tileserver-rs/commit/a428bb9c3bd954307b154106b4db29b84ee2fece))
* **deps-dev:** bump sass from 1.68.0 to 1.69.0 in /client ([35b078f](https://github.com/vinayakkulkarni/tileserver-rs/commit/35b078ff9a5deca2d045aa9aa61b49c1d066b696))
* **deps-dev:** bump sass from 1.68.0 to 1.69.0 in /client ([9acae38](https://github.com/vinayakkulkarni/tileserver-rs/commit/9acae38706cbbad3d5f88b034f5ea215b42a39be))
* **deps-dev:** bump sass from 1.69.0 to 1.69.1 in /client ([14a4a4b](https://github.com/vinayakkulkarni/tileserver-rs/commit/14a4a4b2a0028d24da511965b3430adef09ced20))
* **deps-dev:** bump sass from 1.69.0 to 1.69.1 in /client ([3a92ea1](https://github.com/vinayakkulkarni/tileserver-rs/commit/3a92ea1b281eaedb892d7a961b84fa15b3c192f8))
* **deps-dev:** bump sass from 1.69.1 to 1.69.2 in /client ([b28aba9](https://github.com/vinayakkulkarni/tileserver-rs/commit/b28aba9f744c05d8b7f095723db3fbac4d3ce107))
* **deps-dev:** bump sass from 1.69.1 to 1.69.2 in /client ([ab98340](https://github.com/vinayakkulkarni/tileserver-rs/commit/ab983406cf36c573f207eef49ee384d4856f8dea))
* **deps-dev:** bump sass from 1.69.2 to 1.69.3 in /client ([8ad0169](https://github.com/vinayakkulkarni/tileserver-rs/commit/8ad016991bf62aeab6f2e86358f4af892d6e90b4))
* **deps-dev:** bump sass from 1.69.2 to 1.69.3 in /client ([eb0e212](https://github.com/vinayakkulkarni/tileserver-rs/commit/eb0e2128f16af780442c2abded6a96eb10d98352))
* **deps-dev:** bump sass from 1.69.3 to 1.69.4 in /client ([fb5eee5](https://github.com/vinayakkulkarni/tileserver-rs/commit/fb5eee55c405a49926ff8fde6a714e313ca86a00))
* **deps-dev:** bump sass from 1.69.3 to 1.69.4 in /client ([5183351](https://github.com/vinayakkulkarni/tileserver-rs/commit/5183351b71658cb55da1a377c0b953d642268a0b))
* **deps-dev:** bump sass from 1.69.4 to 1.69.5 in /client ([4818e03](https://github.com/vinayakkulkarni/tileserver-rs/commit/4818e03bcfdf00435cb157bf5c23e625e2600141))
* **deps-dev:** bump sass from 1.69.4 to 1.69.5 in /client ([82f0b74](https://github.com/vinayakkulkarni/tileserver-rs/commit/82f0b745312c8d7da786a106a8b2e26f8310e597))
* **deps-dev:** bump sass from 1.69.5 to 1.77.4 in /client ([5b89c67](https://github.com/vinayakkulkarni/tileserver-rs/commit/5b89c6713a420ddf429f17959a9e9e4d88c5bdbe))
* **deps-dev:** bump stylelint from 15.10.0 to 15.10.1 in /client ([aa90504](https://github.com/vinayakkulkarni/tileserver-rs/commit/aa905041180354580ada0e4d2670e4619b959a3d))
* **deps-dev:** bump stylelint from 15.10.0 to 15.10.1 in /client ([6169d16](https://github.com/vinayakkulkarni/tileserver-rs/commit/6169d16f69bfb8fb097e2c8741cfb5d29dd5f309))
* **deps-dev:** bump stylelint from 15.10.1 to 15.10.2 in /client ([8b1b1ac](https://github.com/vinayakkulkarni/tileserver-rs/commit/8b1b1acd674fec9f267988a4891298dc43053af1))
* **deps-dev:** bump stylelint from 15.10.1 to 15.10.2 in /client ([8809154](https://github.com/vinayakkulkarni/tileserver-rs/commit/8809154250e8d47772bbfd19d1118a5997a1a981))
* **deps-dev:** bump stylelint from 15.10.2 to 15.10.3 in /client ([73eba46](https://github.com/vinayakkulkarni/tileserver-rs/commit/73eba462b814e0b4f78433f28a6b27ab1b897792))
* **deps-dev:** bump stylelint from 15.10.2 to 15.10.3 in /client ([4502e25](https://github.com/vinayakkulkarni/tileserver-rs/commit/4502e25d3e830e0cf6a8fe8adb871171e3a4f950))
* **deps-dev:** bump stylelint from 15.10.3 to 15.11.0 in /client ([771446b](https://github.com/vinayakkulkarni/tileserver-rs/commit/771446bfb633d60079cca895ede2bc97679b0981))
* **deps-dev:** bump stylelint from 15.10.3 to 15.11.0 in /client ([4703a02](https://github.com/vinayakkulkarni/tileserver-rs/commit/4703a028905b14b791b4724fa0e836f1ce6f16b2))
* **deps-dev:** bump stylelint from 15.11.0 to 16.2.0 in /client ([0457e60](https://github.com/vinayakkulkarni/tileserver-rs/commit/0457e607876e2ac55c7199a356c4c14473ef98aa))
* **deps-dev:** bump stylelint from 15.6.1 to 15.6.2 in /client ([f3dcfdc](https://github.com/vinayakkulkarni/tileserver-rs/commit/f3dcfdc28e208eb967769abbeda070db60fc5b94))
* **deps-dev:** bump stylelint from 15.6.1 to 15.6.2 in /client ([18679cf](https://github.com/vinayakkulkarni/tileserver-rs/commit/18679cf78970ebc4d106176fa4cf94d156e54761))
* **deps-dev:** bump stylelint from 15.6.2 to 15.7.0 in /client ([34e2f88](https://github.com/vinayakkulkarni/tileserver-rs/commit/34e2f886bdb423910ec45018935ca497caaef180))
* **deps-dev:** bump stylelint from 15.6.2 to 15.7.0 in /client ([a17b965](https://github.com/vinayakkulkarni/tileserver-rs/commit/a17b965aee83e5d591e3ed1ec66572bb13a0d825))
* **deps-dev:** bump stylelint from 15.7.0 to 15.8.0 in /client ([c74016c](https://github.com/vinayakkulkarni/tileserver-rs/commit/c74016c0137ac0c3eec4e6ecc2a698b94d78727c))
* **deps-dev:** bump stylelint from 15.7.0 to 15.8.0 in /client ([5f96be9](https://github.com/vinayakkulkarni/tileserver-rs/commit/5f96be9011bd352a5cf1ad0ca439c0275b7e6b34))
* **deps-dev:** bump stylelint from 15.8.0 to 15.9.0 in /client ([7c17786](https://github.com/vinayakkulkarni/tileserver-rs/commit/7c17786a451af5dba0ccc9d73edf80b67ee7fbec))
* **deps-dev:** bump stylelint from 15.8.0 to 15.9.0 in /client ([701e497](https://github.com/vinayakkulkarni/tileserver-rs/commit/701e497376cf2abbaad37ec0800ac81ec33b9964))
* **deps-dev:** bump stylelint from 15.9.0 to 15.10.0 in /client ([5603e9d](https://github.com/vinayakkulkarni/tileserver-rs/commit/5603e9de91f2a346a478b10a33e7d4c3b14abfc5))
* **deps-dev:** bump stylelint from 15.9.0 to 15.10.0 in /client ([982fd81](https://github.com/vinayakkulkarni/tileserver-rs/commit/982fd81b40143c32f6ce8e85a0837c3147d38fcb))
* **deps-dev:** bump stylelint-config-recommended-vue from 1.4.0 to 1.5.0 in /client ([aff317e](https://github.com/vinayakkulkarni/tileserver-rs/commit/aff317e2db3809e31f0d04efb49ea76c9deabde0))
* **deps-dev:** bump stylelint-config-recommended-vue in /client ([06417c2](https://github.com/vinayakkulkarni/tileserver-rs/commit/06417c23376fab5dde0ffacd15c9f1d611401639))
* **deps-dev:** bump typescript from 5.0.4 to 5.1.3 in /client ([ae89706](https://github.com/vinayakkulkarni/tileserver-rs/commit/ae89706fefe74f4ea9a6e9adf8f8770524c8a309))
* **deps-dev:** bump typescript from 5.0.4 to 5.1.3 in /client ([9288ded](https://github.com/vinayakkulkarni/tileserver-rs/commit/9288dedce3e8ed53a434442dd900974dd4c7f9da))
* **deps-dev:** bump typescript from 5.1.3 to 5.1.5 in /client ([bc65643](https://github.com/vinayakkulkarni/tileserver-rs/commit/bc65643f95f299f8b962d7a4f9ab945e440875fa))
* **deps-dev:** bump typescript from 5.1.3 to 5.1.5 in /client ([9b76750](https://github.com/vinayakkulkarni/tileserver-rs/commit/9b7675068d5d0a0119a67f2816cc18f1359702a0))
* **deps-dev:** bump typescript from 5.1.5 to 5.1.6 in /client ([0604bbc](https://github.com/vinayakkulkarni/tileserver-rs/commit/0604bbc9bb7b692f8103ec12fbaee52ac5a1bd2d))
* **deps-dev:** bump typescript from 5.1.5 to 5.1.6 in /client ([b49a98a](https://github.com/vinayakkulkarni/tileserver-rs/commit/b49a98a0e34531d33e2b9435709bb4ec7657a329))
* **deps-dev:** bump typescript from 5.1.6 to 5.2.2 in /client ([58ab284](https://github.com/vinayakkulkarni/tileserver-rs/commit/58ab284f9b23db8ab195e6f53dbf98a917c69959))
* **deps-dev:** bump typescript from 5.1.6 to 5.2.2 in /client ([90e1df4](https://github.com/vinayakkulkarni/tileserver-rs/commit/90e1df47208ca53a0c00c6bf32082970e415e881))
* **deps-dev:** bump typescript from 5.2.2 to 5.3.2 in /client ([d62c9ab](https://github.com/vinayakkulkarni/tileserver-rs/commit/d62c9ab63ef0c515fc6e40e06ac34880552852b7))
* **deps-dev:** bump typescript from 5.2.2 to 5.3.2 in /client ([a78fc14](https://github.com/vinayakkulkarni/tileserver-rs/commit/a78fc147899f5e5b89822f5c142dc74bf67b938b))
* **deps-dev:** bump undici from 5.24.0 to 5.26.3 in /client ([f2b5571](https://github.com/vinayakkulkarni/tileserver-rs/commit/f2b5571bfe0d4e7aac7f9b6289ef512f5f1d68cb))
* **deps-dev:** bump undici from 5.24.0 to 5.26.3 in /client ([d22598b](https://github.com/vinayakkulkarni/tileserver-rs/commit/d22598b88a5794c2aa527c96513d3057dbf676bb))
* **deps-dev:** bump undici from 5.28.2 to 5.28.3 in /client ([d3dea80](https://github.com/vinayakkulkarni/tileserver-rs/commit/d3dea808012e7bd2d1fa07684a27af7f7f6cc357))
* **deps-dev:** bump vite from 4.5.0 to 4.5.2 in /client ([69fc1ff](https://github.com/vinayakkulkarni/tileserver-rs/commit/69fc1ff1ef75b09daaa822176e506fde76f32d5a))
* **docs:** bump dependencies ([1fe638a](https://github.com/vinayakkulkarni/tileserver-rs/commit/1fe638a48d3edf84ac53d6e7a471f19f194e3743))
* minor clean up ✨ ([bb04c96](https://github.com/vinayakkulkarni/tileserver-rs/commit/bb04c9645670deee190d098035fa50ff340221de))
* simplify Docker Compose setup ([619d05d](https://github.com/vinayakkulkarni/tileserver-rs/commit/619d05d3bbe95351350c8fb6d46e9ef7353a9f86))


### Code Refactoring

* docs ([cbb7fba](https://github.com/vinayakkulkarni/tileserver-rs/commit/cbb7fba9454520320136198529859aedc60a0b86))
* init 6 ✨ ([6e76b0d](https://github.com/vinayakkulkarni/tileserver-rs/commit/6e76b0db025779efc84f25a360d6d0d70ee37531))
* move fonts in `data` directory ([42eb4d5](https://github.com/vinayakkulkarni/tileserver-rs/commit/42eb4d5022bdb2d4322ef0305c243cac50625505))
