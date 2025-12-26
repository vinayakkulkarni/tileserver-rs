# CLAUDE.md - Tileserver RS Development Guide

> **For AI Assistants (Claude Code, Cursor, etc.)**
> This file helps AI understand the codebase architecture, conventions, and best practices for tileserver-rs.

---

## Project Overview

**tileserver-rs** is a high-performance vector tile server built in Rust with a Nuxt 4 frontend. It serves vector tiles from PMTiles and MBTiles sources with native MapLibre rendering for raster tile generation.

### Key Capabilities
- PMTiles and MBTiles tile serving from local files
- HTTP-based PMTiles serving (remote files)
- **Native MapLibre GL rendering** via FFI bindings to MapLibre Native (C++)
- Raster tile generation (PNG/JPEG/WebP) from vector styles
- Static map image generation (like Mapbox Static API)
- TileJSON 3.0 metadata API
- MapLibre GL JS map viewer
- Style JSON and data inspector
- Configurable via TOML configuration

---

## Tech Stack & Architecture

### Backend (Rust)
- **Axum 0.8** - Web framework
- **Tokio** - Async runtime
- **Tower-HTTP** - Middleware (CORS, compression, tracing)
- **Serde** - Serialization
- **Tracing** - Structured logging
- **Clap** - CLI argument parsing
- **maplibre-native-sys** - FFI bindings to MapLibre Native C++ for server-side rendering

### Frontend (Nuxt 4)
- **Nuxt 4** (v3.15) - Vue 3.5 framework with `app/` directory structure
- **Tailwind CSS v4** - Utility-first styling with `@tailwindcss/vite`
- **shadcn-vue** - UI components (configured at `app/components/ui/`)
- **MapLibre GL JS v4** - Map rendering
- **@maplibre/maplibre-gl-inspect** - Tile inspector
- **VueUse** - Vue composition utilities

### Infrastructure
- **Bun workspaces** - Monorepo package management
- **Docker** - Containerized deployment
- **Multi-stage builds** - Optimized image size

---

## ⛔ CRITICAL RULES - NEVER VIOLATE THESE

> **STOP AND READ BEFORE WRITING ANY CODE**
>
> These rules are **NON-NEGOTIABLE**. Violating them causes frustration and wasted time.

### 🚨 Rule #1: NEVER Define Types/Interfaces Inline in Vue Files

**NEVER define `interface` or `type` inside:**
- ❌ Vue components (`.vue` files)
- ❌ Composables (`composables/**/*.ts`)

**ALWAYS place types in the dedicated `app/types/` directory:**

```typescript
// ❌ WRONG - NEVER DO THIS
// app/pages/index.vue
interface StyleInfo {  // NO! Types don't belong in components!
  id: string;
  name: string;
}

// ❌ WRONG - NEVER DO THIS
// app/composables/useMapStyles.ts
export interface TileJSON { ... }  // NO! Types don't belong in composables!

// ✅ CORRECT - Types in dedicated files
// app/types/style.ts
export interface Style {
  id: string;
  name: string;
  url: string;
  version: number;
}

// Then import correctly:
import type { Style } from '~/types/style';
```

### 🚨 Rule #2: Use Existing Packages - Don't Reinvent

**Before writing custom code, CHECK if a package already exists:**

```typescript
// ❌ WRONG - Inline SVG strings when @nuxt/icon or lucide-vue-next exists
const icon = '<svg xmlns="http://www.w3.org/2000/svg">...</svg>';

// ✅ CORRECT - Use Lucide icons
import { MapPin, Layers, Settings } from 'lucide-vue-next'
<MapPin class="size-4" />
```

### 🚨 Rule #3: No `any` Type - Ever

```typescript
// ❌ WRONG
const data: any = response;
function process(input: any): any { ... }

// ✅ CORRECT
const data: TileJSON = response;
function process(input: TileJSON): ProcessedData { ... }
// If truly unknown, use `unknown` and narrow with type guards
```

