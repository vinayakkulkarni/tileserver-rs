# tileserver-rs 🦀

[![CI Pipeline](https://github.com/vinayakkulkarni/tileserver-rs/actions/workflows/pipeline.yml/badge.svg)](https://github.com/vinayakkulkarni/tileserver-rs/actions/workflows/pipeline.yml)
[![Docker](https://github.com/vinayakkulkarni/tileserver-rs/actions/workflows/release-docker-images.yml/badge.svg)](https://github.com/vinayakkulkarni/tileserver-rs/actions/workflows/release-docker-images.yml)
[![Coverage](https://img.shields.io/codecov/c/github/vinayakkulkarni/tileserver-rs?style=flat-square&logo=codecov)](https://codecov.io/gh/vinayakkulkarni/tileserver-rs)
[![mbgl-sys on crates.io](https://img.shields.io/crates/v/mbgl-sys.svg)](https://crates.io/crates/mbgl-sys)
[![mbgl-sys on docs.rs](https://docs.rs/mbgl-sys/badge.svg)](https://docs.rs/mbgl-sys)

[![Deploy on Railway](https://railway.com/button.svg)](https://railway.com/deploy/1u-LMi)
[![Deploy on Render](https://render.com/images/deploy-to-render-button.svg)](https://render.com/deploy?repo=https://github.com/vinayakkulkarni/tileserver-rs)
[![Deploy to DO](https://www.deploytodo.com/do-btn-blue.svg)](https://cloud.digitalocean.com/apps/new?repo=https://github.com/vinayakkulkarni/tileserver-rs/tree/main)

<img src="./.github/assets/tileserver-rs.png" width="512" height="512" align="center" alt="tileserver-rs logo" />

High-performance vector tile server built in Rust with a modern Nuxt 4 frontend.

## Features

- **PMTiles Support** - Serve tiles from local and remote PMTiles archives
- **MBTiles Support** - Serve tiles from SQLite-based MBTiles files
- **Native Raster Rendering** - Generate PNG/JPEG/WebP tiles using MapLibre Native (C++ FFI)
- **MLT (MapLibre Tiles)** - Serve and transcode MLT tiles with MLT↔MVT on-the-fly conversion (feature-gated)
- **PostgreSQL Out-DB Rasters** - Serve VRT/COG tiles via PostGIS functions with dynamic filtering
- **OGC API Features** - PostGIS tables as OGC-spec collections: read, filter with CQL2, transform CRS, write (CRUD), and introspect schemas
- **STAC Catalog Sources** - Serve COGs directly from any STAC API (static, dynamic per-tile bbox, multi-asset mosaic)
- **Static Map Images** - Create embeddable map screenshots (like Mapbox/Maptiler Static API)
- **Zero-Config Auto-Detect** - Point at a directory or file and start serving instantly
- **Hot Reload** - Reload configuration via `SIGHUP` signal or admin API without downtime
- **High Performance** - ~100ms per tile (warm cache), ~800ms cold cache
- **TileJSON 3.0** - Full TileJSON metadata API
- **MapLibre GL JS** - Built-in map viewer and data inspector
- **Docker Ready** - Easy deployment with Docker Compose v2
- **Fast** - Built in Rust with Axum for maximum performance

## Tech Stack

- **Backend**: Rust 1.75+, Axum 0.8, Tokio
- **Native Rendering**: MapLibre Native (C++) via FFI bindings
- **Frontend**: Nuxt 4, Vue 3.5, Tailwind CSS v4, shadcn-vue
- **Tooling**: Bun workspaces, Docker multi-stage builds

## Table of Contents

- [Features](#features)
- [Tech Stack](#tech-stack)
- [Requirements](#requirements)
- [Quick Start](#quick-start)
- [Installation](#installation)
  - [Using Docker](#using-docker)
  - [Building from Source](#building-from-source)
- [Configuration](#configuration)
- [API Endpoints](#api-endpoints)
- [Deploy](#deploy)
- [Development](#development)
- [Contributing](#contributing)
- [Author](#author)

## Requirements

- [Rust 1.75+](https://www.rust-lang.org/)
- [Node.js 24+](https://nodejs.org/) with [pnpm 11](https://pnpm.io/) (via `corepack enable`)
- (Optional) [Docker](https://www.docker.com/)

### For Native Rendering (Optional)

Native raster tile rendering requires building MapLibre Native. If you don't need raster tiles, the server runs without it (stub implementation returns placeholder images).

**macOS (Apple Silicon/Intel):**
```bash
# Install build dependencies
brew install ninja ccache libuv glfw bazelisk cmake

# Build MapLibre Native
cd crates/mbgl-sys/vendor/maplibre-native
git submodule update --init --recursive
cmake --preset macos-metal
cmake --build build-macos-metal --target mbgl-core mlt-cpp -j8
```

**Linux:**
```bash
# Install build dependencies (Ubuntu/Debian)
apt-get install ninja-build ccache libuv1-dev libglfw3-dev cmake

# Build MapLibre Native
cd crates/mbgl-sys/vendor/maplibre-native
git submodule update --init --recursive
cmake --preset linux
cmake --build build-linux --target mbgl-core mlt-cpp -j8
```

**After building MapLibre Native:**
```bash
# Clear Cargo's cached build to detect the new libraries
cd /path/to/tileserver-rs
rm -rf target/release/build/mbgl-sys-*
cargo build --release
```

You should see `Building with real MapLibre Native renderer` in the build output.

## Quick Start

```bash
# Zero-config: point at a directory of tile files
./tileserver-rs /path/to/data

# Or with an explicit config file
./tileserver-rs --config config.toml

# Using Docker
docker compose up -d
```

The server auto-detects `.pmtiles`, `.mbtiles`, `style.json`, fonts, and GeoJSON files from the given path. See the [Auto-Detect Guide](https://docs.tileserver.app/guides/auto-detect) for details.

## Installation

### Using Homebrew (macOS)

```bash
# Add the tap and install
brew tap vinayakkulkarni/tileserver-rs https://github.com/vinayakkulkarni/tileserver-rs
brew install vinayakkulkarni/tileserver-rs/tileserver-rs

# Run the server
tileserver-rs --config config.toml
```

### Pre-built Binaries

Download the latest release from [GitHub Releases](https://github.com/vinayakkulkarni/tileserver-rs/releases).

| Platform | Architecture | Download |
|----------|--------------|----------|
| macOS | Apple Silicon (ARM64) | `tileserver-rs-aarch64-apple-darwin.tar.gz` |
| macOS | Intel (x86_64) | `tileserver-rs-x86_64-apple-darwin.tar.gz` |
| Linux | x86_64 | `tileserver-rs-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | ARM64 | `tileserver-rs-aarch64-unknown-linux-gnu.tar.gz` |

```bash
# macOS ARM64 (Apple Silicon)
curl -L https://github.com/vinayakkulkarni/tileserver-rs/releases/latest/download/tileserver-rs-aarch64-apple-darwin.tar.gz | tar xz
chmod +x tileserver-rs

# Remove macOS quarantine (required for unsigned binaries)
xattr -d com.apple.quarantine tileserver-rs

# Linux x86_64
curl -L https://github.com/vinayakkulkarni/tileserver-rs/releases/latest/download/tileserver-rs-x86_64-unknown-linux-gnu.tar.gz | tar xz
chmod +x tileserver-rs

# Run
./tileserver-rs --config config.toml
```

> **macOS Security Note:** If you download via a browser, macOS Gatekeeper will block the unsigned binary. Either use the `curl` command above, or after downloading, run `xattr -d com.apple.quarantine <binary>` to remove the quarantine flag. Alternatively, right-click the binary in Finder and select "Open".

### Using Docker

```bash
# Local development (builds locally, mounts ./data directory)
docker compose -f deploy/local/compose.yml up -d

# Local development with PostGIS sidecar (for OGC API + STAC + features)
docker compose -f deploy/local/compose.yml -f deploy/local/compose.override.yml up -d

# Production (uses pre-built image with resource limits)
docker compose -f deploy/prod/compose.yml -f deploy/prod/compose.override.yml up -d

# View logs
docker compose -f deploy/local/compose.yml logs -f tileserver

# Stop
docker compose -f deploy/local/compose.yml down
```

**Or run directly with Docker:**

```bash
docker run -d \
  -p 8080:8080 \
  -v /path/to/data:/data:ro \
  -v /path/to/config.toml:/app/config.toml:ro \
  ghcr.io/vinayakkulkarni/tileserver-rs:latest
```

### Building from Source

```bash
# Clone the repository with submodules
git clone --recursive git@github.com:vinayakkulkarni/tileserver-rs.git
cd tileserver-rs

# Or using HTTPS
git clone --recursive https://github.com/vinayakkulkarni/tileserver-rs.git

# If you already cloned without --recursive:
git submodule update --init --recursive

# Install dependencies
pnpm install

# Build the Rust backend
cargo build --release

# Build with MLT transcoding support (optional)
cargo build --release --features mlt

# Build the frontend
pnpm run build:client

# Run the server
./target/release/tileserver-rs --config config.toml
```

> **Note:** The `--recursive` flag fetches the MapLibre Native submodule (~200MB) required for native raster rendering. If the clone times out, use `git submodule update --init --depth 1` for a shallow clone. See [CONTRIBUTING.md](./CONTRIBUTING.md) for detailed setup instructions.

## Configuration

Create a `config.toml` file. **Important:** Root-level options (`fonts`, `files`) must come before any `[section]` headers:

```toml
# Root-level options (must come BEFORE [sections])
fonts = "/data/fonts"
files = "/data/files"

[server]
host = "0.0.0.0"
port = 8080
cors_origins = ["*", "https://example.com"]  # Supports multiple origins
# Admin server bind address for hot-reload endpoint (default: disabled)
# admin_bind = "127.0.0.1:9099"

[telemetry]
enabled = false                         # OTLP traces + metrics push (default: false)
# endpoint = "http://localhost:4317"    # OTLP gRPC collector
# metrics_enabled = true                # OTLP metrics push (requires enabled = true)
# prometheus_bind = "127.0.0.1:9100"    # Optional: separate Prometheus /metrics scrape listener
# metrics_label_cardinality = "strict"  # "strict" (default) | "verbose"

[[sources]]
id = "openmaptiles"
type = "pmtiles"
path = "/data/tiles.pmtiles"
name = "OpenMapTiles"
attribution = "© OpenMapTiles © OpenStreetMap contributors"

[[sources]]
id = "terrain"
type = "mbtiles"
path = "/data/terrain.mbtiles"
name = "Terrain Data"

[[styles]]
id = "osm-bright"
path = "/data/styles/osm-bright/style.json"

# PostgreSQL Out-of-Database Rasters (optional)
[postgres]
connection_string = "postgresql://user:pass@localhost:5432/gis"

[[postgres.outdb_rasters]]
id = "imagery"                    # Also used as function name if 'function' is omitted
schema = "public"
# function = "get_raster_paths"   # Optional: defaults to 'id' value
name = "Satellite Imagery"
```

### Admin Server (Hot Reload)

Enable the admin server by setting `admin_bind` in `[server]`:

```toml
[server]
admin_bind = "127.0.0.1:9099"
```

This exposes `POST /__admin/reload` on a separate port for reloading configuration without restarting the server. You can also send `SIGHUP` to the process for the same effect. See the [Hot Reload Guide](https://docs.tileserver.app/guides/hot-reload) for details.

### Observability (Metrics & Tracing)

tileserver-rs exposes runtime metrics two ways — both off by default, both independent of each other:

- **Prometheus pull**: a separate HTTP listener (default port disabled) that exposes `GET /metrics` in Prometheus text format. Recommended for production scraping.
- **OTLP push**: traces + metrics pushed to an OpenTelemetry collector (e.g. Grafana Alloy, OTel Collector, Jaeger).

```toml
[telemetry]
# Prometheus pull endpoint (separate listener, scoped to your monitoring network):
prometheus_bind = "127.0.0.1:9100"
prometheus_path = "/metrics"
metrics_label_cardinality = "strict"   # "strict" = production-safe; "verbose" = debug only

# OTLP push pipeline (independent of prometheus_bind above):
enabled = true
endpoint = "http://localhost:4317"
service_name = "tileserver-rs"
sample_rate = 1.0
```

> **Why a separate port?** Following the [official Axum example](https://github.com/tokio-rs/axum/blob/main/examples/prometheus-metrics/src/main.rs) and projects like Opengist, the metrics endpoint runs on a dedicated listener so you can bind it to a private interface (e.g. `127.0.0.1` or a VPC-internal address) while keeping `/data`, `/styles`, and `/__admin` behind their own ACLs.

**Exported metrics (Prometheus naming):**

| Metric | Type | Unit | Labels | Description |
|---|---|---|---|---|
| `http_requests_total` | counter | — | `route`, `status_class` | HTTP requests by matched route and 1xx/2xx/3xx/4xx/5xx |
| `http_request_duration_seconds` | histogram | seconds | `route`, `status_class` | Per-request latency |
| `http_requests_in_flight` | up-down | — | — | Currently in-flight HTTP requests |
| `tile_requests_total` | counter | — | `source`, `format`, `z_bucket`, `outcome` | Tile lookups by hit/miss/not_found/error |
| `tile_request_duration_seconds` | histogram | seconds | `source`, `format`, `z_bucket`, `outcome` | End-to-end tile latency |
| `tile_request_bytes` | histogram | bytes | `source`, `format` | Tile response payload size |
| `tile_cache_hits_total` | counter | — | `source` | In-process tile cache hits |
| `tile_cache_misses_total` | counter | — | `source` | In-process tile cache misses |
| `render_duration_seconds` | histogram | seconds | `style`, `format` | Native MapLibre raster render duration |
| `render_errors_total` | counter | — | `style`, `reason` | Native render failures bucketed by reason |

**Cardinality matters.** The `metrics_label_cardinality` knob exists because [unbounded labels blow up Prometheus storage](https://prometheus.io/docs/practices/naming/):

- **`strict`** (default) — `z_bucket` collapses zoom levels into `low` (0–6), `mid` (7–12), `high` (13+). Tile coordinates (`x`, `y`) are dropped entirely. Bounded label combinations safe for long-term retention.
- **`verbose`** — passes raw zoom 0..=22 through. Useful for short-window investigations; **do not leave enabled in production** — a globe-scale source at z=22 has trillions of unique tile coordinates.

**Scrape config example:**

```yaml
# prometheus.yml
scrape_configs:
  - job_name: tileserver-rs
    scrape_interval: 15s
    static_configs:
      - targets: ['tileserver-rs.internal:9100']
```

When `prometheus_bind` is unset (the default), the listener task is never spawned and the per-request cost is a single atomic load — no Prometheus, no overhead. See the [Telemetry Guide](https://docs.tileserver.app/guides/telemetry) for Grafana/Tempo/Jaeger backend setup.

See [data/configs/example.toml](./data/configs/example.toml) for a complete example, or [data/configs/offline.toml](./data/configs/offline.toml) for a local development setup.

## API Endpoints

### Health & Admin Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check (returns `OK`) |
| `GET /ping` | Runtime metadata (config hash, loaded sources/styles, version) |
| `GET /metrics` | Prometheus metrics exposition (separate listener — see [Observability](#observability-metrics--tracing)) |
| `POST /__admin/reload` | Hot-reload configuration (admin server only) |
| `POST /__admin/reload?flush=true` | Force reload even if config unchanged |

### Data Endpoints (Vector Tiles)

| Endpoint | Description |
|----------|-------------|
| `GET /data.json` | List all tile sources |
| `GET /data/{source}.json` | TileJSON for a source |
| `GET /data/{source}/{z}/{x}/{y}.{format}` | Get a vector tile (`.pbf`, `.mvt`, `.mlt`) |
| `GET /data/{source}/{z}/{x}/{y}.geojson` | Get tile as GeoJSON (for debugging) |

### Style Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /styles.json` | List all styles |
| `GET /styles/{style}/style.json` | Get MapLibre GL style JSON |
| `GET /styles/{style}/sprite[@2x].{png,json}` | Get sprite image/metadata |
| `GET /styles/{style}/wmts.xml` | WMTS capabilities (for QGIS/ArcGIS) |

### Font Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /fonts.json` | List available font families |
| `GET /fonts/{fontstack}/{range}.pbf` | Get font glyphs (PBF format) |

### Other Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /files/{filepath}` | Serve static files (GeoJSON, icons, etc.) |
| `GET /index.json` | Combined TileJSON for all sources and styles |

### PostgreSQL Out-DB Raster Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /data/{outdb_source}/{z}/{x}/{y}.{format}` | Raster tile from PostgreSQL-referenced VRT/COG |
| `GET /data/{outdb_source}/{z}/{x}/{y}.{format}?satellite=...` | With dynamic filtering via query params |

### Rendering Endpoints (Native MapLibre)

| Endpoint | Description |
|----------|-------------|
| `GET /styles/{style}/{z}/{x}/{y}[@{scale}x].{format}` | Raster tile (PNG/JPEG/WebP) |
| `GET /styles/{style}/static/{type}/{size}[@{scale}x].{format}` | Static map image |

**Raster Tile Examples:**
```
/styles/protomaps-light/14/8192/5461.png          # 512x512 PNG @ 1x
/styles/protomaps-light/14/8192/5461@2x.webp      # 1024x1024 WebP @ 2x (retina)
```

**Performance:**
- Warm cache: ~100ms per tile
- Cold cache: ~700-800ms per tile (includes tile fetching)
- Static images: ~3s for 800x600

**Static Image Types:**
- **Center**: `{lon},{lat},{zoom}[@{bearing}[,{pitch}]]`
  ```
  /styles/protomaps-light/static/-122.4,37.8,12/800x600.png
  /styles/protomaps-light/static/-122.4,37.8,12@45,60/800x600@2x.webp
  ```
- **Bounding Box**: `{minx},{miny},{maxx},{maxy}`
  ```
  /styles/protomaps-light/static/-123,37,-122,38/1024x768.jpeg
  ```
- **Auto-fit**: `auto` (with `?path=` or `?marker=` query params)
  ```
  /styles/protomaps-light/static/auto/800x600.png?path=path-5+f00(-122.4,37.8|-122.5,37.9)
  ```

**Static Image Limits:**
- Maximum dimensions: 4096x4096 pixels
- Maximum scale: 4x

## Development

```bash
# Install dependencies
pnpm install

# Start Rust backend (with hot reload via cargo-watch)
cargo watch -x run

# Start Nuxt frontend (in another terminal)
pnpm run dev:client

# Start marketing site (landing page)
pnpm run dev:marketing

# Run linters
pnpm run lint
cargo clippy

# Build for production
cargo build --release
pnpm run build:client
```

### Cargo Feature Flags

| Feature | Description |
|---------|-------------|
| `http` | Enable serving PMTiles from remote HTTP URLs |
| `mlt` | Enable MLT (MapLibre Tiles) transcoding support |

```bash
# Build with MLT support
cargo build --release --features mlt

# Build with all optional features
cargo build --release --features http,mlt
```

### Project Structure

```
tileserver-rs/
├── apps/
│   ├── client/              # Nuxt 4 frontend (embedded in binary)
│   ├── docs/                # Documentation site (docs.tileserver.app)
│   └── marketing/           # Landing page (tileserver.app)
├── benchmarks/              # Load-test harness vs titiler/martin/tileserver-gl
├── crates/
│   ├── tileserver-rs/       # Binary crate (Rust backend)
│   │   ├── src/             # main.rs, admin, config, render, sources, …
│   │   ├── benches/         # criterion benchmarks (mlt, cache, raster)
│   │   └── tests/           # integration tests
│   └── mbgl-sys/            # FFI bindings to MapLibre Native (C++)
│       ├── cpp/             # C wrapper over mbgl::*
│       ├── src/lib.rs       # Rust FFI bindings
│       └── vendor/maplibre-native/  # MapLibre Native source (submodule)
├── data/                    # Runtime assets + configs
│   ├── configs/             # tileserver-rs config files (TOML)
│   │   ├── example.toml     # Reference configuration
│   │   ├── offline.toml     # Offline/local development
│   │   ├── dev.toml         # Development config
│   │   ├── dev-postgres.toml # Development with PostGIS + OGC API
│   │   ├── geoparquet.toml  # GeoParquet source testing
│   │   ├── duckdb.toml      # DuckDB source testing
│   │   └── benchmark-raster.toml # Raster benchmark config
│   ├── styles/              # MapLibre GL style JSONs
│   ├── tiles/, fonts/, raster/, overture/, postgres-dev/
├── deploy/                  # Deployment manifests
│   ├── compose.yml          # Docker Compose (base)
│   ├── compose.dev.yml      # Docker Compose (development overrides)
│   ├── compose.prod.yml     # Docker Compose (production overrides)
│   └── docker-entrypoint.sh # Container entrypoint
├── homebrew/                # Homebrew formula (tileserver-rs.rb)
├── Cargo.toml               # Virtual workspace manifest
├── Cargo.lock
├── Dockerfile               # Multi-stage build (Rust + Node + MapLibre)
├── fly.toml, railway.toml, render.yaml  # One-click platform configs
└── release-please-config.json + .release-please-manifest.json
```

## Deploy

### One-Click Cloud Deploy

Deploy a fully working tileserver-rs instance with sample data in minutes. No configuration needed — sample tile data is automatically downloaded on first start.

| Platform | Deploy | Notes |
|----------|--------|-------|
| **Render** | [![Deploy on Render](https://render.com/images/deploy-to-render-button.svg)](https://render.com/deploy?repo=https://github.com/vinayakkulkarni/tileserver-rs) | Uses `render.yaml` blueprint |
| **DigitalOcean** | [![Deploy to DO](https://www.deploytodo.com/do-btn-blue.svg)](https://cloud.digitalocean.com/apps/new?repo=https://github.com/vinayakkulkarni/tileserver-rs/tree/main) | Uses `.do/deploy.template.yaml` |
| **Railway** | [![Deploy on Railway](https://railway.com/button.svg)](https://railway.com/deploy/1u-LMi) | Uses `railway.toml` config |
| **Fly.io** | `fly launch --copy-config` | Uses `fly.toml` — see below |
| **Docker** | `docker compose -f deploy/local/compose.yml up -d` | See `deploy/README.md` for local/prod variants |

### How Sample Data Works

When the Docker container starts with an empty `/data` directory (no tile files mounted), it automatically downloads sample data (~15 MB) from the [latest GitHub release](https://github.com/vinayakkulkarni/tileserver-rs/releases). This includes:

- **Protomaps sample tiles** (PMTiles) — world basemap extract
- **Zurich MBTiles** — detailed city extract
- **Noto Sans fonts** — for label rendering
- **Protomaps Light style** — ready-to-use map style
- **Sample raster data** — COG test files

To use your own data, mount a volume at `/data`:

```bash
docker run -d -p 8080:8080 -v /path/to/your/data:/data:ro ghcr.io/vinayakkulkarni/tileserver-rs:latest
```

Set `SAMPLE_DATA_VERSION=v2.12.1` to pin a specific release version instead of `latest`.

### Deploy on Fly.io

```bash
# Install flyctl: https://fly.io/docs/flyctl/install/
fly launch --copy-config
fly deploy
```

The included `fly.toml` configures auto-stop/start machines, health checks, and 512MB RAM. Add a persistent volume for your own tile data:

```bash
fly volumes create tile_data --size 10 --region iad
```

## Deployments

### Documentation Site (docs.tileserver.app)

The docs site is deployed automatically via Cloudflare Pages (linked repo). Any changes to `docs/` trigger a rebuild.

### Marketing Site (tileserver.app)

The marketing/landing page is deployed via GitHub Actions to a separate CF Pages project.

**Setup (one-time):**

1. Create a new CF Pages project named `tileserver-marketing` (Direct Upload, not linked to repo)
2. Add custom domain `tileserver.app` to the project
3. Add these secrets to GitHub repo settings:
   - `CLOUDFLARE_API_TOKEN` - API token with "Cloudflare Pages: Edit" permission
   - `CLOUDFLARE_ACCOUNT_ID` - Your Cloudflare account ID

Deployments are triggered on push to `main` when files in `marketing/` change.

## Releases

This project uses [release-please](https://github.com/googleapis/release-please) for automated releases based on [Conventional Commits](https://www.conventionalcommits.org/).

**How it works:**
1. Commits to `main` with conventional commit messages (`feat:`, `fix:`, etc.) trigger release-please to create/update a **Release PR**
2. The Release PR contains version bumps in `Cargo.toml`, `apps/client/package.json`, `homebrew/Formula/tileserver-rs.rb`, and changelog updates
3. Merging the Release PR pushes a `vX.Y.Z` tag, which triggers the Linux/macOS binary, Docker image, sample-data, and Homebrew formula workflows

**Version bumping:**
- `feat:` commits → minor version (0.1.0 → 0.2.0)
- `fix:` commits → patch version (0.1.0 → 0.1.1)
- `feat!:` or `BREAKING CHANGE:` → major version (0.1.0 → 1.0.0)

**Release artifacts:**
- GitHub Release with changelog
- macOS ARM64 binary (`.tar.gz`)
- Linux x86_64 + ARM64 binaries — full (with MapLibre Native) and headless variants
- Docker image (`ghcr.io/vinayakkulkarni/tileserver-rs`) — multi-arch `linux/amd64` + `linux/arm64`
- Homebrew formula auto-update

### Known release race: retry Docker after mbgl-sys build

When release-please merges a Release PR that bumps **both** `tileserver-rs` AND `mbgl-sys` (our two-package workspace), both tags (`v2.x.y` and `mbgl-sys-v0.1.z`) push simultaneously. This triggers `build-mbgl-native.yml` and `release-docker-images.yml` in parallel — but the Docker workflow starts with a pre-flight `Verify mbgl-sys assets exist` step that 404s because `build-mbgl-native` hasn't finished uploading release assets yet (build takes 10–15 min).

**Symptom**: Release Docker Images run fails within seconds, job `Verify mbgl-sys assets exist` = failure, all downstream build jobs = skipped.

**Fix**: wait for `Build MapLibre Native` run to conclude success, then rerun the failed Docker workflow:

```bash
gh run list --workflow release-docker-images.yml --limit 1 --json databaseId --jq '.[].databaseId' \
  | xargs -I{} gh run rerun {}
```

The pre-flight gate is **intentional and load-bearing** — it prevents shipping a Docker image that points at non-existent mbgl-sys binaries. Do not remove it. Upstream fix blocked on release-please not supporting cross-tag `workflow_run` chaining (build-mbgl-native triggers on `mbgl-sys-v*`, Docker on `v*`).

Observed on v2.26.0, v2.26.2. Manual rerun is the idiomatic workaround.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for detailed guidelines.

**Quick Start:**

1. Fork it ([https://github.com/vinayakkulkarni/tileserver-rs/fork](https://github.com/vinayakkulkarni/tileserver-rs/fork))
2. Clone with submodules: `git clone --recursive <your-fork-url>`
3. Create your feature branch (`git checkout -b feat/new-feature`)
4. Commit your changes (`git commit -Sam 'feat: add feature'`)
5. Push to the branch (`git push origin feat/new-feature`)
6. Create a new [Pull Request](https://github.com/vinayakkulkarni/tileserver-rs/compare)

**Working with Git Submodules:**

```bash
# After cloning (if you forgot --recursive)
git submodule update --init --recursive

# After pulling changes from upstream
git pull
git submodule update --init --recursive

# If clone times out (shallow clone)
git submodule update --init --depth 1
```

**Notes:**

1. Please contribute using [GitHub Flow](https://guides.github.com/introduction/flow/)
2. Commits & PRs will be allowed only if the commit messages & PR titles follow the [conventional commit standard](https://www.conventionalcommits.org/)
3. Ensure your commits are signed. [Read why](https://withblue.ink/2020/05/17/how-and-why-to-sign-git-commits.html)

## Author

**tileserver-rs** © [Vinayak](https://vinayakkulkarni.dev), Released under the [MIT](./LICENSE) License.

Authored and maintained by Vinayak Kulkarni with help from contributors ([list](https://github.com/vinayakkulkarni/tileserver-rs/contributors)).

> [vinayakkulkarni.dev](https://vinayakkulkarni.dev) · GitHub [@vinayakkulkarni](https://github.com/vinayakkulkarni) · Twitter [@_vinayak_k](https://twitter.com/_vinayak_k)

### Special Thanks

- [tileserver-gl](https://github.com/maptiler/tileserver-gl) - Inspiration for this project
- [MapLibre](https://maplibre.org/) - Open-source mapping library
- [PMTiles](https://github.com/protomaps/PMTiles) - Cloud-optimized tile archive format
- [PostGIS](https://postgis.net/) - Spatial database extension for PostgreSQL
