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
- **MLT (MapLibre Tiles) transcoding** — on-the-fly MLT↔MVT conversion (feature-gated)

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
- **mlt-core** - MLT tile parsing and decoding (optional, `mlt` feature)
- **prost** - Protobuf encoding for MVT tile generation (optional, `mlt` feature)
- **geo-types** - Geometry types used by mlt-core (optional, `mlt` feature)

### Frontend (Nuxt 4)
- **Nuxt 4** (v3.15) - Vue 3.5 framework with `app/` directory structure
- **Tailwind CSS v4** - Utility-first styling with `@tailwindcss/vite`
- **shadcn-vue** - UI components (configured at `app/components/ui/`)
- **MapLibre GL JS v4** - Map rendering
- **maplibre-gl-inspect** - Tile inspector
- **VueUse** - Vue composition utilities
- **TanStack AI Vue** - AI chat with `useChat` hook and `stream()` adapter
- **WebLLM** - Browser-local LLM inference via WebGPU (`@mlc-ai/web-llm`)
- **motion-v** - Vue animation library for UI transitions

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

### 🚨 Rule #6: Vue Components Are Thin Templates

**Vue component files (`.vue`) should NOT exceed ~100 lines of code.**

Components are **presentation only** — they destructure from composables and bind to the template. **ALL logic, state, and functions belong in composables.**

```vue
<!-- ❌ WRONG - Logic in the component -->
<script setup lang="ts">
const panelOpen = ref(true);

function togglePanel() {
  panelOpen.value = !panelOpen.value;
}

function navigateBack() {
  history.replaceState(null, '', window.location.pathname);
  navigateTo('/');
}
</script>

<!-- ✅ CORRECT - Destructure everything from composable -->
<script setup lang="ts">
const {
  mapOptions,
  panelOpen,
  navigateBack,
  togglePanel,
} = useDataInspector(dataId);
</script>
```

When a component grows too large:
1. **Move logic/state/functions to composables** (`composables/useFeature.ts`)
2. **Extract sub-components** into the same feature folder
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

### 🚨 Rule #12: Abstract `$fetch` to API Layer — NEVER Use `$fetch` Directly

**NEVER call `$fetch` directly in components or composables.** Always wrap API calls in `useQuery`/`useMutation` inside `utils/api/` files.

```
app/utils/
├── query-keys/               # Centralized query key constants
│   ├── data.ts
│   ├── styles.ts
│   └── index.ts
└── api/                      # API layer composables
    ├── data/
    │   └── queries.ts         # useQuery + $fetch for data sources
    ├── styles/
    │   └── queries.ts         # useQuery + $fetch for map styles
    └── upload/
        ├── use-upload-file.mutation.ts   # useMutation for file upload
        └── use-delete-upload.mutation.ts # useMutation for upload deletion
```

**Pattern:**
- `useQuery` wraps `$fetch` for **reads** (GET)
- `useMutation` wraps `$fetch` for **writes** (POST, PUT, DELETE)
- Components/composables ONLY call the hook — never `$fetch` directly

```typescript
// ❌ WRONG - $fetch in composable
export function useUploadFile() {
  async function upload(file: File) {
    const result = await $fetch('/api/upload', { method: 'POST', body: formData });
  }
}

// ✅ CORRECT - useMutation in utils/api/upload/
// utils/api/upload/use-upload-file.mutation.ts
export function useUploadFileMutation() {
  return useMutation({
    mutationFn: async (file: File) => {
      const formData = new FormData();
      formData.append('file', file);
      return $fetch<UploadResponse>('/api/upload', {
        method: 'POST',
        body: formData,
      });
    },
  });
}

// composable only calls the hook:
const uploadMutation = useUploadFileMutation();
await uploadMutation.mutateAsync(file);
```

### 🚨 Rule #13: Always Use Bun Workspace Catalogs for Dependencies