### 🚨 Rule #4: Composables Export Functions, Not Types

Composable files should **ONLY** export functions. Types are imported from type files.

```typescript
// ❌ WRONG - app/composables/useMapStyles.ts
export type Style = '...';  // NO! Types don't belong here
export interface TileJSON { ... }  // NO!
export function useMapStyles() { ... }

// ✅ CORRECT - app/composables/useMapStyles.ts
import type { Style, TileJSON } from '~/types';
export function useMapStyles() { ... }  // Only export functions
```

### 🚨 Rule #5: Component Naming - Don't Duplicate Folder Prefix

**Nuxt auto-imports components with folder path as prefix. Don't repeat it in filenames.**

```
components/
└── map/           ← folder name becomes prefix "Map"
    └── Controls.vue      ← filename becomes suffix "Controls"

    Result: <MapControls />   ✅ Clean!
```

**WRONG - Redundant naming:**
```
components/
└── map/
    └── MapControls.vue       → <MapMapControls />  ❌ "Map" appears twice!
```

**CORRECT - Clean naming:**
```
components/
└── map/
    └── Controls.vue          → <MapControls />     ✅
    └── Viewer.vue            → <MapViewer />       ✅
```

### 🚨 Rule #6: Vue Components Must Be Under 100 Lines

**Vue component files (`.vue`) should NOT exceed ~100 lines of code.**

When a component grows too large:
1. **Extract sub-components** into the same feature folder
2. **Move logic to composables** (`composables/useFeature.ts`)
3. **Move constants to composables** (not in `.vue` files)

### 🚨 Rule #7: No Inline Arrow Functions in Vue Templates

**Never use inline arrow functions with multiple parameters in Vue templates.**

```vue
<!-- ❌ WRONG - Inline arrow function in template -->
<MapViewer
  @layer-toggle="(layerId, visible) => emit('toggle-visibility', layerId, visible)"
/>

<!-- ✅ CORRECT - Named function in script setup -->
<script setup>
function toggleLayerVisibility(layerId: string, visible: boolean) {
  emit('toggle-visibility', layerId, visible);
}
</script>

<template>
  <MapViewer @layer-toggle="toggleLayerVisibility" />
</template>
```

### 🚨 Rule #8: Use VueUse Utilities - Don't Reinvent Helpers

**VueUse provides SSR-safe, reactive utilities. Use them instead of writing custom code:**

```typescript
// ❌ WRONG - Manual event listener with cleanup
const handler = (e: KeyboardEvent) => { ... };
onMounted(() => window.addEventListener('keydown', handler));
onUnmounted(() => window.removeEventListener('keydown', handler));

// ✅ CORRECT - Use VueUse's useEventListener (auto-cleanup)
import { useEventListener } from '@vueuse/core';
useEventListener('keydown', (e: KeyboardEvent) => { ... });
```

### 🚨 Rule #9: Prefer `computed` Over `watch`

**Avoid using `watch` for derived state. Use `computed` instead:**

```typescript
// ❌ WRONG - Using watch for derived state
const state = ref('expanded');
watch(open, (newValue) => {
  state.value = newValue ? 'expanded' : 'collapsed';
}, { immediate: true });

// ✅ CORRECT - Use computed for derived state
const state = computed(() => open.value ? 'expanded' : 'collapsed');
```

### 🚨 Rule #10: Use Tailwind's `size-*` Utility - NEVER `w-N h-N`

```vue
<!-- ❌ WRONG - Outdated pattern -->
<Icon class="w-4 h-4" />
<div class="h-8 w-8 rounded-full" />

<!-- ✅ CORRECT - Use size-* utility -->
<Icon class="size-4" />
<div class="size-8 rounded-full" />
```

### 🚨 Rule #11: Use Script Setup with defineComponent Only When Necessary

**Prefer `<script setup>` over Options API with `defineComponent`:**

