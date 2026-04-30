# Changelog

## [2.27.1](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.27.0...v2.27.1) (2026-04-30)


### Bug Fixes

* **deps:** bump maplibre-gl from 5.23.0 to 5.24.0 ([1cb84cf](https://github.com/vinayakkulkarni/tileserver-rs/commit/1cb84cf63d4508e0a83df909de5bf7d64253db10))

## [2.27.0](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.26.4...v2.27.0) (2026-04-26)


### Features

* **benchmarks:** real-world grid mode + deploy/ split + bench refresh ([243c3e7](https://github.com/vinayakkulkarni/tileserver-rs/commit/243c3e73d131edaa464f4843a007bff007531d8c))


### Bug Fixes

* **sources:** make raster feature compile standalone without postgres ([114b7e9](https://github.com/vinayakkulkarni/tileserver-rs/commit/114b7e99d589be1b16d05b8a145c3907bd789cac))


### Performance Improvements

* **bench:** add benches for stac, cog, postgres, frontend, duckdb, geoparquet features ([67ade59](https://github.com/vinayakkulkarni/tileserver-rs/commit/67ade59cca68497f73e9e10eb71e9265e0609c3e))
* **benches:** add style-rewrite bench + audit cache/mlt/raster ([c675857](https://github.com/vinayakkulkarni/tileserver-rs/commit/c675857a459820ba297c964de5cf142ee4b25930))

## [2.26.4](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.26.3...v2.26.4) (2026-04-23)


### Bug Fixes

* **deps:** bump vue from 3.5.32 to 3.5.33 ([424abe8](https://github.com/vinayakkulkarni/tileserver-rs/commit/424abe8))


### Code Refactoring

* **client:** swap markstream-vue for @comark/nuxt ([78e19fc](https://github.com/vinayakkulkarni/tileserver-rs/commit/78e19fc))


### Miscellaneous

* **deps:** bump workspace catalogs + align lockfiles ([9323f70](https://github.com/vinayakkulkarni/tileserver-rs/commit/9323f70))

## [2.26.3](https://github.com/vinayakkulkarni/tileserver-rs/compare/v2.26.2...v2.26.3) (2026-04-22)


### Bug Fixes

* **tests:** resolve data/ paths relative to workspace root after crate move ([84c9743](https://github.com/vinayakkulkarni/tileserver-rs/commit/84c9743539f8e647f30bb602e7016f27b8f65515))


### Code Refactoring

* **workspace:** move tileserver-rs crate to crates/tileserver-rs/ ([11c3afb](https://github.com/vinayakkulkarni/tileserver-rs/commit/11c3afb4200bae564505865a71d14c43dc92dd50))
