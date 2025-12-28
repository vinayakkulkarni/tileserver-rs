# Publishing tileserver-rs to crates.io

This document outlines the plan to restructure tileserver-rs for publishing to crates.io, following Rust ecosystem best practices (similar to git2-rs, rusqlite, etc.).

## Target Structure

```
tileserver-rs/
├── Cargo.toml                    # Workspace root + tileserver-rs binary
├── src/                          # Binary source (HTTP server, CLI)
│   ├── main.rs
│   ├── cli.rs
│   ├── logging.rs
│   ├── telemetry.rs
│   ├── openapi.rs
│   ├── wmts.rs
│   ├── cache_control.rs
│   └── render/                   # Native rendering (uses maplibre-native-sys)
│
├── tileserver-core/              # Library crate (publishable)
│   ├── Cargo.toml
│   ├── README.md
│   └── src/
│       ├── lib.rs
│       ├── config.rs
│       ├── error.rs
│       ├── sources/              # PMTiles, MBTiles
│       └── styles/               # Style management
│
├── maplibre-native-sys/          # FFI crate (publishable)
│   ├── Cargo.toml
│   ├── README.md
│   ├── build.rs                  # Downloads pre-built OR builds from source
│   ├── src/lib.rs
│   ├── cpp/                      # C wrapper code
│   └── vendor/maplibre-native/   # Git submodule (optional, for bundled builds)
│
└── .github/workflows/
    ├── ci-rust.yml               # Lint, test, build
    ├── build-maplibre-native.yml # NEW: Build & cache native libs
    ├── release-native-libs.yml   # NEW: Publish pre-built libs
    └── release-*.yml             # Existing release workflows
```

## Crate Descriptions

### 1. `tileserver-core` (NEW)

Core library for tile serving, no native dependencies.

**Publishable to crates.io**: Yes
**Dependencies**: Pure Rust (pmtiles, rusqlite, serde, etc.)

```toml
[package]
name = "tileserver-core"
version = "0.1.0"

[features]
default = []
http = ["reqwest", "pmtiles/http-async"]  # Remote PMTiles support
```

**Exports**:
- `TileSource` trait
- `PmTilesSource`, `MBTilesSource`
- `SourceManager`
- `StyleManager`
- `Config`

### 2. `maplibre-native-sys` (EXISTING, ENHANCED)

Low-level FFI bindings to MapLibre GL Native.

**Publishable to crates.io**: Yes (with pre-built binaries)
**Strategy**: Download pre-built static libraries OR build from source

```toml
[package]
name = "maplibre-native-sys"
version = "0.3.0"
links = "maplibre-native"

[features]
default = []
# Download pre-built libraries (fast, recommended)
prebuilt = []
# Build from vendored source (slow, requires cmake/ninja)
vendored = []
```

**Pre-built Library Distribution**:
- Host on GitHub Releases as `maplibre-native-libs-{version}-{target}.tar.gz`
- Targets: `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`
- Build script downloads appropriate archive based on target

### 3. `tileserver-rs` (EXISTING, UPDATED)

The CLI binary that users install.

**Publishable to crates.io**: Yes
**Install**: `cargo install tileserver-rs`

```toml
[package]
name = "tileserver-rs"
version = "0.3.0"

[dependencies]
tileserver-core = "0.1"
maplibre-native-sys = { version = "0.3", features = ["prebuilt"] }

[features]
default = ["native-render"]
native-render = ["maplibre-native-sys"]
```

## Implementation Plan

### Phase 1: Create tileserver-core crate

1. Create `tileserver-core/` directory
2. Move `src/sources/`, `src/styles/`, `src/config.rs`, `src/error.rs`
3. Create clean public API in `lib.rs`
4. Update imports in main crate to use `tileserver_core::`
5. Test: `cargo check -p tileserver-core`

### Phase 2: Enhance maplibre-native-sys

1. Add `prebuilt` feature to Cargo.toml
2. Update `build.rs`:
   - If `prebuilt`: download from GitHub Releases
   - If `vendored`: build from source (current behavior)
   - If neither: use stub
3. Create GitHub Release workflow for pre-built libs

### Phase 3: GitHub Actions Workflows

#### `build-maplibre-native.yml`
Builds MapLibre Native for all platforms:
- Triggers: manual, or when maplibre-native-sys changes
- Matrix: macOS (arm64, x86_64), Linux (x86_64, arm64)
- Outputs: `libmbgl-core.a`, `libmlt-cpp.a`, etc.
- Caches: Using GitHub Actions cache

#### `release-native-libs.yml`
Publishes pre-built libraries:
- Triggers: tag `maplibre-native-sys-v*`
- Creates GitHub Release with tarballs:
  - `maplibre-native-libs-0.3.0-aarch64-apple-darwin.tar.gz`
  - `maplibre-native-libs-0.3.0-x86_64-unknown-linux-gnu.tar.gz`
  - etc.

### Phase 4: Update build.rs for Pre-built Downloads

```rust
fn download_prebuilt() -> Result<PathBuf, Error> {
    let version = env!("CARGO_PKG_VERSION");
    let target = env::var("TARGET")?;
    let url = format!(
        "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/\
         maplibre-native-sys-v{version}/maplibre-native-libs-{version}-{target}.tar.gz"
    );
    // Download and extract...
}
```

### Phase 5: Publishing to crates.io

Order matters due to dependencies:

1. `cargo publish -p tileserver-core`
2. Build and release native libs (tag `maplibre-native-sys-v0.3.0`)
3. `cargo publish -p maplibre-native-sys`
4. `cargo publish -p tileserver-rs`

## CI/CD Changes Summary

| Workflow | Purpose | Trigger |
|----------|---------|---------|
| `ci-rust.yml` | Lint, test, build all crates | PR, push to main |
| `build-maplibre-native.yml` | Build native libs for all platforms | Manual, changes to maplibre-native-sys |
| `release-native-libs.yml` | Publish pre-built libs to GitHub Releases | Tag `maplibre-native-sys-v*` |
| `release-crates.yml` (NEW) | Publish crates to crates.io | Tag `v*` |
| `release-*.yml` | Build binary releases | Tag `tileserver-rs-v*` |

## Version Strategy

- `tileserver-core`: Starts at 0.1.0, follows semver
- `maplibre-native-sys`: Bumps to 0.3.0 (adds prebuilt feature)
- `tileserver-rs`: Bumps to 0.3.0 (new architecture)

## User Experience

After publishing:

```bash
# Install CLI with native rendering (downloads pre-built libs)
cargo install tileserver-rs

# Use as library (no native deps)
cargo add tileserver-core

# Use with native rendering
cargo add tileserver-core
cargo add maplibre-native-sys --features prebuilt
```