```vue
<!-- ❌ WRONG - Options API style (verbose) -->
<script lang="ts">
export default defineComponent({
  name: 'MyPage',
  setup() {
    const data = ref([]);
    return { data };
  },
});
</script>

<!-- ✅ CORRECT - Composition API with script setup -->
<script setup lang="ts">
const data = ref([]);
</script>
```

---

## Project Structure

```
tileserver-rs/
├── apps/
│   ├── client/                        # Nuxt 4 frontend
│   │   ├── app/                       # Nuxt 4 app directory
│   │   │   ├── app.vue                # Root component
│   │   │   ├── pages/                 # File-based routing
│   │   │   │   ├── index.vue          # Home page (styles + data listing)
│   │   │   │   ├── styles/[style].vue # Style map viewer
│   │   │   │   └── data/[data].vue    # Data inspector
│   │   │   ├── components/
│   │   │   │   └── ui/                # shadcn-vue components
│   │   │   ├── composables/           # Vue composables
│   │   │   │   ├── useDataSource.ts   # Single data source fetching
│   │   │   │   ├── useDataSources.ts  # All data sources listing
│   │   │   │   ├── useMapStyle.ts     # Single style fetching
│   │   │   │   └── useMapStyles.ts    # All styles listing
│   │   │   ├── types/                 # Frontend TypeScript types
│   │   │   │   ├── index.ts           # Barrel export
│   │   │   │   ├── data.ts            # Data/TileJSON types
│   │   │   │   └── style.ts           # Style types
│   │   │   ├── assets/
│   │   │   │   └── css/
│   │   │   │       └── tailwind.css   # Tailwind CSS v4 entry point
│   │   │   └── lib/
│   │   │       └── utils.ts           # shadcn-vue cn() utility
│   │   ├── public/                    # Static assets
│   │   ├── components.json            # shadcn-vue configuration
│   │   ├── nuxt.config.ts             # Nuxt configuration
│   │   └── package.json               # @tileserver-rs/client
│   │
│   └── docs/                          # Docus v3 documentation (planned)
│       └── package.json               # @tileserver-rs/docs
│
├── maplibre-native-sys/               # FFI bindings to MapLibre Native
│   ├── cpp/                           # C/C++ wrapper code
│   │   ├── maplibre_c.h               # C API header
│   │   ├── maplibre_c.cpp             # C++ implementation wrapping mbgl::*
│   │   └── maplibre_c_stub.c          # Stub for development without native libs
│   ├── src/lib.rs                     # Rust FFI bindings
│   ├── build.rs                       # Build script (links MapLibre Native)
│   └── vendor/maplibre-native/        # MapLibre Native C++ source (git submodule)
│
├── src/                               # Rust backend
│   ├── main.rs                        # Server entry point, routes
│   ├── cli.rs                         # CLI argument parsing
│   ├── config.rs                      # TOML configuration
│   ├── error.rs                       # Error types
│   ├── cache_control.rs               # Cache headers middleware
│   ├── render/                        # Native MapLibre rendering
│   │   ├── mod.rs                     # Module exports
│   │   ├── native.rs                  # Safe Rust wrappers around FFI
│   │   ├── pool.rs                    # Renderer pool (per scale factor)
│   │   ├── renderer.rs                # High-level render API
│   │   └── types.rs                   # RenderOptions, ImageFormat, etc.
│   ├── styles/                        # Style management
│   │   └── mod.rs                     # Style loading + rewrite_style_for_native()
│   └── sources/                       # Tile source implementations
│       ├── mod.rs                     # TileSource trait, TileMetadata, TileJSON
│       ├── manager.rs                 # SourceManager (loads and manages sources)
│       ├── pmtiles.rs                 # PMTiles source
│       └── mbtiles.rs                 # MBTiles source
│
├── Cargo.toml                         # Rust dependencies
├── config.example.toml                # Example configuration
├── package.json                       # Root workspace (bun workspaces)
├── Dockerfile                         # Multi-stage Docker build
├── compose.yml                        # Docker Compose v2 base config
├── compose.override.yml               # Development overrides
├── compose.prod.yml                   # Production config
└── CLAUDE.md                          # This file
```

