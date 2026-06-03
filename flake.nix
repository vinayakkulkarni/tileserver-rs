{
  description = "tileserver-rs — high-performance vector + raster tile server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      crane,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        inherit (pkgs) lib stdenv clangStdenv;

        # Pin the toolchain to the exact channel in rust-toolchain.toml so the
        # flake never drifts from the dtolnay/rust-toolchain CI install.
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # The Cargo source feeds crane. Keep the full tree (build.rs + vendored
        # mbgl-sys headers need siblings), only filtering target/, .git/, result/
        # and the heavy node/frontend dirs that the Rust build never reads.
        src = lib.cleanSourceWith {
          src = ./.;
          filter =
            path: type:
            (craneLib.filterCargoSources path type)
            || (lib.hasSuffix "/maplibre_c.h" path)
            || (lib.hasSuffix "/maplibre_c.cpp" path)
            || (lib.hasSuffix "/maplibre_c_stub.c" path);
        };

        version = (builtins.fromTOML (builtins.readFile ./crates/tileserver-rs/Cargo.toml)).package.version;

        # --- MapLibre Native (mbgl-sys) -------------------------------------
        # mbgl-sys/build.rs exposes an MBGL_SYS_LIB_DIR escape hatch that links
        # pre-built static archives instead of compiling the vendored C++ tree
        # inside the Rust build. We build those archives once here (cmake +
        # ninja, the macos-metal / linux-opengl presets) and hand the directory
        # to the Rust derivation via env. This keeps the long C++ compile in its
        # own cached derivation rather than rerunning on every Rust change.
        #
        # clangStdenv (not the default gcc stdenv): maplibre-native's cmake
        # presets hardcode clang/clang++ as CMAKE_C(XX)_COMPILER, so the build
        # sandbox must provide clang on PATH or configure fails before any
        # compilation starts.
        maplibre-native = clangStdenv.mkDerivation {
          pname = "maplibre-native-mbgl";
          inherit version;
          # Fetch the vendored submodule by its pinned rev rather than the flake
          # git tree: Nix flakes don't expose submodule contents, and this source
          # carries 38 nested submodules that fetchSubmodules pulls in. Keep this
          # rev in lockstep with .gitmodules (crates/mbgl-sys/vendor/maplibre-native).
          src = pkgs.fetchFromGitHub {
            owner = "maplibre";
            repo = "maplibre-native";
            rev = "ee9ddebc3367601a1b99e1bd357b88c25e548150"; # ios-v6.22.1
            fetchSubmodules = true;
            hash = "sha256-ifGep22nGlax2044Ffrj/q6iT63Tx5gd1y4szVskSz4=";
          };

          nativeBuildInputs = with pkgs; [
            cmake
            ninja
            pkg-config
          ];
          buildInputs =
            (with pkgs; [
              curl
              libuv
              glfw3
            ])
            # Linux opengl preset (platform/linux/linux.cmake) pulls in extra
            # system libs the macos-metal preset gets from the Apple SDK.
            ++ lib.optionals stdenv.hostPlatform.isLinux (with pkgs; [
              libjpeg
              libpng
              libwebp
              libuuid
              icu
              libGL
              libGLU
              xorg.libX11
              wayland
              libxkbcommon
            ])
            ++ lib.optionals stdenv.hostPlatform.isDarwin [ pkgs.apple-sdk ];

          # -DMLN_WITH_GLFW=OFF: the GLFW platform target is interactive
          # windowing/demo code the headless server never links. It also drags
          # in a tinyobjloader FetchContent that git-clones at configure time —
          # impossible inside Nix's network-less build sandbox. The server only
          # needs the mbgl-core + mlt-cpp library targets.
          #
          # Empty *_COMPILER_LAUNCHER: maplibre-native wires ccache as the
          # compiler launcher, but Nix builds run with no writable CCACHE_DIR
          # (ccache: Permission denied) and ccache buys nothing in an isolated
          # clean-build derivation. Clear the launcher so clang runs directly.
          configurePhase = ''
            runHook preConfigure
            cmake --preset ${
              if stdenv.hostPlatform.isDarwin then "macos-metal" else "linux-opengl"
            } -DMLN_WITH_GLFW=OFF \
              -DCMAKE_C_COMPILER_LAUNCHER= \
              -DCMAKE_CXX_COMPILER_LAUNCHER=
            runHook postConfigure
          '';

          # Build every static archive that mbgl-sys/build.rs links, not just
          # mbgl-core+mlt-cpp: the vendored freetype/harfbuzz/csscolorparser/
          # parsedate/nunicode/sqlite archives are separate CMake targets and
          # must exist in $out/lib for the Rust link step to resolve.
          buildPhase = ''
            runHook preBuild
            cmake --build ${
              if stdenv.hostPlatform.isDarwin then "build-macos-metal" else "build-linux-opengl"
            } --target mbgl-core mlt-cpp mbgl-freetype mbgl-harfbuzz \
              mbgl-vendor-csscolorparser mbgl-vendor-parsedate \
              ${
                if stdenv.hostPlatform.isDarwin then "mbgl-vendor-icu" else "mbgl-vendor-nunicode mbgl-vendor-sqlite"
              } -j$NIX_BUILD_CORES
            runHook postBuild
          '';

          # Install the static archives AND the header tree the maplibre_c
          # wrapper compiles against. Headers keep their source-relative layout
          # under $out/include so the mbgl-c-wrapper derivation can pass the
          # exact -I set mbgl-sys/build.rs uses (include/, platform/*/include,
          # src/, vendor/maplibre-native-base/..., vendor/rapidjson/include).
          installPhase = ''
            runHook preInstall
            mkdir -p $out/lib $out/include
            find . -name '*.a' -exec cp {} $out/lib/ \;

            cp -r include $out/include/include
            cp -r src $out/include/src
            mkdir -p $out/include/platform
            cp -r platform/default $out/include/platform/default
            ${lib.optionalString stdenv.hostPlatform.isLinux ''
              cp -r platform/linux $out/include/platform/linux
            ''}
            ${lib.optionalString stdenv.hostPlatform.isDarwin ''
              cp -r platform/darwin $out/include/platform/darwin
            ''}
            mkdir -p $out/include/vendor
            cp -r vendor/maplibre-native-base $out/include/vendor/maplibre-native-base
            cp -r vendor/rapidjson $out/include/vendor/rapidjson
            runHook postInstall
          '';
        };

        # --- maplibre_c C-API wrapper --------------------------------------
        # mbgl-sys ships a thin C wrapper (crates/mbgl-sys/cpp/maplibre_c.cpp)
        # around mbgl::*. build.rs's MBGL_SYS_LIB_DIR path expects a pre-built
        # libmaplibre_c.a alongside the mbgl archives, but the upstream cmake
        # build knows nothing about our shim. Compile it here against the
        # maplibre-native headers, then expose ONE lib dir that holds both the
        # wrapper and (symlinked) mbgl archives so a single MBGL_SYS_LIB_DIR
        # satisfies every -l the Rust link emits. Flags mirror build.rs's
        # cc::Build exactly (-std=c++20 -fPIC -fvisibility=hidden, warnings off).
        mbgl-c-wrapper = clangStdenv.mkDerivation {
          pname = "mbgl-c-wrapper";
          inherit version;
          src = lib.cleanSourceWith {
            src = ./crates/mbgl-sys/cpp;
            filter =
              path: _type:
              lib.hasSuffix "/maplibre_c.cpp" path || lib.hasSuffix "/maplibre_c.h" path;
          };
          buildInputs = [ maplibre-native ];
          mbglInc = "${maplibre-native}/include";
          buildPhase = ''
            runHook preBuild
            $CXX -std=c++20 -fPIC -fvisibility=hidden -w \
              -I . \
              -I $mbglInc/include \
              -I $mbglInc/platform/default/include \
              -I $mbglInc/src \
              -I $mbglInc/vendor/maplibre-native-base/extras/expected-lite/include \
              -I $mbglInc/vendor/maplibre-native-base/include \
              -I $mbglInc/vendor/maplibre-native-base/deps/geojson.hpp/include \
              -I $mbglInc/vendor/maplibre-native-base/deps/geometry.hpp/include \
              -I $mbglInc/vendor/maplibre-native-base/deps/variant/include \
              -I $mbglInc/vendor/maplibre-native-base/deps/optional/include \
              -I $mbglInc/vendor/rapidjson/include \
              ${
                if stdenv.hostPlatform.isDarwin then
                  "-I $mbglInc/platform/darwin/include -mmacosx-version-min=14.3"
                else
                  "-I $mbglInc/platform/linux/include"
              } \
              -c maplibre_c.cpp -o maplibre_c.o
            $AR rcs libmaplibre_c.a maplibre_c.o
            runHook postBuild
          '';
          # One lib dir for the linker: the freshly built wrapper plus symlinks
          # to every mbgl archive, so build.rs's single MBGL_SYS_LIB_DIR -L flag
          # resolves -lmaplibre_c AND -lmbgl-core/-lmlt-cpp/-lmbgl-vendor-*.
          installPhase = ''
            runHook preInstall
            mkdir -p $out/lib
            cp libmaplibre_c.a $out/lib/
            ln -s ${maplibre-native}/lib/*.a $out/lib/
            runHook postInstall
          '';
        };

        # --- Embedded Nuxt frontend ----------------------------------------
        # rust-embed bakes apps/client/.output/public into the binary (see
        # crates/tileserver-rs/src/main.rs #[folder = "../../apps/client/.output/public"]).
        # Build it as its own pnpm derivation, then copy the output into the
        # Rust source tree in preBuild — the martin/martin-ui pattern.
        frontend = stdenv.mkDerivation (finalAttrs: {
          pname = "tileserver-rs-client";
          inherit version;
          src = ./.;

          # fetcherVersion 4 + pnpm_11: matches the project's packageManager
          # pin (pnpm@11.3.0, lockfileVersion 9.0). fetcherVersion 3 produced
          # non-deterministic hashes across CI runs (nixpkgs#484013); v4 dumps
          # pnpm's SQLite store index to a reproducible SQL text file
          # (nixpkgs#522703), making the FOD hash stable and arch-independent.
          pnpmDeps = pkgs.fetchPnpmDeps {
            inherit (finalAttrs) pname version src;
            pnpm = pkgs.pnpm_11;
            fetcherVersion = 4;
            hash = "sha256-ASHylImzfWGOq4z37xGGDhn/Ry+0hyBDfElV4cEDxKM=";
          };

          nativeBuildInputs = [
            pkgs.nodejs_24
            pkgs.pnpmConfigHook
            pkgs.pnpm_11
          ];

          buildPhase = ''
            runHook preBuild
            pnpm --filter @tileserver-rs/client run build
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            cp -r apps/client/.output/public $out
            runHook postInstall
          '';
        });

        # --- Common crane args ---------------------------------------------
        commonArgs = {
          inherit src version;
          pname = "tileserver-rs";
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake
            rustPlatform.bindgenHook
            protobuf
          ];

          buildInputs =
            (with pkgs; [
              openssl
              zlib
              zstd
              gdal
              libpq
              postgresql
              duckdb
            ])
            # System libs mbgl-sys/build.rs link_system_libs() emits for the
            # raster feature on Linux (ICU is not vendored; GL/EGL/X11 come from
            # the linux-opengl preset's MLN_WITH_OPENGL=ON).
            ++ lib.optionals stdenv.hostPlatform.isLinux (with pkgs; [
              icu
              curl
              libpng
              libjpeg
              libwebp
              libuv
              libGL
              xorg.libX11
              sqlite
            ])
            ++ lib.optionals stdenv.hostPlatform.isDarwin [ pkgs.apple-sdk ];

          # gdal-sys/bindgen + libduckdb-sys env. gdal is found via pkg-config
          # from buildInputs; bindgenHook wires LIBCLANG_PATH. duckdb env points
          # libduckdb-sys at the nixpkgs lib so it skips the vendored build.
          env = {
            DUCKDB_LIB_DIR = "${pkgs.duckdb}/lib";
            DUCKDB_INCLUDE_DIR = "${pkgs.duckdb}/include";
            # Link the pre-built MapLibre Native archives + the maplibre_c
            # wrapper for the raster feature. mbgl-c-wrapper bundles both in one
            # lib dir so build.rs's single MBGL_SYS_LIB_DIR -L resolves every -l.
            MBGL_SYS_LIB_DIR = "${mbgl-c-wrapper}/lib";
          };
        };

        # Build the dependency tree once and reuse it across every package and
        # check derivation — the whole reason to use crane.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Inject the built frontend before the Rust build sees the source so
        # rust-embed finds apps/client/.output/public populated.
        embedFrontend = ''
          mkdir -p apps/client/.output
          cp -r ${frontend} apps/client/.output/public
        '';

        # doCheck = false on the package builds: several unit tests load runtime
        # fixtures (e.g. data/tiles/zurich_switzerland.mbtiles) that the flake
        # `src` filter intentionally strips, so `cargo test` can't find them in
        # the sandbox. The package build's job is the binary; the test suite runs
        # in CI (ci-rust.yml / coverage.yml) where the fixtures are present.
        mkPkg =
          features:
          craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              doCheck = false;
              preBuild = embedFrontend;
              cargoExtraArgs = "--locked --no-default-features --features ${features}";
            }
          );

        defaultFeatures = "postgres,raster,mlt,cloud,stac,frontend";
      in
      {
        packages = {
          default = mkPkg defaultFeatures;
          slim = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              doCheck = false;
              cargoExtraArgs = "--locked --no-default-features --features postgres";
            }
          );
          full = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              doCheck = false;
              preBuild = embedFrontend;
              cargoExtraArgs = "--locked --all-features";
            }
          );
          inherit maplibre-native frontend;
        };

        checks = {
          inherit (self.packages.${system}) default;
          clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets --all-features -- -D warnings";
            }
          );
          fmt = craneLib.cargoFmt { inherit src; };
        };

        devShells.default = craneLib.devShell {
          inputsFrom = [ self.packages.${system}.default ];
          inherit (commonArgs) env;
          packages = with pkgs; [
            nodejs_24
            pnpm_10
            postgresql_17
            postgresql17Packages.postgis
            gdal
            gh
            rust-analyzer
          ];
        };

        formatter = pkgs.nixfmt-rfc-style;
      }
    );
}
