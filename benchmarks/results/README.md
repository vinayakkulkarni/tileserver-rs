# Benchmark Results

Raw autocannon JSON output files are stored in this directory.

## How to Run

```sh
# From repo root
cd benchmarks
npm run bench:metrics
```

## v2.28.0 Prometheus Metrics Results

Results for the `no-telemetry` and `metrics-strict` profiles will be
added here after running the macro-bench suite manually.

See the PR description for the Criterion micro-benchmark numbers
(sub-250 ns/call for strict cardinality).

## Brotli/zstd compression (#1000)

`node run-benchmarks.js --type compression` — Florence z13 PMTiles tile
(82.93 KB raw), 10s / 100 connections, tileserver-rs vs martin.

| Server        | Encoding | On-wire   | Req/s | Avg ms | P99 ms |
| ------------- | -------- | --------- | ----- | ------ | ------ |
| tileserver-rs | identity | 82.93 KB  | 3671  | 27.1   | 52     |
| tileserver-rs | gzip     | 61.81 KB  | 1486  | 70.0   | 83     |
| tileserver-rs | br       | 60.54 KB  | 1478  | 69.5   | 98     |
| tileserver-rs | zstd     | 64.55 KB  | 1388  | 73.5   | 95     |
| martin        | gzip     | 61.81 KB  | 1484  | 69.7   | 136    |
| martin        | br       | 58.52 KB  | 107   | 888.7  | 1206   |
| martin        | zstd     | 64.55 KB  | 1398  | 72.7   | 144    |

Headline: tileserver-rs serves brotli at ~1478 req/s (parity with its gzip
path) because re-encoded variants are cached per `(source, z, x, y, encoding)`.
Martin re-encodes brotli per request with no cache and collapses to 107 req/s
(~14x slower, 889 ms avg). Martin's br is marginally smaller (58.5 vs 60.5 KB)
because tileserver-rs defaults to `br_quality = 5` for first-paint latency;
raise it when precomputing. zstd and gzip are at parity across both servers.

## DEM terrain-RGB encoding (#1008)

`node run-benchmarks.js --type dem` — live on-the-fly DEM → Terrarium PNG
encoding from the SF-Bay `test-dem.cog.tif` fixture (EPSG:4326, elevations
−41..1041 m), 10s / 100 connections, 0 errors on both servers. titiler
(`algorithm=terrarium`, the only apples-to-apples live competitor; its
terrain-RGB feature merged Apr 2025) vs tileserver-rs.

| Server        | Zoom | Req/s | Avg ms  | P99 ms  |
| ------------- | ---- | ----- | ------- | ------- |
| tileserver-rs | z9   | 203   | 487     | 642     |
| tileserver-rs | z10  | 148   | 664     | 969     |
| tileserver-rs | z11  | 86    | 1152    | 1694    |
| tileserver-rs | z12  | 83    | 1274    | 2642    |
| titiler       | z9   | 50    | 1803    | 3778    |
| titiler       | z10  | 35    | 2472    | 5347    |
| titiler       | z11  | 21    | 3993    | 7096    |
| titiler       | z12  | 8     | 5045    | 10061   |

Headline: tileserver-rs serves live DEM terrain-RGB **~4–10× faster** than
titiler (130 vs 28 avg req/s, 894 vs 3328 ms avg latency) with **byte-accurate
encoding** — a round-trip decode of an encoded tile reproduces the source
elevation within the encoding interval (≤0.1 m Mapbox-RGB, ≤1/256 m Terrarium).
The gap widens at high zoom (10.4× at z12) where titiler's per-request Python /
rasterio cost dominates. Honest caveat: titiler reads from a `file://` COG here
(no network), the most favourable setup for it; numbers are single-run on an
Apple-Silicon Docker host, so treat the ratio (not the absolute req/s) as the
portable result.

## MVT→MLT transcode: native mlt-core 0.11 reader

`cargo bench --features mlt --bench mlt -- mvt_to_mlt_transcode` — real
OpenMapTiles MVT fixtures, Criterion (100 samples). Compares the `mvt_to_mlt`
path before and after adopting mlt-core 0.11's native `mvt::mvt_to_tile_layers`
reader, which replaced the hand-rolled GeoJSON `FeatureCollection` bridge. Both
sides run mlt-core 0.11.0, so this isolates the reader change, not the bump.

| Zoom | Bridge (before) | Native (after) | Change       | Throughput     |
| ---- | --------------- | -------------- | ------------ | -------------- |
| z0   | 15.59 ms        | 16.67 ms       | +6.95% (slower) | 5.06 → 4.73 MiB/s |
| z4   | 184.6 ms        | 142.3 ms       | −22.9% (faster) | 9.51 → 12.34 MiB/s |
| z7   | 230.2 ms        | 183.2 ms       | −20.4% (faster) | 7.65 → 9.61 MiB/s |
| z13  | 19.72 ms        | 15.89 ms       | −19.5% (faster) | 23.68 → 29.40 MiB/s |

Headline: ~20–23% faster MVT→MLT transcoding on real-data tiles (z4/z7/z13)
because the native reader decodes MVT straight into row-oriented `TileLayer`s
and skips the intermediate GeoJSON allocation + hand-rolled column-type
inference. The trivial z0 world-overview tile regresses ~7% (15.6 → 16.7 ms):
its near-empty payload is dominated by the native reader's fixed setup cost, so
there is nothing to amortise. All deltas are statistically significant
(p < 0.05). Measured on Apple Silicon (`--release`); absolute timings are
machine-specific — the percentage deltas are the portable signal.