---

## Rust Backend Conventions

### 1. Error Handling - Use Custom Error Types

```rust
// ✅ CORRECT - Use TileServerError
use crate::error::{Result, TileServerError};

async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
    if z > self.metadata.maxzoom {
        return Ok(None); // Tile not found is not an error
    }
    // ...
}

// ❌ WRONG - Don't use anyhow in library code
async fn get_tile(&self, z: u8, x: u32, y: u32) -> anyhow::Result<Option<TileData>> { ... }
```

### 2. Configuration - Use config.rs Types

```rust
// ✅ CORRECT - Type-safe configuration
let config = Config::load(cli.config)?;
let sources = SourceManager::from_configs(&config.sources).await?;

// ❌ WRONG - Hardcoded values
let source = PmTilesSource::from_file("/data/tiles.pmtiles").await?;
```

### 3. API Response - Use Consistent JSON Structure

```rust
// ✅ CORRECT - TileJSON 3.0 spec
#[derive(Serialize)]
pub struct TileJson {
    pub tilejson: String,      // Always "3.0.0"
    pub tiles: Vec<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub minzoom: u8,
    pub maxzoom: u8,
    pub bounds: Option<[f64; 4]>,
    pub center: Option<[f64; 3]>,
    // ...
}
```

### 4. Async Trait - Use `#[async_trait]`

```rust
use async_trait::async_trait;

#[async_trait]
pub trait TileSource: Send + Sync {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>>;
    fn metadata(&self) -> &TileMetadata;
}
```

---

## Frontend Conventions

### 1. File-Based Routing

```
app/pages/
├── index.vue              → /
├── styles/[style].vue     → /styles/:style
└── data/[data].vue        → /data/:data
```

### 2. Composables Pattern

```typescript
// app/composables/useMapStyles.ts
export async function useMapStyles() {
  const { data } = await useFetch<Style[]>('/styles.json');
  return { styles: data };
}
```

### 3. Type-Safe Fetch

```typescript
// ✅ CORRECT - Type the response
const { data } = await useFetch<TileJSON>(`/data/${id}.json`);

// ❌ WRONG - Untyped with cast
const { data } = await useFetch(`/data/${id}.json`);
const tileJSON = data.value as TileJSON; // BAD!
```

### 4. MapLibre Integration

```vue
<script setup lang="ts">
import maplibregl from 'maplibre-gl';
import type { Map, StyleSpecification } from 'maplibre-gl';

const mapRef = ref<HTMLDivElement | null>(null);
let map: Map | null = null;

onMounted(() => {
  if (!mapRef.value) return;

  map = new maplibregl.Map({
    container: mapRef.value,
    style: styleSpec,
    center: [0, 0],
    zoom: 2,
    hash: true,
  });
});

onUnmounted(() => {
  map?.remove();
});
</script>

<template>
  <div ref="mapRef" class="size-full" />
</template>
```

---

## Configuration Format (config.toml)

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

[[styles]]
id = "osm-bright"
path = "/data/styles/osm-bright/style.json"
```

---

## API Endpoints

### Data Endpoints (Vector Tiles)

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check |
| `GET /data.json` | List all tile sources |
| `GET /data/{source}.json` | TileJSON for a source |
| `GET /data/{source}/{z}/{x}/{y}.{format}` | Get a vector tile |

### Style Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /styles.json` | List all styles |
| `GET /styles/{style}/style.json` | Get style JSON |

### Raster Rendering Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /styles/{style}/{z}/{x}/{y}.{format}` | Raster tile (PNG/JPEG/WebP) |
| `GET /styles/{style}/{z}/{x}/{y}@{scale}x.{format}` | Retina raster tile |
| `GET /styles/{style}/static/{lon},{lat},{zoom}/{width}x{height}.{format}` | Static image by center |
| `GET /styles/{style}/static/{minx},{miny},{maxx},{maxy}/{width}x{height}.{format}` | Static image by bounds |

