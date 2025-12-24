# Release Architecture

This document describes the build and release architecture for tileserver-rs across all supported platforms.

## Supported Platforms

| Platform | Architecture | Runner | Target Triple | Bun Binary | Status |
|----------|-------------|--------|---------------|------------|--------|
| macOS | ARM64 (Apple Silicon) | `macos-14` | `aarch64-apple-darwin` | `bun-darwin-aarch64` | ✅ |
| macOS | x86_64 (Intel) | `macos-13` | `x86_64-apple-darwin` | `bun-darwin-x64` | 🚧 Planned |
| Linux | x86_64 | `ubuntu-latest` | `x86_64-unknown-linux-gnu` | `bun-linux-x64` | 🚧 Planned |
| Linux | ARM64 | `ubuntu-latest` + QEMU | `aarch64-unknown-linux-gnu` | `bun-linux-aarch64` | 🚧 Planned |
| Windows | x86_64 | `windows-latest` | `x86_64-pc-windows-msvc` | `bun-windows-x64` | 🚧 Planned |
| Windows | ARM64 | `windows-latest` | `aarch64-pc-windows-msvc` | `bun-windows-aarch64` | 🚧 Planned |

## Runtime Architecture

The release package runs two processes:

```
┌─────────────────────────────────────────────────────────────┐
│                      tileserver-rs                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────┐     ┌─────────────────────────┐   │
│  │   Rust Tile Server  │     │   Nuxt SSR Server       │   │
│  │   (tileserver-rs)   │     │   (Bun + Nitro)         │   │
│  │                     │     │                         │   │
│  │   Port: 8080        │     │   Port: 3000            │   │
│  │                     │     │                         │   │
│  │   - /health         │     │   - / (home)            │   │
│  │   - /data.json      │     │   - /styles/:id         │   │
│  │   - /data/:id.json  │     │   - /data/:id           │   │
│  │   - /data/:id/...   │     │   - SSR + hydration     │   │
│  └─────────────────────┘     └─────────────────────────┘   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                         start.sh
```

## Release Package Structure

Each release package contains:

```
tileserver-rs-{platform}-{arch}/
├── bin/
│   ├── tileserver-rs      # Rust tile server binary
│   └── bun                # Bun runtime for SSR
├── client/
│   └── .output/
│       ├── public/        # Static assets
│       └── server/
│           └── index.mjs  # Nitro SSR server entry
├── config.toml            # Example configuration
├── start.sh               # Launcher script (start.bat on Windows)
├── README.md              # Documentation
└── LICENSE                # License file
```

## Build Process

### 1. Nuxt Client Build

The frontend is built using Bun with SSR enabled:

```bash
bun install --frozen-lockfile
bun run --filter @tileserver-rs/client build
```

Output structure (`apps/client/.output/`):
- `public/` - Static assets served directly
- `server/index.mjs` - Nitro SSR server entry point

### 2. Rust Binary Build

The tile server is compiled with release optimizations:

```bash
cargo build --release --target <target-triple>
```

Release profile settings (from `Cargo.toml`):
- `lto = true` - Link-time optimization
- `codegen-units = 1` - Single codegen unit for better optimization
- `opt-level = 3` - Maximum optimization
- `strip = true` - Strip symbols for smaller binary

### 3. Bun Runtime Download

The standalone Bun binary is downloaded for the target platform:

```bash
# Example for macOS ARM64
curl -fsSL https://github.com/oven-sh/bun/releases/download/bun-v1.1.42/bun-darwin-aarch64.zip -o bun.zip
unzip bun.zip
```

### 4. Package Assembly

All components are combined into a distributable archive:
- **macOS/Linux**: `.tar.gz`
- **Windows**: `.zip`

## Launcher Script

The `start.sh` script manages both processes:

```bash
# Default ports (configurable via environment)
TILESERVER_PORT=8080  # Rust tile server
CLIENT_PORT=3000      # Nuxt SSR server

# Start both services
./start.sh

# Custom ports
TILESERVER_PORT=9000 CLIENT_PORT=4000 ./start.sh

# Custom config
CONFIG_FILE=/path/to/config.toml ./start.sh
```

Features:
- Starts both Rust and Nuxt servers
- Graceful shutdown on SIGINT/SIGTERM
- Environment variable configuration
- Auto-detects script directory for portability

## Workflow Files

| Workflow | Purpose | Trigger |
|----------|---------|---------|
| `release-macos-arm64.yml` | macOS Apple Silicon builds | Tags `v*`, manual dispatch |
| `release-macos-x64.yml` | macOS Intel builds | 🚧 Planned |
| `release-linux-x64.yml` | Linux x86_64 builds | 🚧 Planned |
| `release-linux-arm64.yml` | Linux ARM64 builds | 🚧 Planned |
| `release-windows-x64.yml` | Windows x86_64 builds | 🚧 Planned |
| `release-windows-arm64.yml` | Windows ARM64 builds | 🚧 Planned |
| `release.yml` | Unified release workflow | 🚧 Planned |

## GitHub Actions Runners

### macOS
- **macos-14**: M1/M2 (Apple Silicon) - ARM64 native
- **macos-13**: Intel - x86_64 native

### Linux
- **ubuntu-latest**: x86_64 native
- For ARM64: Use QEMU emulation or self-hosted ARM runners

### Windows
- **windows-latest**: x86_64 native
- For ARM64: Cross-compilation from x86_64

## Bun Runtime Versions

The Bun version is pinned in each workflow:

```yaml
env:
  BUN_VERSION: '1.1.42'
```

Bun release downloads: https://github.com/oven-sh/bun/releases

## Cross-Compilation Notes

### Linux ARM64 from x86_64
Requires cross-compilation toolchain:
```bash
rustup target add aarch64-unknown-linux-gnu
apt-get install gcc-aarch64-linux-gnu
```

### Windows ARM64 from x86_64
```bash
rustup target add aarch64-pc-windows-msvc
```

## Docker vs Native Binaries

| Approach | Pros | Cons |
|----------|------|------|
| Docker | Consistent environment, single container | Larger size, requires Docker |
| Native | Direct installation, no container runtime | Two processes, platform-specific |

This project provides both:
- **Docker**: For containerized deployments (see `Dockerfile`)
- **Native**: For direct installation with bundled Bun runtime

## Version Tagging

Release workflow triggers on tags matching `v*`:
- `v0.1.0` - Stable release
- `v0.1.0-beta.1` - Pre-release (marked as prerelease on GitHub)
- `v0.1.0-rc.1` - Release candidate

## Manual Releases

Workflows can be triggered manually via `workflow_dispatch`:

1. Go to Actions tab
2. Select the release workflow
3. Click "Run workflow"
4. Enter version tag (e.g., `v0.1.0`)

## Future Improvements

- [ ] Unified release workflow that builds all platforms in parallel
- [ ] Automatic changelog generation
- [ ] Code signing for macOS and Windows
- [ ] Homebrew formula for macOS
- [ ] APT/RPM packages for Linux
- [ ] MSI installer for Windows
- [ ] SHA256 checksums for all artifacts
- [ ] Single-port mode with Rust proxying to Nuxt
- [ ] Systemd/launchd service files
