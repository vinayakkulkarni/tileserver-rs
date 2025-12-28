# Contributing to tileserver-rs

Thank you for your interest in contributing to tileserver-rs! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Cloning the Repository](#cloning-the-repository)
  - [Setting Up MapLibre Native](#setting-up-maplibre-native)
- [Development Workflow](#development-workflow)
  - [Git Workflow](#git-workflow)
  - [Working with the Submodule](#working-with-the-submodule)
  - [Running Tests](#running-tests)
  - [Code Style](#code-style)
- [Submitting Changes](#submitting-changes)
  - [Commit Messages](#commit-messages)
  - [Pull Requests](#pull-requests)
- [Release Process](#release-process)
- [Project Structure](#project-structure)
- [Need Help?](#need-help)

## Code of Conduct

This project follows our [Code of Conduct](./CODE_OF_CONDUCT.md). By participating, you agree to uphold this code.

## Getting Started

### Prerequisites

- [Rust 1.75+](https://www.rust-lang.org/tools/install)
- [Bun 1.0+](https://bun.sh/)
- [Git](https://git-scm.com/)
- (Optional) [Docker](https://www.docker.com/)

For native rendering support, you'll also need:
- CMake 3.20+
- Ninja
- C++ compiler (clang or gcc)

### Cloning the Repository

```bash
# Clone with submodules (SSH - recommended for contributors)
git clone --recursive git@github.com:vinayakkulkarni/tileserver-rs.git
cd tileserver-rs

# Or using HTTPS
git clone --recursive https://github.com/vinayakkulkarni/tileserver-rs.git
cd tileserver-rs

# If you already cloned without --recursive, initialize submodules:
git submodule update --init --recursive
```

> **Note:** The `--recursive` flag fetches the MapLibre Native submodule (~200MB). If cloning times out, try a shallow clone:
> ```bash
> git clone git@github.com:vinayakkulkarni/tileserver-rs.git
> cd tileserver-rs
> git submodule update --init --depth 1
> ```

### Setting Up MapLibre Native

The `maplibre-native-sys/vendor/maplibre-native` directory contains the MapLibre Native C++ library as a Git submodule. This is required for native raster tile rendering.

**macOS (Apple Silicon/Intel):**
```bash
# Install dependencies
brew install ninja ccache libuv glfw bazelisk cmake

# Initialize submodule and build
cd maplibre-native-sys/vendor/maplibre-native
git submodule update --init --recursive
cmake --preset macos-metal
cmake --build build-macos-metal --target mbgl-core mlt-cpp -j$(sysctl -n hw.ncpu)
```

**Linux (Ubuntu/Debian):**
```bash
# Install dependencies
sudo apt-get install -y ninja-build ccache libuv1-dev libglfw3-dev cmake \
  libcurl4-openssl-dev libicu-dev libjpeg-dev libpng-dev libsqlite3-dev

# Initialize submodule and build
cd maplibre-native-sys/vendor/maplibre-native
git submodule update --init --recursive
cmake --preset linux
cmake --build build-linux --target mbgl-core mlt-cpp -j$(nproc)
```

**Note:** If you don't need native raster rendering, you can skip this step. The server will use a stub implementation that returns placeholder images.

**Important:** After building MapLibre Native, you must clear Cargo's build cache to detect the new libraries:

```bash
# Clear the cached build script output
rm -rf target/release/build/maplibre-native-sys-*

# Rebuild
cargo build --release
```

You should see `Building with real MapLibre Native renderer` instead of `using stub implementation`.

## Development Workflow

### Git Workflow

We use [GitHub Flow](https://guides.github.com/introduction/flow/):

1. Fork the repository
2. Create a feature branch from `main`
3. Make your changes
4. Submit a pull request

### Working with the Submodule

The MapLibre Native source code is managed as a Git submodule. Here's how to work with it:

#### After Cloning

```bash
# If you cloned without --recursive
git submodule update --init --recursive
```

#### After Pulling Changes

```bash
# Always update submodules after pulling
git pull
git submodule update --init --recursive
```

#### Updating the Submodule to a New Version

```bash
# Navigate to submodule directory
cd maplibre-native-sys/vendor/maplibre-native

# Fetch and checkout the desired version
git fetch origin
git checkout <tag-or-commit>

# Go back to root and commit the submodule update
cd ../../..
git add maplibre-native-sys/vendor/maplibre-native
git commit -m "chore: update maplibre-native to <version>"
```

#### Common Submodule Issues

**Detached HEAD in submodule:**
This is normal. Submodules are always checked out at a specific commit.

**Submodule has local changes:**
```bash
# Discard local changes in submodule
cd maplibre-native-sys/vendor/maplibre-native
git checkout .
git clean -fd
```

**Submodule out of sync:**
```bash
# Reset submodule to the commit tracked by the parent repo
git submodule update --init --recursive --force
```

**Clone timed out (large submodule):**
```bash
# Use shallow clone for the submodule
git submodule update --init --depth 1
```

**Submodule URL issues (SSH vs HTTPS):**
The submodule is configured to use SSH (`git@github.com:...`). If you need HTTPS:
```bash
# Temporarily override submodule URL
git config submodule.maplibre-native-sys/vendor/maplibre-native.url https://github.com/maplibre/maplibre-native.git
git submodule update --init --recursive
```

### Running Tests

```bash
# Run Rust tests
cargo test

# Run Rust linter
cargo clippy

# Check Rust formatting
cargo fmt --all -- --check

# Run frontend linter
bun run lint
```

### Code Style

**Rust:**
- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- No `unwrap()` in library code - use proper error handling

**Security:**
- Always validate user input (paths, coordinates, dimensions)
- Use path canonicalization to prevent directory traversal
- Reject `..`, `/`, and `\` in user-provided filenames
- Set reasonable limits on resource sizes (max 4096x4096 for images)
- Don't expose internal paths in error messages

**TypeScript/Vue:**
- Follow the ESLint configuration
- Use TypeScript strict mode
- See [CLAUDE.md](./CLAUDE.md) for detailed frontend conventions

## Submitting Changes

### Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/). Format:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation only
- `style` - Code style (formatting, semicolons, etc.)
- `refactor` - Code refactoring
- `test` - Adding or updating tests
- `chore` - Maintenance tasks

**Examples:**
```bash
feat(sources): add S3 PMTiles support
fix(render): handle empty tile responses
docs(readme): update configuration examples
chore(deps): upgrade axum to 0.8
```

**Important:** Commits must be signed. [Learn how to sign commits](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits).

### Pull Requests

1. **Title:** Use conventional commit format (e.g., `feat(sources): add S3 support`)
2. **Description:** Explain what and why, not just how
3. **Tests:** Add tests for new functionality
4. **Documentation:** Update docs if needed
5. **Breaking Changes:** Clearly mark and explain any breaking changes

**PR Checklist:**
- [ ] Code follows project style guidelines
- [ ] Tests pass locally (`cargo test`)
- [ ] Linters pass (`cargo clippy`, `cargo fmt --check`, `bun run lint`)
- [ ] Documentation updated (if applicable)
- [ ] Commit messages follow conventional commits
- [ ] Commits are signed

## Release Process

This project uses [Release Please](https://github.com/googleapis/release-please) for automated releases. You don't need to manually bump versions or write changelogs.

### How It Works

1. **Commit with conventional messages** - Use `feat:`, `fix:`, `docs:`, etc.
2. **Release Please creates a PR** - After merging to `main`, Release Please automatically creates/updates a Release PR with:
   - Version bump in `Cargo.toml`
   - Auto-generated `CHANGELOG.md`
   - Updated versions in `apps/client/package.json` and `homebrew/Formula/tileserver-rs.rb`
3. **Merge the Release PR** - When ready to release, merge the Release PR (use **regular merge**, not squash)
4. **Automated release** - Merging triggers:
   - GitHub Release creation with changelog
   - macOS ARM64 binary build
   - Docker image build and push
   - Homebrew formula update

### Version Bumping Rules

| Commit Type | Version Bump | Example |
|-------------|--------------|---------|
| `fix:` | Patch (0.1.0 → 0.1.1) | `fix(render): handle empty tiles` |
| `feat:` | Minor (0.1.0 → 0.2.0) | `feat(sources): add S3 support` |
| `feat!:` or `BREAKING CHANGE:` | Major (0.1.0 → 1.0.0) | `feat!: change config format` |

### Manual Release (if needed)

For maintainers who need to trigger a release manually:

```bash
# Trigger macOS ARM64 build for a specific tag
gh workflow run release-macos-arm64.yml --ref v0.2.0 -f version=v0.2.0
```

### Release Artifacts

Each release produces:
- **GitHub Release** - Changelog and release notes
- **macOS ARM64 binary** - `tileserver-rs-macos-arm64.tar.gz`
- **Docker image** - `ghcr.io/vinayakkulkarni/tileserver-rs:v0.x.x`
- **Homebrew formula** - Auto-updated in `homebrew/Formula/tileserver-rs.rb`

## Project Structure

```
tileserver-rs/
├── apps/
│   ├── client/                  # Nuxt 4 frontend
│   │   ├── app/                 # Nuxt app directory
│   │   │   ├── components/      # Vue components
│   │   │   ├── composables/     # Vue composables
│   │   │   ├── pages/           # File-based routing
│   │   │   └── types/           # TypeScript types
│   │   └── nuxt.config.ts
│   └── docs/                    # Documentation site
│
├── maplibre-native-sys/         # FFI bindings crate
│   ├── cpp/                     # C/C++ wrapper code
│   ├── src/lib.rs               # Rust FFI declarations
│   ├── build.rs                 # Build script
│   └── vendor/maplibre-native/  # MapLibre Native (submodule)
│
├── src/                         # Main Rust application
│   ├── main.rs                  # Entry point, HTTP routes
│   ├── config.rs                # TOML configuration
│   ├── error.rs                 # Error types
│   ├── render/                  # Native rendering
│   │   ├── pool.rs              # Renderer pool
│   │   ├── renderer.rs          # High-level API
│   │   └── native.rs            # Safe FFI wrappers
│   ├── sources/                 # Tile sources
│   │   ├── pmtiles/             # PMTiles (local + HTTP)
│   │   └── mbtiles.rs           # MBTiles
│   └── styles/                  # Style management
│
├── compose.yml                  # Docker Compose base
├── Dockerfile                   # Multi-stage build
├── Cargo.toml                   # Rust workspace
├── CLAUDE.md                    # AI assistant guidelines
└── CONTRIBUTING.md              # This file
```

## Need Help?

- **Questions:** Open a [Discussion](https://github.com/vinayakkulkarni/tileserver-rs/discussions)
- **Bugs:** Open an [Issue](https://github.com/vinayakkulkarni/tileserver-rs/issues)
- **Security:** See [SECURITY.md](./SECURITY.md) (if applicable)

---

Thank you for contributing to tileserver-rs!