---

## Native MapLibre Rendering Architecture

The project uses **MapLibre Native** (C++) for server-side raster tile generation, similar to tileserver-gl. This provides fast rendering (~100-800ms per tile) compared to browser-based approaches.

### Architecture

```
tileserver-rs (main binary)
    └── src/render/
        ├── renderer.rs  (high-level API)
        ├── pool.rs      (renderer pooling by scale factor)
        ├── native.rs    (safe Rust wrappers)
        └── types.rs     (RenderOptions, ImageFormat, etc.)
    
maplibre-native-sys (FFI crate)
    ├── src/lib.rs       (unsafe FFI declarations)
    ├── cpp/maplibre_c.h (C API header)
    ├── cpp/maplibre_c.cpp (C++ implementation using mbgl::*)
    └── vendor/maplibre-native/ (C++ library source)
        └── build-macos-metal/ (compiled .a files)
```

### Key Components

1. **maplibre-native-sys** - Rust crate providing FFI bindings to MapLibre Native
2. **Renderer Pool** - Maintains pools of native renderers per scale factor (1x, 2x, 3x)
3. **Style Rewriter** - Converts relative source URLs to absolute tile URLs for native rendering

### Style Rewriting

The native renderer cannot fetch TileJSON from our server (same process), so styles are rewritten before rendering:

```rust
// Before: style references TileJSON endpoint
"sources": {
  "protomaps": {
    "type": "vector",
    "url": "/data/protomaps.json"
  }
}

// After: style has inline tile URLs
"sources": {
  "protomaps": {
    "type": "vector",
    "tiles": ["http://localhost:8080/data/protomaps/{z}/{x}/{y}.pbf"]
  }
}
```

### Building MapLibre Native (macOS)

```bash
cd maplibre-native-sys/vendor/maplibre-native
git submodule update --init --recursive
brew install ninja ccache libuv glfw bazelisk
cmake --preset macos-metal
cmake --build build-macos-metal --target mbgl-core mlt-cpp -j8
```

### Performance

- **Warm cache**: ~100ms per tile
- **Cold cache**: ~700-800ms per tile (includes remote tile fetching)
- **Static images**: ~3s for 800x600 (depends on tile count)

---

## Development Commands

### Root (Workspace)
```bash
bun install              # Install all dependencies
bun run dev:client       # Start Nuxt dev server
bun run build:client     # Build Nuxt for production
bun run lint             # Lint all packages
```

### Rust Backend
```bash
cargo check              # Type check
cargo build              # Debug build
cargo build --release    # Release build
cargo run -- --config config.toml  # Run server
```

### Docker (planned)
```bash
docker compose up        # Start with Docker
docker compose build     # Rebuild images
```

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level (error, warn, info, debug, trace) | `info` |
| `CONFIG_PATH` | Path to config.toml | `config.toml` |
| `HOST` | Server host | `0.0.0.0` |
| `PORT` | Server port | `8080` |

---

## Cargo Features

```toml
[features]
default = []
http = ["reqwest"]      # HTTP PMTiles support
# s3 = ["aws-sdk-s3"]   # S3 PMTiles support (planned)
```

---

## Git Commit Message Format

Follow conventional commits:

```
type(scope): description

feat(sources): add PMTiles HTTP backend support
fix(api): handle empty tile responses correctly
docs(readme): update configuration examples
chore(deps): upgrade axum to 0.8
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

---

## Code Review Checklist

Before merging:
- [ ] No `any` types in TypeScript
- [ ] No inline types in Vue components
- [ ] Components under 100 lines
- [ ] Composables only export functions
- [ ] Uses `size-*` instead of `w-N h-N`
- [ ] Proper error handling (Result types in Rust)
- [ ] No hardcoded configuration values
- [ ] Types defined in `app/types/` directory
