# Changelog

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
