# TODO - tileserver-rs

Feature parity tracking with [tileserver-gl](https://tileserver.readthedocs.io/en/latest/endpoints.html).

## Legend

- ✅ Implemented
- 🚧 In Progress
- ❌ Not Implemented

---

## API Endpoints

### Styles

| Endpoint | Status | Description |
|----------|--------|-------------|
| `GET /styles.json` | ✅ | List all available styles |
| `GET /styles/{id}/style.json` | ✅ | Get MapLibre GL style JSON |
| `GET /styles/{id}/sprite[@2x].{format}` | ✅ | Serve sprite images (png) and metadata (json) |
| `GET /fonts/{fontstack}/{start}-{end}.pbf` | ✅ | Serve font glyphs (PBF format) |

### Rendered Tiles (Raster)

| Endpoint | Status | Description |
|----------|--------|-------------|
| `GET /styles/{id}/{z}/{x}/{y}[@{scale}x].{format}` | ✅ | Raster tiles (PNG/JPEG/WebP) |
| `GET /styles/{id}/{tileSize}/{z}/{x}/{y}[@{scale}x].{format}` | ✅ | Variable tile size (256/512) |
| `GET /styles/{id}.json` | ✅ | TileJSON for raster tiles |

### WMTS

| Endpoint | Status | Description |
|----------|--------|-------------|
| `GET /styles/{id}/wmts.xml` | ✅ | WMTS capabilities document |

### Static Images

| Endpoint | Status | Description |
|----------|--------|-------------|
| `GET /styles/{id}/static/{lon},{lat},{zoom}[@{bearing}[,{pitch}]]/{width}x{height}[@{scale}x].{format}` | ✅ | Center-based static image |
| `GET /styles/{id}/static/{minx},{miny},{maxx},{maxy}/{width}x{height}[@{scale}x].{format}` | ✅ | Bounding box static image |
| `GET /styles/{id}/static/auto/{width}x{height}[@{scale}x].{format}` | ❌ | Auto-fit static image |
| `?path=...` query parameter | ❌ | Path/polyline overlay |
| `?marker=...` query parameter | ❌ | Marker overlay |
| `?padding=...` query parameter | ❌ | Padding for auto-fit |

### Source Data (Vector Tiles)

| Endpoint | Status | Description |
|----------|--------|-------------|
| `GET /data.json` | ✅ | List all tile sources |
| `GET /data/{id}.json` | ✅ | TileJSON for a source |
| `GET /data/{id}/{z}/{x}/{y}.{format}` | ✅ | Get vector tile (pbf/mvt) |
| `GET /data/{id}/{z}/{x}/{y}.geojson` | ✅ | Get tile as GeoJSON |
| `GET /data/{id}/elevation/{z}/{x}/{y}` | ❌ | Elevation for tile |
| `GET /data/{id}/elevation/{z}/{lon}/{lat}` | ❌ | Elevation at coordinate |

### Other

| Endpoint | Status | Description |
|----------|--------|-------------|
| `GET /health` | ✅ | Health check |
| `GET /fonts.json` | ✅ | List available fonts |
| `GET /index.json` | ✅ | Combined TileJSON array |
| `GET /files/{filename}` | ❌ | Static file serving |

---

## Priority Implementation Order

### High Priority (Required for most styles)

1. **Sprites** - `/styles/{id}/sprite[@2x].{format}`
   - Required for styles with icons/symbols
   - Need to serve both `.png` (image) and `.json` (metadata)
   - Support `@2x`, `@3x` for retina

2. **Fonts/Glyphs** - `/fonts/{fontstack}/{start}-{end}.pbf`
   - Required for text labels
   - PBF format (SDF glyphs)
   - Support font stacks (fallbacks)

3. **WMTS Capabilities** - `/styles/{id}/wmts.xml`
   - Required for GIS software (QGIS, ArcGIS)
   - XML format following OGC WMTS spec

### Medium Priority

4. **Variable Tile Size** - Support 256px and 512px tiles
   - Add `/{tileSize}/` path segment
   - Default to 512px (current behavior)

5. **Auto-fit Static Images** - `/styles/{id}/static/auto/...`
   - Auto-calculate bounds from path/markers
   - Useful for embedding maps with dynamic content

6. **Path Overlay** - `?path=...` query parameter
   - Draw polylines on static images
   - Support Google Encoded Polyline format
   - Styling options: stroke, fill, width

7. **Marker Overlay** - `?marker=...` query parameter
   - Place markers on static images
   - Support custom icons
   - Scale and offset options

### Lower Priority

8. **GeoJSON Tile Conversion** - `/data/{id}/{z}/{x}/{y}.geojson`
   - Convert PBF tiles to GeoJSON on-the-fly
   - Useful for debugging/inspection

9. **Elevation API** - `/data/{id}/elevation/...`
   - Query elevation from terrain tiles
   - Requires terrain RGB encoding support

10. **Font List** - `/fonts.json`
    - List all available font families
    - Useful for style editors

11. **Static File Serving** - `/files/{filename}`
    - Serve arbitrary static files
    - Useful for GeoJSON overlays

12. **Combined TileJSON** - `/index.json`
    - Array of all TileJSONs (styles + data)
    - With optional tile size prefix

---

## Implementation Notes

### Sprites

```
GET /styles/{id}/sprite.json      -> sprite metadata
GET /styles/{id}/sprite.png       -> sprite image
GET /styles/{id}/sprite@2x.json   -> retina metadata
GET /styles/{id}/sprite@2x.png    -> retina image
```

Sprites are typically stored alongside the style.json:
```
styles/
└── my-style/
    ├── style.json
    ├── sprite.json
    ├── sprite.png
    ├── sprite@2x.json
    └── sprite@2x.png
```

### Fonts

```
GET /fonts/Open Sans Regular/0-255.pbf
GET /fonts/Open Sans Bold,Arial Unicode MS Regular/256-511.pbf
```

Font stacks are comma-separated, server should:
1. Try first font in stack
2. Fall back to subsequent fonts
3. Return 404 if no fonts found

Fonts directory structure:
```
fonts/
├── Open Sans Regular/
│   ├── 0-255.pbf
│   ├── 256-511.pbf
│   └── ...
└── Arial Unicode MS Regular/
    └── ...
```

### WMTS

WMTS XML should include:
- Service metadata
- Layer definitions (one per style)
- TileMatrixSet (Web Mercator)
- Resource URLs for tiles

---

## Current Implementation Status

**Implemented:**
- Core tile serving (PMTiles, MBTiles)
- HTTP PMTiles support
- Native MapLibre rendering (via FFI)
- Raster tile generation
- Static image generation (center/bbox)
- TileJSON 3.0 for sources and styles
- Web UI with MapLibre GL JS viewer
- Raster/Vector viewer toggle

**Performance:**
- Warm cache: ~100ms per tile
- Cold cache: ~700-800ms per tile
- Static images: ~3s for 800x600
