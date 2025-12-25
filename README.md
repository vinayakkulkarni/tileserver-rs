# tileserver-rs 🦀

[![CI Pipeline](https://github.com/vinayakkulkarni/tileserver-rs/actions/workflows/pipeline.yml/badge.svg)](https://github.com/vinayakkulkarni/tileserver-rs/actions/workflows/pipeline.yml)
[![Docker](https://github.com/vinayakkulkarni/tileserver-rs/actions/workflows/docker.yml/badge.svg)](https://github.com/vinayakkulkarni/tileserver-rs/actions/workflows/docker.yml)

<img src="./.github/assets/tileserver-rs.png" width="512" height="512" align="center" alt="tileserver-rs logo" />

High-performance vector tile server built in Rust with a modern Nuxt 4 frontend.

## Features

- **PMTiles Support** - Serve tiles from local and remote PMTiles archives
- **MBTiles Support** - Serve tiles from SQLite-based MBTiles files
- **Raster Tile Rendering** - Generate PNG/JPEG/WebP tiles from vector styles
- **Static Map Images** - Create embeddable map screenshots (like Mapbox/Maptiler)
- **TileJSON 3.0** - Full TileJSON metadata API
- **MapLibre GL JS** - Built-in map viewer and data inspector
- **Docker Ready** - Easy deployment with Docker Compose v2 (includes Chromium)
- **Fast** - Built in Rust with Axum for maximum performance

## Tech Stack

- **Backend**: Rust 1.75+, Axum 0.8, Tokio
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
- [Development](#development)
- [Contributing](#contributing)
- [Author](#author)

## Requirements

- [Rust 1.75+](https://www.rust-lang.org/)
- [Bun 1.0+](https://bun.sh/)
- **Chrome/Chromium** - Required for raster tile rendering and static image generation
  - macOS: `brew install --cask chromium`
  - Linux: `apt-get install chromium` or `dnf install chromium`
  - Windows: Download from [chromium.org](https://www.chromium.org/getting-involved/download-chromium/)
- (Optional) [Docker](https://www.docker.com/) - Chromium is pre-installed in the Docker image

## Quick Start

```bash
# Using Docker
docker compose up -d

# Or build from source
cargo build --release
./target/release/tileserver-rs --config config.toml
```

## Installation

### Using Docker

**Development (with local data directory):**

```bash
# Start the tileserver
docker compose up -d

# View logs
docker compose logs -f tileserver

# Stop
docker compose down
```

**Production:**

```bash
# Start with production configuration
docker compose -f compose.yml -f compose.prod.yml up -d

# Or use pre-built image
docker run -d \
  -p 8080:8080 \
  -v /path/to/tiles:/data:ro \
  -v /path/to/config.toml:/app/config.toml:ro \
  ghcr.io/vinayakkulkarni/tileserver-rs:latest
```

### Building from Source

```bash
# Clone the repository
git clone https://github.com/vinayakkulkarni/tileserver-rs.git
cd tileserver-rs

# Install dependencies
bun install

# Build the Rust backend
cargo build --release

# Build the frontend
bun run build:client

# Run the server
./target/release/tileserver-rs --config config.toml
```

## Configuration

Create a `config.toml` file:

```toml
[server]
host = "0.0.0.0"
port = 8080
cors_origins = ["*"]

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
```

See [config.example.toml](./config.example.toml) for a complete example.

## API Endpoints

### Data Endpoints (Vector Tiles)

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check |
| `GET /data.json` | List all tile sources |
| `GET /data/{source}.json` | TileJSON for a source |
| `GET /data/{source}/{z}/{x}/{y}.{format}` | Get a vector tile (`.pbf`, `.mvt`) |

### Style Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /styles.json` | List all styles |
| `GET /styles/{style}/style.json` | Get MapLibre GL style JSON |

### Rendering Endpoints (Requires Chromium)

| Endpoint | Description |
|----------|-------------|
| `GET /styles/{style}/{z}/{x}/{y}[@{scale}x].{format}` | Raster tile (PNG/JPEG/WebP) |
| `GET /styles/{style}/static/{type}/{size}[@{scale}x].{format}` | Static map image |

**Raster Tile Examples:**
```
/styles/protomaps-light/14/8192/5461.png          # 512x512 PNG @ 1x
/styles/protomaps-light/14/8192/5461@2x.webp      # 512x512 WebP @ 2x (retina)
```

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

## Development

```bash
# Install dependencies
bun install

# Start Rust backend (with hot reload via cargo-watch)
cargo watch -x run

# Start Nuxt frontend (in another terminal)
bun run dev:client

# Run linters
bun run lint
cargo clippy

# Build for production
cargo build --release
bun run build:client
```

### Project Structure

```
tileserver-rs/
├── apps/
│   ├── client/          # Nuxt 4 frontend
│   └── docs/            # Documentation site
├── src/                 # Rust backend
│   ├── main.rs          # Entry point, routes
│   ├── config.rs        # Configuration
│   ├── error.rs         # Error types
│   ├── render/          # Chromium-based rendering (NEW!)
│   │   ├── pool.rs      # Browser pool management
│   │   ├── renderer.rs  # MapLibre GL rendering engine
│   │   └── types.rs     # Rendering types & options
│   ├── sources/         # Tile source implementations
│   └── styles/          # Style management
├── compose/             # Docker Compose modules
├── compose.yml          # Base compose config
├── compose.override.yml # Development overrides
├── compose.prod.yml     # Production config
├── Dockerfile           # Multi-stage Docker build (includes Chromium)
└── config.example.toml  # Example configuration
```

## Contributing

1. Fork it ([https://github.com/vinayakkulkarni/tileserver-rs/fork](https://github.com/vinayakkulkarni/tileserver-rs/fork))
2. Create your feature branch (`git checkout -b feat/new-feature`)
3. Commit your changes (`git commit -Sam 'feat: add feature'`)
4. Push to the branch (`git push origin feat/new-feature`)
5. Create a new [Pull Request](https://github.com/vinayakkulkarni/tileserver-rs/compare)

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