**NEVER hardcode dependency versions in workspace packages.** All versions are managed centrally in the root `package.json` catalogs.

```json
// Root package.json — versions defined HERE
{
  "workspaces": {
    "catalogs": {
      "default": { "vue": "^3.5.29", ... },
      "client": { "pmtiles": "^4.4.0", ... }
    }
  }
}

// ❌ WRONG - Hardcoded version in workspace package
// apps/client/package.json
{
  "dependencies": {
    "pmtiles": "^4.4.0"
  }
}

// ✅ CORRECT - Catalog reference in workspace package
// apps/client/package.json
{
  "dependencies": {
    "pmtiles": "catalog:client"
  }
}
```

**Which catalog to use:**
- `catalog:default` — Shared packages used across all workspace apps (vue, nuxt, tailwindcss, vueuse, etc.)
- `catalog:client` — Packages specific to `@tileserver-rs/client` (deck.gl, tanstack, maplibre-gl-inspect, etc.)
- `catalog:marketing` — Packages specific to `@tileserver-rs/marketing`

**When adding a NEW dependency:**
1. Add the version to the appropriate catalog in root `package.json`
2. Reference it as `"catalog:client"` (or `"catalog:default"`) in the workspace package
3. Run `bun install` to verify resolution

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
│   ├── docs/                          # Docus v3 documentation site
│   │   ├── content/                   # Markdown documentation files
│   │   ├── nuxt.config.ts             # Nuxt/Docus configuration
│   │   └── package.json               # @tileserver-rs/docs (excluded from workspace)
│   │
│   └── marketing/                     # Marketing landing page
│       ├── app/                       # Nuxt 4 app directory
│       ├── nuxt.config.ts             # Nuxt configuration
│       └── package.json               # @tileserver-rs/marketing (excluded from workspace)
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
│   ├── transcode.rs                   # MLT↔MVT transcoding (feature-gated `mlt`)
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

> **When modifying files under `src/`, follow the 179 Rust best practice rules in [`.claude/skills/rust-skills/SKILL.md`](.claude/skills/rust-skills/SKILL.md).**
> Priority: Ownership & Borrowing, Error Handling, Memory Optimization (CRITICAL) > API Design, Async, Compiler Optimization (HIGH) > the rest.

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

### Health & Admin Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check (returns `OK`) |
| `GET /ping` | Runtime metadata (config hash, loaded sources/styles, version) |
| `POST /__admin/reload` | Hot-reload configuration (admin server only) |

### Data Endpoints (Vector Tiles)

| Endpoint | Description |
|----------|-------------|
| `GET /data.json` | List all tile sources |
| `GET /data/{source}.json` | TileJSON for a source |
| `GET /data/{source}/{z}/{x}/{y}.{format}` | Get a vector tile (`.pbf`, `.mvt`, `.mlt`) |

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

## MLT (MapLibre Tiles) Transcoding Architecture

The `mlt` feature flag enables on-the-fly transcoding between MLT and MVT tile formats.

### Overview

MLT is a next-generation vector tile format from the MapLibre project, designed as a more efficient alternative to MVT (Mapbox Vector Tiles). tileserver-rs supports:

- **Phase 1** (Passthrough): Serve MLT tiles directly from PMTiles/MBTiles sources (always enabled)
- **Phase 2** (MVT→MLT): Not yet available — `mlt-core` v0.1.x lacks an encoding API
- **Phase 3** (MLT→MVT): Decode MLT tiles and re-encode as MVT protobuf for legacy clients

### Architecture

```
src/transcode.rs  (feature-gated: mlt)
├── MvtProto module       ─ Prost-derived MVT protobuf types (Tile, Layer, Feature, Value)
├── transcode_tile()      ─ Public API: dispatches format conversion
├── mlt_to_mvt()          ─ Phase 3: MLT → MVT using mlt-core + prost encoding
├── feature_collection_to_mvt() ─ Builds MVT Tile from mlt-core FeatureCollection
├── encode_geometry_to_mvt()   ─ Encodes geo_types geometries to MVT command sequences
└── decompress_tile_data()    ─ Handles gzip decompression of compressed tiles
```

