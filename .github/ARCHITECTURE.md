# Release Architecture

This document describes the build and release architecture for tileserver-rs.

## Single Binary Design

tileserver-rs is distributed as a **single self-contained binary**, similar to [martin](https://maplibre.org/martin/) and [tileserver-gl](https://tileserver.readthedocs.io/). The web UI (Nuxt SPA) is embedded directly into the Rust binary at compile time using `rust-embed`.

```
┌─────────────────────────────────────────┐
│           tileserver-rs binary          │
├─────────────────────────────────────────┤
│  ┌───────────────┐  ┌────────────────┐  │
│  │  Tile Server  │  │  Embedded SPA  │  │
│  │    (Axum)     │  │    (Nuxt)      │  │
│  │               │  │                │  │
│  │  /health      │  │  /             │  │
│  │  /data.json   │  │  /styles/:id   │  │
│  │  /data/:id/.. │  │  /data/:id     │  │
│  └───────────────┘  └────────────────┘  │
└─────────────────────────────────────────┘
         Single port (default: 8080)
```

## Supported Platforms

| Platform | Architecture | Runner | Target Triple | Status |
|----------|-------------|--------|---------------|--------|
| macOS | ARM64 (Apple Silicon) | `macos-14` | `aarch64-apple-darwin` | ✅ |
| macOS | x86_64 (Intel) | `macos-13` | `x86_64-apple-darwin` | 🚧 Planned |
| Linux | x86_64 | `ubuntu-latest` | `x86_64-unknown-linux-gnu` | 🚧 Planned |
| Linux | ARM64 | `ubuntu-latest` + cross | `aarch64-unknown-linux-gnu` | 🚧 Planned |
| Windows | x86_64 | `windows-latest` | `x86_64-pc-windows-msvc` | 🚧 Planned |
| Windows | ARM64 | `windows-latest` | `aarch64-pc-windows-msvc` | 🚧 Planned |

## Build Process

### 1. Generate Nuxt Static SPA

The frontend is generated as a static SPA (`ssr: false` in nuxt.config.ts):

```bash
bun install --frozen-lockfile
bun run --filter @tileserver-rs/client generate
```

Output: `apps/client/.output/public/` (static HTML + JS bundles)

See: [Nuxt Static Hosting](https://nuxt.com/docs/getting-started/deployment#static-hosting)

### 2. Build Rust Binary

The Rust binary embeds the SPA at compile time via `rust-embed`:

```rust
#[derive(Embed)]
#[folder = "apps/client/.output/public"]
struct Assets;
```

```bash
cargo build --release --target <target-triple>
```

Release profile optimizations (`Cargo.toml`):
- `lto = true` - Link-time optimization
- `codegen-units = 1` - Single codegen unit
- `opt-level = 3` - Maximum optimization
- `strip = true` - Strip debug symbols

### 3. Package

The final artifact is just the binary:

```bash
# macOS/Linux
tileserver-rs-macos-arm64.tar.gz
tileserver-rs-macos-arm64.tar.gz.sha256

# Windows
tileserver-rs-windows-x64.zip
tileserver-rs-windows-x64.zip.sha256
```

## Usage

```bash
# Download and extract
curl -LO https://github.com/.../tileserver-rs-macos-arm64.tar.gz
tar -xzf tileserver-rs-macos-arm64.tar.gz

# Run (UI enabled by default)
./tileserver-rs-macos-arm64 --config config.toml

# Run without UI (API only)
./tileserver-rs-macos-arm64 --no-ui --config config.toml
```

## CLI Options

```
tileserver-rs [OPTIONS]

Options:
  -c, --config <FILE>   Path to configuration file
      --host <HOST>     Host to bind to [env: TILESERVER_HOST]
  -p, --port <PORT>     Port to bind to [env: TILESERVER_PORT]
      --ui              Enable the web UI (default: true)
      --no-ui           Disable the web UI
  -v, --verbose         Enable verbose logging
  -h, --help            Print help
  -V, --version         Print version
```

## Workflow Files

| Workflow | Purpose | Trigger |
|----------|---------|---------|
| `release-macos-arm64.yml` | macOS Apple Silicon | Tags `v*`, manual |
| `release-macos-x64.yml` | macOS Intel | 🚧 Planned |
| `release-linux-x64.yml` | Linux x86_64 | 🚧 Planned |
| `release-linux-arm64.yml` | Linux ARM64 | 🚧 Planned |
| `release-windows-x64.yml` | Windows x86_64 | 🚧 Planned |
| `release-windows-arm64.yml` | Windows ARM64 | 🚧 Planned |
| `release.yml` | Unified multi-platform | 🚧 Planned |

## GitHub Actions Runners

### macOS
- **macos-14**: M1/M2 (Apple Silicon) - ARM64 native
- **macos-13**: Intel - x86_64 native

### Linux
- **ubuntu-latest**: x86_64 native
- For ARM64: Use cross-compilation or self-hosted ARM runners

### Windows
- **windows-latest**: x86_64 native
- For ARM64: Cross-compilation from x86_64

## Version Tagging

Release triggers on tags matching `v*`:

- `v0.1.0` - Stable release
- `v0.1.0-beta.1` - Pre-release (marked on GitHub)
- `v0.1.0-rc.1` - Release candidate

## Manual Releases

Trigger via `workflow_dispatch`:

1. Go to Actions tab
2. Select release workflow
3. Click "Run workflow"
4. Enter version tag

## Cross-Compilation

### Linux ARM64 from x86_64

```bash
rustup target add aarch64-unknown-linux-gnu
apt-get install gcc-aarch64-linux-gnu
```

### Windows from macOS/Linux

```bash
rustup target add x86_64-pc-windows-msvc
# Requires cargo-xwin or cross
```

## Comparison with Similar Tools

| Feature | tileserver-rs | martin | tileserver-gl |
|---------|--------------|--------|---------------|
| Single binary | ✅ | ✅ | ❌ (Node.js) |
| Embedded UI | ✅ | ✅ | ✅ |
| PMTiles | ✅ | ✅ | ❌ |
| MBTiles | ✅ | ✅ | ✅ |
| Language | Rust | Rust | JavaScript |

## Future Improvements

- [ ] Unified release workflow (all platforms in parallel)
- [ ] Automatic changelog generation
- [ ] Code signing (macOS notarization, Windows signing)
- [ ] Homebrew formula
- [ ] APT/RPM packages
- [ ] MSI/PKG installers
