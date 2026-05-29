# tileserver-rs benchmarks

Apples-to-apples performance comparison harness for tileserver-rs against
**tileserver-gl**, **martin**, and **titiler**. All servers run in Docker, all
hit the same fixtures, and the harness uses [autocannon](https://github.com/mcollina/autocannon)
for HTTP load.

## Quick start

```bash
# Build tileserver-rs first
docker build -t tileserver-rs:latest .

# Bring up everything (including titiler)
docker compose -f benchmarks/docker-compose.yml --profile all up -d

# Run all benchmarks
pnpm --filter @tileserver-rs/benchmarks run bench
```

## Benchmark types

| `--type`             | Servers tested                                    | What it measures                          |
|----------------------|---------------------------------------------------|-------------------------------------------|
| `pmtiles`            | tileserver-gl, tileserver-rs, martin              | PMTiles vector tile read throughput       |
| `mbtiles`            | tileserver-gl, tileserver-rs, martin              | MBTiles vector tile read throughput       |
| `postgres`           | tileserver-rs, martin                             | PostGIS table → MVT pipeline              |
| `postgres_function`  | tileserver-rs, martin                             | PostGIS user function → MVT               |
| `cog`                | tileserver-rs, titiler                            | Local COG raster tile generation          |
| `raster`             | tileserver-gl, tileserver-rs                      | Native MapLibre raster rendering          |
| **`ogc`**            | **tileserver-rs only**                            | OGC API Features (CQL2, schema, CRUD)     |
| **`stac`**           | **tileserver-rs, titiler**                        | STAC raster tiles (dynamic + static)      |
| **`compression`**    | **tileserver-rs, martin**                        | Brotli/zstd Accept-Encoding negotiation — payload size + throughput vs martin's gzip-only |

Pass `--type all` (default) to run everything. The harness auto-skips servers
that don't support a given protocol with an `N/A (protocol not supported)`
marker — no failures, no fake numbers.

## OGC API Features benchmark

tileserver-rs is the only entry in this suite that natively exposes OGC API
Features with full CRUD support. Run:

```bash
docker compose -f benchmarks/docker-compose.yml --profile full up -d
pnpm --filter @tileserver-rs/benchmarks run bench -- --type ogc
```

Endpoints exercised:

- `GET /ogc/collections/cities/items?limit=100`
- `GET /ogc/collections/countries/items?bbox=...&limit=100`
- `GET /ogc/collections/cities/items?filter=population%20%3E%201000000&filter-lang=cql2-text` — CQL2 filter
- `GET /ogc/collections/cities/items/1` — single feature
- `GET /ogc/collections/cities/schema` — JSON Schema introspection

The `cities` table is configured `writable = true`, so it also exercises Part 4
(Create/Replace/Delete) authentication paths.

Seed data lives in `benchmarks/postgres/init-ogc.sql` (500 cities, 50
countries, 1000 roads — all with GIST indexes).

## STAC raster tile benchmark

Compares tileserver-rs's native STAC integration against
[titiler](https://developmentseed.org/titiler/) — the canonical Python COG
server. Both serve raster tiles over the Alps bbox `[6.0, 43.0, 10.0, 46.0]`.

```bash
docker compose -f benchmarks/docker-compose.yml --profile all up -d
pnpm --filter @tileserver-rs/benchmarks run bench -- --type stac
```

Endpoints exercised:

- `GET /styles/sentinel2-dynamic/{z}/{x}/{y}.png` — dynamic per-tile STAC search
- `GET /styles/sentinel2-static/{z}/{x}/{y}.png` — static single-asset mode
- titiler equivalent: `GET /cog/tiles/WebMercatorQuad/{z}/{x}/{y}.png?url=file:///data/raster/benchmark-rgb.cog.tif`

> **Note:** STAC dynamic mode requires network access to
> `earth-search.aws.element84.com`. Cold runs are dominated by the STAC search +
> COG range-reads; the 30-second warmup helps but the numbers reflect real-world
> internet latency. titiler hits a local COG fixture (faster baseline) — this is
> intentional: it shows the cost of the STAC abstraction.

## Compression benchmark

Compares tileserver-rs's Brotli/zstd Accept-Encoding negotiation against
[martin](https://maplibre.org/martin/)'s gzip-only baseline. For each encoding
(`br`, `zstd`, `gzip`, `identity`) the harness:

1. Issues a single `HEAD`/`GET` request with `Accept-Encoding: <encoding>` and
   reads the `Content-Encoding` response header + payload byte size
2. Runs a 10-second, 100-connection autocannon throughput test at the same encoding

The output table shows requested encoding, actual encoding the server returned,
compressed payload size, req/s, and latency. If a server doesn't support an
encoding (e.g. martin receives `br` but only implements `gzip`), it falls back
silently and the "Actual" column flags the mismatch with `← fallback` in red.

```bash
docker compose -f benchmarks/docker-compose.yml --profile all up -d
pnpm --filter @tileserver-rs/benchmarks run bench -- --type compression
```

## Profiles

`docker-compose.yml` ships with these profiles:

- `full` — postgres + tileserver-gl + tileserver-rs + martin
- `titiler` — adds titiler
- `all` — everything above

```bash
docker compose -f benchmarks/docker-compose.yml --profile titiler up -d titiler
```

## CLI reference

```
node run-benchmarks.js [options]

  -s, --server <server>      tileserver-rs | tileserver-gl | martin | titiler | all  (default: all)
  -t, --type <type>          pmtiles | mbtiles | postgres | postgres_function | cog | raster | ogc | stac | compression | all
  -d, --duration <seconds>   measurement duration (single mode)             (default: 10)
  -c, --connections <num>    concurrent connections (single mode)           (default: 100)
  -m, --mode <mode>          single | grid                                  (default: single)
  -g, --grid-size <WxH>      grid dimensions for viewport simulation        (default: 4x4)
  -i, --iterations <num>     iterations per zoom in grid mode               (default: 50)
      --markdown             emit a Markdown report instead of console tables
```

OGC and STAC benchmarks always run a 30-second warmup pass before measurement
to remove JIT, connection-pool, and cache-warming noise.