### How Transcoding Works

When a client requests a tile in a different format than the source provides (e.g., requesting `.pbf` from an MLT source), the `get_tile` handler in `main.rs` detects the mismatch and calls `transcode_tile()`. The flow:

1. Detect source format vs requested format
2. Decompress tile data if gzip-compressed
3. Parse MLT tile using `mlt-core::parse_layers()` + `layer.decode_all()`
4. Convert to intermediate `FeatureCollection` via `FeatureCollection::from_layers()`
5. Build MVT protobuf using key/value interning, geometry encoding, and `prost` serialization
6. Return transcoded tile bytes with correct `Content-Type`

### Key Implementation Details

- Features are grouped by `_layer` property (injected by `from_layers()`)
- Geometry encoding uses standard MVT command sequences (MoveTo, LineTo, ClosePath)
- Coordinates are zigzag-encoded per the MVT spec
- Key/value interning deduplicates property strings per layer
- Fallback: if transcoding fails, the original tile is served with a warning log
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
mlt = ["mlt-core", "prost", "geo-types"]  # MLT transcoding support
# s3 = ["aws-sdk-s3"]   # S3 PMTiles support (planned)

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
- [ ] Frontend follows Laws of UX principles

---

## Laws of UX

Design principles to follow when building frontend components and interactions. Reference: [lawsofux.com](https://lawsofux.com/)

| #   | Law                              | Description                                                                        |
| --- | -------------------------------- | ---------------------------------------------------------------------------------- |
| 1   | **Aesthetic-Usability Effect**   | Users perceive aesthetically pleasing design as more usable                        |
| 2   | **Choice Overload**              | People get overwhelmed with too many options                                       |
| 3   | **Chunking**                     | Break information into meaningful groups                                           |
| 4   | **Cognitive Bias**               | Systematic errors in thinking influence perception and decisions                   |
| 5   | **Cognitive Load**               | Minimize mental resources needed to interact with an interface                     |
| 6   | **Doherty Threshold**            | Keep interactions under 400ms so neither user nor system waits                     |
| 7   | **Fitts's Law**                  | Time to reach a target depends on distance and size — make targets large and close |
| 8   | **Flow**                         | Design for full immersion — minimize interruptions                                 |
| 9   | **Goal-Gradient Effect**         | Motivation increases with proximity to a goal — show progress                      |
| 10  | **Hick's Law**                   | Decision time increases with number and complexity of choices                      |
| 11  | **Jakob's Law**                  | Users prefer your site to work like sites they already know                        |
| 12  | **Law of Common Region**         | Elements sharing a boundary are perceived as grouped                               |
| 13  | **Law of Proximity**             | Objects near each other are perceived as grouped                                   |
| 14  | **Law of Prägnanz**              | People interpret complex images as the simplest form possible                      |
| 15  | **Law of Similarity**            | Similar elements are perceived as a group                                          |
| 16  | **Law of Uniform Connectedness** | Visually connected elements are perceived as more related                          |
| 17  | **Mental Model**                 | Users carry expectations about how systems work                                    |
| 18  | **Miller's Law**                 | Working memory holds 7 (±2) items — chunk information accordingly                  |
| 19  | **Occam's Razor**                | Prefer the simplest solution with fewest assumptions                               |
| 20  | **Paradox of the Active User**   | Users never read manuals — they start using immediately                            |
| 21  | **Pareto Principle**             | 80% of effects come from 20% of causes — focus on high-impact work                 |
| 22  | **Parkinson's Law**              | Tasks expand to fill available time — set constraints                              |
| 23  | **Peak-End Rule**                | Experiences are judged by their peak moment and ending                             |
| 24  | **Postel's Law**                 | Be liberal in what you accept, conservative in what you send                       |
| 25  | **Selective Attention**          | Users focus on stimuli related to their goals                                      |
| 26  | **Serial Position Effect**       | First and last items in a series are remembered best                               |
| 27  | **Tesler's Law**                 | Every system has irreducible complexity — put it in the right place                |
| 28  | **Von Restorff Effect**          | The item that differs from the rest is most memorable                              |
| 29  | **Working Memory**               | Cognitive system that temporarily holds info for tasks                             |
| 30  | **Zeigarnik Effect**             | Incomplete tasks are remembered better than complete ones                          |

---

## Browser-Local LLM Chat Architecture

### Overview

The map viewer (`/styles/[style]`) includes an AI chat panel powered by a browser-local LLM via WebLLM. Users can talk to their maps — ask questions, fly to locations, change styles, and query features — all without any server-side AI infrastructure. The LLM runs entirely in the browser using WebGPU.

### File Structure

```
app/
├── components/llm/
│   ├── Panel.vue              # Chat panel (Sheet/drawer)
│   ├── MessageList.vue        # Message rendering with markdown
│   └── Input.vue              # Chat input with send/stop buttons
├── composables/
│   ├── use-llm-engine.ts      # WebLLM engine lifecycle (init, load, progress)
│   ├── use-llm-chat.ts        # TanStack AI useChat + stream() adapter + map tools
│   └── use-llm-panel.ts       # Panel state, input, auto-scroll, suggested prompts
└── types/llm.ts               # LLM types (model config, chat state, map tools)
```

### Key Packages

- `@tanstack/ai` - Core AI types and AG-UI protocol events
- `@tanstack/ai-vue` - Vue integration with `useChat` hook (re-exports `stream`, `ConnectionAdapter` from `@tanstack/ai-client`)
- `@mlc-ai/web-llm` - Browser-local LLM inference via WebGPU
- `zod` - Tool input schema validation

**IMPORTANT:** `@tanstack/ai-vue` re-exports everything needed from `@tanstack/ai-client` — do NOT import `@tanstack/ai-client` directly.

### Data Flow

```
User Input → LlmInput.vue
    ↓
useLlmChat (composable)
    ↓
useChat({ connection: stream(adapter) })
    ↓
stream() adapter — converts WebLLM output to AG-UI events
    ↓
WebLLM engine (browser-local, WebGPU)
    ├── chat.completions.create({ stream: true })
    └── Tool calls (fly_to, set_filter, etc.)
    ↓
AG-UI events stream back
    ↓
TanStack AI Vue updates messages
    ↓
LlmMessageList.vue (renders messages)
```

### Model Configuration

**WebLLM (browser-local, WebGPU):**
- Default Model: `Qwen2.5-3B-Instruct-q4f16_1-MLC` (best tool-calling at ~2GB)
- Lightweight: `Qwen2.5-1.5B-Instruct-q4f16_1-MLC` (~1GB)
- Alternative: `Llama-3.2-3B-Instruct-q4f16_1-MLC` (~2GB)

### Connection Adapter Pattern

TanStack AI Vue uses `stream()` to create a `ConnectionAdapter` from an async generator. This bridges WebLLM's OpenAI-compatible streaming API to AG-UI protocol events:

```typescript
import { useChat, stream } from '@tanstack/ai-vue';
import type { UIMessage } from '@tanstack/ai-vue';

// Create connection adapter from WebLLM
const connection = stream(async function* (messages: UIMessage[]) {
  const runId = crypto.randomUUID();
  const messageId = crypto.randomUUID();

  yield { type: 'RUN_STARTED', runId, timestamp: Date.now() };
  yield { type: 'TEXT_MESSAGE_START', messageId, role: 'assistant', timestamp: Date.now() };

  const openaiMessages = messages.map((m) => ({
    role: m.role,
    content: extractText(m),
  }));

  const response = await engine.chat.completions.create({
    messages: openaiMessages,
    stream: true,
  });

  for await (const chunk of response) {
    const delta = chunk.choices[0]?.delta?.content || '';
    if (delta) {
      yield { type: 'TEXT_MESSAGE_CONTENT', messageId, delta, timestamp: Date.now() };
    }
  }

  yield { type: 'TEXT_MESSAGE_END', messageId, timestamp: Date.now() };
  yield { type: 'RUN_FINISHED', runId, finishReason: 'stop', timestamp: Date.now() };
});

// Use in composable
const chat = useChat({ connection });
```

### AG-UI Event Types

The `stream()` adapter must yield AG-UI protocol events:

| Event Type | Fields | Purpose |
|---|---|---|
| `RUN_STARTED` | `runId` | Start of an LLM response |
| `TEXT_MESSAGE_START` | `messageId`, `role` | Begin assistant message |
| `TEXT_MESSAGE_CONTENT` | `messageId`, `delta` | Streamed text chunk |
| `TEXT_MESSAGE_END` | `messageId` | End of text message |
| `TOOL_CALL_START` | `toolCallId`, `toolName` | Begin tool invocation |
| `TOOL_CALL_ARGS` | `toolCallId`, `delta` | Streamed tool arguments |
| `TOOL_CALL_END` | `toolCallId` | End tool invocation |
| `RUN_FINISHED` | `runId`, `finishReason` | End of LLM response |

### Map Tools (Client-Side)

Map tools let the LLM interact with the MapLibre GL map instance:

| Tool | Description | Parameters |
|---|---|---|
| `fly_to` | Animate camera to location | `lng`, `lat`, `zoom?`, `bearing?`, `pitch?` |
| `set_layer_paint` | Change layer paint property | `layerId`, `property`, `value` |
| `query_rendered_features` | Query visible features | `point?`, `layers?`, `filter?` |
| `set_filter` | Set layer filter expression | `layerId`, `filter` |
| `fit_bounds` | Fit camera to bounding box | `bounds`, `padding?` |

### Type Organization

| Type | Location | Import Path |
|---|---|---|
| `LlmModelConfig` | `app/types/llm.ts` | `~/types/llm` |
| `LlmChatState` | `app/types/llm.ts` | `~/types/llm` |
| `LlmPanelState` | `app/types/llm.ts` | `~/types/llm` |
| `MapTool` | `app/types/llm.ts` | `~/types/llm` |
| `SuggestedPrompt` | `app/types/llm.ts` | `~/types/llm` |

### WebLLM Engine Lifecycle

```typescript
// 1. Initialize engine (download + compile model, ~30s first time)
const engine = await CreateMLCEngine(modelId, {
  initProgressCallback: (progress) => {
    // Update loading bar: progress.text, progress.progress (0-1)
  },
});

// 2. Chat completions (OpenAI-compatible API)
const stream = await engine.chat.completions.create({
  messages: [{ role: 'user', content: 'Hello' }],
  stream: true,
  tools: mapTools,  // Optional: enable tool calling
});

// 3. Tool calls arrive on the LAST streaming chunk only
// Check chunk.choices[0]?.delta?.tool_calls for tool invocations
```

### Key Implementation Notes

1. **No server required** — WebLLM runs entirely in-browser via WebGPU
2. **First load is slow** (~30s model download + compilation) — show progress bar
3. **Model is cached** in browser IndexedDB — subsequent loads are fast (~2-5s)
4. **Tool calls are NOT streamed** — they arrive on the final chunk only
5. **`@tanstack/ai-vue` re-exports** `stream`, `ConnectionAdapter`, `fetchServerSentEvents` from `@tanstack/ai-client` — never import `@tanstack/ai-client` directly
6. **Types go in `app/types/llm.ts`** — never define interfaces in composables or components
