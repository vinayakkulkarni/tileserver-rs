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
