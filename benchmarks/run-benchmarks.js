#!/usr/bin/env node
/**
 * Benchmark runner for tileserver-rs vs martin vs tileserver-gl
 *
 * Tests PMTiles and MBTiles performance across all three servers
 *
 * Usage:
 *   node run-benchmarks.js                          # Run all benchmarks (single tile mode)
 *   node run-benchmarks.js --server tileserver-rs   # Single server
 *   node run-benchmarks.js --format mbtiles         # MBTiles only
 *   node run-benchmarks.js --duration 30            # 30 second tests
 *   node run-benchmarks.js --mode grid              # Grid tile viewport simulation
 *   node run-benchmarks.js --mode grid --grid-size 5x4  # Custom grid dimensions
 *   node run-benchmarks.js --mode grid --iterations 100  # More iterations for accuracy
 *
 * Prometheus Metrics Benchmarks:
 * To measure the overhead of Prometheus metrics, run tileserver-rs with different configs:
 *
 *   BASELINE (telemetry disabled):
 *     cargo run -- --config data/configs/no-telemetry.toml
 *     (in another terminal) npm run bench:metrics
 *
 *   METRICS ENABLED (strict cardinality):
 *     cargo run -- --config data/configs/metrics-strict.toml
 *     (in another terminal) npm run bench:metrics
 *
 * Results will show throughput and latency impact on the hot path (/data/{z}/{x}/{y}.pbf).
 * Micro-benchmarks (Criterion) for metrics overhead are in crates/tileserver-rs/benches/.
 */

import autocannon from 'autocannon';
import { program } from 'commander';
import chalk from 'chalk';
import Table from 'cli-table3';

// Server configurations
// Use 127.0.0.1 instead of localhost to avoid proxy issues
// All servers run in Docker for fair apples-to-apples comparison
const SERVERS = {
  'tileserver-gl': {
    name: 'tileserver-gl',
    port: 8900,
    color: chalk.yellow,
    pmtiles: {
      source: 'protomaps-sample',
      tileUrl: (z, x, y) => `http://127.0.0.1:8900/data/protomaps-sample/${z}/${x}/${y}.pbf`,
    },
    mbtiles: {
      source: 'zurich_switzerland',
      tileUrl: (z, x, y) => `http://127.0.0.1:8900/data/zurich_switzerland/${z}/${x}/${y}.pbf`,
    },
    raster: {
      source: 'protomaps-light',
      tileUrl: (z, x, y) => `http://127.0.0.1:8900/styles/protomaps-light/${z}/${x}/${y}@2x.png`,
    },
    healthUrl: 'http://127.0.0.1:8900/health',
  },
  'tileserver-rs': {
    name: 'tileserver-rs',
    port: 8901,
    color: chalk.green,
    pmtiles: {
      source: 'pmtiles',
      tileUrl: (z, x, y) => `http://127.0.0.1:8901/data/pmtiles/${z}/${x}/${y}.pbf`,
    },
    mbtiles: {
      source: 'mbtiles',
      tileUrl: (z, x, y) => `http://127.0.0.1:8901/data/mbtiles/${z}/${x}/${y}.pbf`,
    },
    raster: {
      source: 'protomaps-light',
      tileUrl: (z, x, y) => `http://127.0.0.1:8901/styles/protomaps-light/${z}/${x}/${y}.png`,
    },
    postgres: {
      source: 'benchmark_table',
      tileUrl: (z, x, y) => `http://127.0.0.1:8901/data/benchmark_table/${z}/${x}/${y}.pbf`,
    },
    postgres_function: {
      source: 'benchmark_points',
      tileUrl: (z, x, y) => `http://127.0.0.1:8901/data/benchmark_points/${z}/${x}/${y}.pbf`,
    },
    cog: {
      source: 'cog-rgb',
      tileUrl: (z, x, y) => `http://127.0.0.1:8901/data/cog-rgb/${z}/${x}/${y}.png`,
    },
    ogc: {
      source: 'ogc-features',
      endpointUrl: (path) => `http://127.0.0.1:8901${path}`,
    },
    stac: {
      source: 'sentinel2',
      tileUrl: (z, x, y) => `http://127.0.0.1:8901/styles/sentinel2-dynamic/${z}/${x}/${y}.png`,
      tileUrlStatic: (z, x, y) => `http://127.0.0.1:8901/styles/sentinel2-static/${z}/${x}/${y}.png`,
    },
    healthUrl: 'http://127.0.0.1:8901/health',
  },
  martin: {
    name: 'martin',
    port: 8902,
    color: chalk.blue,
    pmtiles: {
      source: 'protomaps-sample',
      tileUrl: (z, x, y) => `http://127.0.0.1:8902/protomaps-sample/${z}/${x}/${y}`,
    },
    mbtiles: {
      source: 'zurich_switzerland',
      tileUrl: (z, x, y) => `http://127.0.0.1:8902/zurich_switzerland/${z}/${x}/${y}`,
    },
    postgres: {
      source: 'benchmark_points',
      tileUrl: (z, x, y) => `http://127.0.0.1:8902/benchmark_points/${z}/${x}/${y}`,
    },
    postgres_function: {
      source: 'get_benchmark_tiles',
      tileUrl: (z, x, y) => `http://127.0.0.1:8902/get_benchmark_tiles/${z}/${x}/${y}`,
    },
    healthUrl: 'http://127.0.0.1:8902/catalog',
  },
  titiler: {
    name: 'titiler',
    port: 8903,
    color: chalk.magenta,
    cog: {
      source: 'benchmark-rgb',
      tileUrl: (z, x, y) => `http://127.0.0.1:8903/cog/tiles/WebMercatorQuad/${z}/${x}/${y}.png?url=file:///data/raster/benchmark-rgb.cog.tif`,
    },
    stac: {
      source: 'benchmark-rgb',
      tileUrl: (z, x, y) =>
        `http://127.0.0.1:8903/cog/tiles/WebMercatorQuad/${z}/${x}/${y}.png?url=file:///data/raster/benchmark-rgb.cog.tif`,
    },
    healthUrl: 'http://127.0.0.1:8903/healthz',
  },
};

// Protocol support matrix — used for skip messaging and markdown report.
// ✓ supported, ✗ not supported, ◐ partial / read-only
const PROTOCOL_SUPPORT = {
  pmtiles: { 'tileserver-gl': '✓', 'tileserver-rs': '✓', martin: '✓', titiler: '✗' },
  mbtiles: { 'tileserver-gl': '✓', 'tileserver-rs': '✓', martin: '✓', titiler: '✗' },
  postgres: { 'tileserver-gl': '✗', 'tileserver-rs': '✓', martin: '✓', titiler: '✗' },
  postgres_function: { 'tileserver-gl': '✗', 'tileserver-rs': '✓', martin: '✓', titiler: '✗' },
  cog: { 'tileserver-gl': '✗', 'tileserver-rs': '✓', martin: '✗', titiler: '✓' },
  raster: { 'tileserver-gl': '✓', 'tileserver-rs': '✓', martin: '✗', titiler: '✗' },
  ogc: { 'tileserver-gl': '✗', 'tileserver-rs': '✓', martin: '✗', titiler: '✗' },
  stac: { 'tileserver-gl': '✗', 'tileserver-rs': '✓', martin: '✗', titiler: '◐' },
};

// Test tiles - coordinates calculated from actual data bounds using:
//   x = floor((lon + 180) / 360 * 2^z)
//   y = floor((1 - asinh(tan(lat_rad)) / pi) / 2 * 2^z)
//
// PMTiles (Florence): bounds [11.22, 43.75, 11.29, 43.79], zoom 0-15
//   Center: lat=43.7672, lon=11.2543
// MBTiles (Zurich): bounds [8.45, 47.32, 8.63, 47.44], zoom 0-14
//   Center: lat=47.377, lon=8.538
//
// All coordinates verified to return 200 OK with real tile data.
// NOTE for grid mode: GRID_TILES (below) is used instead because single-tile
// zooms like z10 produce 4×4 grids that extend beyond the small fixture bounds
// (Florence fixture is ~5 km wide; a z10 4×4 grid spans ~32 km). GRID_TILES
// starts at zooms where the full 4×4 / 5×4 grid lives entirely inside the data.
const TEST_TILES = {
  pmtiles: [
    { z: 10, x: 544, y: 373, desc: 'Florence z10' },
    { z: 11, x: 1088, y: 746, desc: 'Florence z11' },
    { z: 12, x: 2176, y: 1493, desc: 'Florence z12' },
    { z: 13, x: 4352, y: 2986, desc: 'Florence z13' },
    { z: 14, x: 8704, y: 5972, desc: 'Florence z14' },
    { z: 15, x: 17408, y: 11944, desc: 'Florence z15' },
  ],
  mbtiles: [
    { z: 10, x: 536, y: 358, desc: 'Zurich z10' },
    { z: 11, x: 1072, y: 717, desc: 'Zurich z11' },
    { z: 12, x: 2145, y: 1434, desc: 'Zurich z12' },
    { z: 13, x: 4290, y: 2868, desc: 'Zurich z13' },
    { z: 14, x: 8580, y: 5737, desc: 'Zurich z14' },
  ],
  postgres: [
    { z: 10, x: 536, y: 358, desc: 'Table z10' },
    { z: 11, x: 1072, y: 717, desc: 'Table z11' },
    { z: 12, x: 2145, y: 1434, desc: 'Table z12' },
    { z: 13, x: 4290, y: 2868, desc: 'Table z13' },
    { z: 14, x: 8580, y: 5737, desc: 'Table z14' },
  ],
  postgres_function: [
    { z: 10, x: 536, y: 358, desc: 'Function z10' },
    { z: 11, x: 1072, y: 717, desc: 'Function z11' },
    { z: 12, x: 2145, y: 1434, desc: 'Function z12' },
    { z: 13, x: 4290, y: 2868, desc: 'Function z13' },
    { z: 14, x: 8580, y: 5737, desc: 'Function z14' },
  ],
  cog: [
    { z: 0, x: 0, y: 0, desc: 'World z0' },
    { z: 1, x: 0, y: 0, desc: 'World z1' },
    { z: 1, x: 1, y: 0, desc: 'World z1' },
    { z: 2, x: 1, y: 1, desc: 'World z2' },
    { z: 2, x: 2, y: 1, desc: 'World z2' },
    { z: 3, x: 4, y: 3, desc: 'World z3' },
  ],
  raster: [
    { z: 10, x: 544, y: 373, desc: 'Florence z10' },
    { z: 11, x: 1088, y: 746, desc: 'Florence z11' },
    { z: 12, x: 2176, y: 1493, desc: 'Florence z12' },
    { z: 13, x: 4352, y: 2986, desc: 'Florence z13' },
    { z: 14, x: 8704, y: 5972, desc: 'Florence z14' },
  ],
  // STAC tiles — Alps bbox [6.0, 43.0, 10.0, 46.0], center (lat=44.5, lon=8.0)
  // x = floor((8 + 180) / 360 * 2^z), y = floor((1 - asinh(tan(44.5°))/π)/2 * 2^z)
  stac: [
    { z: 8, x: 133, y: 91, desc: 'Alps z8' },
    { z: 10, x: 534, y: 366, desc: 'Alps z10' },
    { z: 12, x: 2138, y: 1465, desc: 'Alps z12' },
    { z: 14, x: 8554, y: 5862, desc: 'Alps z14' },
  ],
};

// Grid-mode tiles: center coordinates chosen so the default 4×4 / 5×4 grid
// lives entirely inside fixture bounds. See TEST_TILES comment for why these
// diverge from single-tile zooms. Numbers verified with `curl` against all
// 16 tiles of each grid returning HTTP 200.
const GRID_TILES = {
  pmtiles: [
    { z: 14, x: 8704, y: 5972, desc: 'Florence z14' },
    { z: 15, x: 17408, y: 11944, desc: 'Florence z15' },
  ],
  mbtiles: [
    { z: 12, x: 2145, y: 1434, desc: 'Zurich z12' },
    { z: 13, x: 4290, y: 2868, desc: 'Zurich z13' },
    { z: 14, x: 8580, y: 5737, desc: 'Zurich z14' },
  ],
  postgres: [
    { z: 12, x: 2145, y: 1434, desc: 'Table z12' },
    { z: 13, x: 4290, y: 2868, desc: 'Table z13' },
    { z: 14, x: 8580, y: 5737, desc: 'Table z14' },
  ],
  postgres_function: [
    { z: 12, x: 2145, y: 1434, desc: 'Function z12' },
    { z: 13, x: 4290, y: 2868, desc: 'Function z13' },
    { z: 14, x: 8580, y: 5737, desc: 'Function z14' },
  ],
  cog: [
    { z: 2, x: 2, y: 1, desc: 'World z2' },
    { z: 3, x: 4, y: 3, desc: 'World z3' },
    { z: 4, x: 8, y: 6, desc: 'World z4' },
  ],
  raster: [
    { z: 13, x: 4352, y: 2986, desc: 'Florence z13' },
    { z: 14, x: 8704, y: 5972, desc: 'Florence z14' },
  ],
  stac: [
    { z: 10, x: 534, y: 366, desc: 'Alps z10' },
    { z: 12, x: 2138, y: 1465, desc: 'Alps z12' },
  ],
};

// OGC API Features endpoints (tileserver-rs only)
const OGC_ENDPOINTS = [
  { path: '/ogc/collections/cities/items?limit=100', desc: 'cities items (100)' },
  { path: '/ogc/collections/countries/items?bbox=-10,40,20,60&limit=100', desc: 'countries bbox' },
  {
    path: '/ogc/collections/cities/items?filter=population%20%3E%201000000&filter-lang=cql2-text&limit=50',
    desc: 'cities CQL2 filter',
  },
  { path: '/ogc/collections/cities/items/1', desc: 'single feature' },
  { path: '/ogc/collections/cities/schema', desc: 'schema introspect' },
];

// STAC test endpoints — mix of dynamic and static modes for tileserver-rs
function stacEndpointsFor(server) {
  const cfg = server.stac;
  if (!cfg) return [];
  const out = [];
  for (const t of TEST_TILES.stac) {
    out.push({ url: cfg.tileUrl(t.z, t.x, t.y), desc: `${t.desc} dynamic` });
    if (cfg.tileUrlStatic) {
      out.push({ url: cfg.tileUrlStatic(t.z, t.x, t.y), desc: `${t.desc} static` });
    }
  }
  return out;
}

// ─── Grid Viewport Simulation ──────────────────────────────────────────────────
// Real MapLibre GL JS viewports load a grid of tiles simultaneously.
// For 1920×1080 at 512px tiles: ceil(1920/512)+1=5 wide, ceil(1080/512)+1=4 tall
// Default: 4×4 = 16 tiles per viewport load

function generateTileGrid(centerZ, centerX, centerY, width = 4, height = 4) {
  const tiles = [];
  const startX = centerX - Math.floor(width / 2);
  const startY = centerY - Math.floor(height / 2);

  for (let row = 0; row < height; row++) {
    for (let col = 0; col < width; col++) {
      tiles.push({
        z: centerZ,
        x: startX + col,
        y: startY + row,
      });
    }
  }
  return tiles;
}

// Measures time until the LAST tile completes (= viewport fill time)
async function loadGrid(urls) {
  const start = performance.now();

  const results = await Promise.all(
    urls.map(async (url) => {
      try {
        const tileStart = performance.now();
        const response = await fetch(url);
        const buffer = await response.arrayBuffer();
        return {
          ok: response.ok,
          size: buffer.byteLength,
          latency: performance.now() - tileStart,
        };
      } catch {
        return { ok: false, size: 0, latency: performance.now() - start };
      }
    })
  );

  return {
    gridLatency: performance.now() - start,
    totalSize: results.reduce((sum, r) => sum + r.size, 0),
    errors: results.filter((r) => !r.ok).length,
    tileCount: urls.length,
    maxTileLatency: Math.max(...results.map((r) => r.latency)),
  };
}

async function benchmarkGridIterations(urls, iterations, warmup = 3) {
  // Warmup runs — prime caches, JIT, connection pools
  for (let i = 0; i < warmup; i++) {
    await loadGrid(urls);
  }

  const gridResults = [];
  for (let i = 0; i < iterations; i++) {
    const result = await loadGrid(urls);
    gridResults.push(result);
  }

  const latencies = gridResults.map((r) => r.gridLatency).sort((a, b) => a - b);
  const avgSize = gridResults.reduce((sum, r) => sum + r.totalSize, 0) / iterations;
  const totalErrors = gridResults.reduce((sum, r) => sum + r.errors, 0);
  const avgLatency = latencies.reduce((s, l) => s + l, 0) / latencies.length;

  return {
    iterations,
    tileCount: urls.length,
    min: latencies[0],
    avg: avgLatency,
    p50: percentile(latencies, 50),
    p99: percentile(latencies, 99),
    max: latencies[latencies.length - 1],
    gridsPerSec: 1000 / avgLatency,
    avgThroughput: avgSize,
    errors: totalErrors,
  };
}

function percentile(sorted, p) {
  const idx = Math.ceil((p / 100) * sorted.length) - 1;
  return sorted[Math.max(0, idx)];
}

// ─── Single-Tile Benchmarks (existing) ─────────────────────────────────────────

// Benchmark configuration
let BENCHMARK_CONFIG = {
  duration: 10, // seconds
  connections: 100,
  pipelining: 1,
  timeout: 30,
};

/**
 * Run autocannon benchmark
 */
async function runBenchmark(url, name, overrides = {}) {
  return new Promise((resolve, reject) => {
    const instance = autocannon(
      {
        url,
        ...BENCHMARK_CONFIG,
        ...overrides,
        title: name,
      },
      (err, result) => {
        if (err) {
          reject(err);
        } else {
          resolve(result);
        }
      }
    );

    // Don't print autocannon's default output
    autocannon.track(instance, { renderProgressBar: false });
  });
}

/**
 * Check if server is available
 */
async function checkServer(healthUrl) {
  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 2000);

    const response = await fetch(healthUrl, {
      signal: controller.signal,
    });
    clearTimeout(timeout);
    return response.ok || response.status === 200;
  } catch {
    return false;
  }
}

/**
 * Run single-tile benchmarks for a server and format
 */
async function benchmarkServerFormat(serverKey, format) {
  const server = SERVERS[serverKey];
  if (!server) {
    console.log(chalk.red(`Unknown server: ${serverKey}`));
    return [];
  }

  const formatConfig = server[format];
  if (!formatConfig) {
    console.log(chalk.gray(`  ${server.name}: ${format.toUpperCase()} not supported, skipping`));
    return [];
  }

  const tiles = TEST_TILES[format];
  const results = [];

  console.log(server.color(`\n  Testing ${server.name} (${format.toUpperCase()})...`));

  for (const tile of tiles) {
    const url = formatConfig.tileUrl(tile.z, tile.x, tile.y);

    process.stdout.write(chalk.gray(`    z${tile.z.toString().padStart(2)} (${tile.desc.padEnd(12)})... `));

    try {
      const overrides = format === 'raster' ? { connections: 10, timeout: 60 } : {};
      const result = await runBenchmark(url, `${server.name} ${format} z${tile.z}`, overrides);

      const reqPerSec = (result.requests.total / BENCHMARK_CONFIG.duration).toFixed(0);
      const latencyAvg = result.latency.average.toFixed(2);
      const errors = result.errors + (result.non2xx || 0);

      // Color code based on performance
      let perfColor = chalk.green;
      if (result.latency.average > 50) perfColor = chalk.yellow;
      if (result.latency.average > 200 || errors > 0) perfColor = chalk.red;

      if (errors > 0) {
        console.log(perfColor(`${reqPerSec} req/s, ${latencyAvg}ms avg (${errors} errors)`));
      } else {
        console.log(perfColor(`${reqPerSec} req/s, ${latencyAvg}ms avg`));
      }

      results.push({
        server: server.name,
        serverId: serverKey,
        format,
        zoom: tile.z,
        desc: tile.desc,
        requests: result.requests.total,
        throughput: result.throughput.total,
        latencyAvg: result.latency.average,
        latencyP50: result.latency.p50,
        latencyP99: result.latency.p99,
        errors: errors,
      });
    } catch (err) {
      console.log(chalk.red(`Error: ${err.message}`));
    }
  }

  return results;
}

async function benchmarkServerGrid(serverKey, format, gridConfig) {
  const server = SERVERS[serverKey];
  if (!server) {
    console.log(chalk.red(`Unknown server: ${serverKey}`));
    return [];
  }

  const formatConfig = server[format];
  if (!formatConfig) {
    console.log(chalk.gray(`  ${server.name}: ${format.toUpperCase()} not supported, skipping`));
    return [];
  }

  const tiles = GRID_TILES[format] ?? TEST_TILES[format];
  if (!tiles) return [];

  const results = [];
  const { width, height, iterations } = gridConfig;

  console.log(server.color(`\n  Testing ${server.name} (${format.toUpperCase()}, ${width}×${height} grid, ${iterations} iterations)...`));

  for (const centerTile of tiles) {
    const grid = generateTileGrid(centerTile.z, centerTile.x, centerTile.y, width, height);
    const urls = grid.map((t) => formatConfig.tileUrl(t.z, t.x, t.y));

    process.stdout.write(chalk.gray(`    z${centerTile.z.toString().padStart(2)} (${centerTile.desc.padEnd(12)}) ${width}×${height}... `));

    try {
      // Raster tiles are much slower — use fewer iterations
      const effectiveIterations = format === 'raster' ? Math.max(10, Math.floor(iterations / 3)) : iterations;
      const stats = await benchmarkGridIterations(urls, effectiveIterations);

      let perfColor = chalk.green;
      if (stats.avg > 500) perfColor = chalk.yellow;
      if (stats.avg > 2000 || stats.errors > 0) perfColor = chalk.red;

      if (stats.errors > 0) {
        console.log(perfColor(`${stats.gridsPerSec.toFixed(1)} grids/s, ${stats.avg.toFixed(0)}ms avg, ${stats.p99.toFixed(0)}ms p99 (${stats.errors} errors)`));
      } else {
        console.log(perfColor(`${stats.gridsPerSec.toFixed(1)} grids/s, ${stats.avg.toFixed(0)}ms avg, ${stats.p99.toFixed(0)}ms p99`));
      }

      results.push({
        server: server.name,
        serverId: serverKey,
        format,
        zoom: centerTile.z,
        desc: centerTile.desc,
        grid: `${width}×${height}`,
        tileCount: width * height,
        ...stats,
      });
    } catch (err) {
      console.log(chalk.red(`Error: ${err.message}`));
    }
  }

  return results;
}

// ─── OGC API Features Benchmarks ───────────────────────────────────────────────

async function benchmarkServerOgc(serverKey) {
  const server = SERVERS[serverKey];
  if (!server) return [];

  if (!server.ogc) {
    console.log(chalk.gray(`  ${server.name}: OGC API Features N/A (protocol not supported), skipping`));
    return [];
  }

  const results = [];
  console.log(server.color(`\n  Testing ${server.name} (OGC API Features)...`));
  console.log(chalk.gray(`    warmup: 30s, measure: ${BENCHMARK_CONFIG.duration}s, ${BENCHMARK_CONFIG.connections} conns`));

  for (const ep of OGC_ENDPOINTS) {
    const url = server.ogc.endpointUrl(ep.path);
    process.stdout.write(chalk.gray(`    ${ep.desc.padEnd(22)}... `));

    try {
      // Warmup pass — discard
      await runBenchmark(url, `warmup`, { duration: 30 });
      const result = await runBenchmark(url, `${server.name} ogc ${ep.desc}`);

      const reqPerSec = (result.requests.total / BENCHMARK_CONFIG.duration).toFixed(0);
      const latencyAvg = result.latency.average.toFixed(2);
      const errors = result.errors + (result.non2xx || 0);

      let perfColor = chalk.green;
      if (result.latency.average > 50) perfColor = chalk.yellow;
      if (result.latency.average > 200 || errors > 0) perfColor = chalk.red;

      console.log(perfColor(`${reqPerSec} req/s, ${latencyAvg}ms avg${errors ? ` (${errors} errors)` : ''}`));

      results.push({
        server: server.name,
        serverId: serverKey,
        format: 'ogc',
        zoom: 0,
        desc: ep.desc,
        requests: result.requests.total,
        throughput: result.throughput.total,
        latencyAvg: result.latency.average,
        latencyP50: result.latency.p50,
        latencyP99: result.latency.p99,
        errors,
      });
    } catch (err) {
      console.log(chalk.red(`Error: ${err.message}`));
    }
  }

  return results;
}

// ─── STAC Raster Tile Benchmarks ───────────────────────────────────────────────

async function benchmarkServerStac(serverKey) {
  const server = SERVERS[serverKey];
  if (!server) return [];

  if (!server.stac) {
    console.log(chalk.gray(`  ${server.name}: STAC N/A (protocol not supported), skipping`));
    return [];
  }

  const endpoints = stacEndpointsFor(server);
  if (endpoints.length === 0) return [];

  const results = [];
  console.log(server.color(`\n  Testing ${server.name} (STAC raster)...`));
  console.log(chalk.gray(`    warmup: 30s, measure: ${BENCHMARK_CONFIG.duration}s, 20 conns (CPU-heavy)`));

  for (const ep of endpoints) {
    process.stdout.write(chalk.gray(`    ${ep.desc.padEnd(22)}... `));

    try {
      const overrides = { connections: 20, timeout: 60 };
      await runBenchmark(ep.url, 'warmup', { ...overrides, duration: 30 });
      const result = await runBenchmark(ep.url, `${server.name} stac ${ep.desc}`, overrides);

      const reqPerSec = (result.requests.total / BENCHMARK_CONFIG.duration).toFixed(0);
      const latencyAvg = result.latency.average.toFixed(2);
      const errors = result.errors + (result.non2xx || 0);

      let perfColor = chalk.green;
      if (result.latency.average > 500) perfColor = chalk.yellow;
      if (result.latency.average > 2000 || errors > 0) perfColor = chalk.red;

      console.log(perfColor(`${reqPerSec} req/s, ${latencyAvg}ms avg${errors ? ` (${errors} errors)` : ''}`));

      results.push({
        server: server.name,
        serverId: serverKey,
        format: 'stac',
        zoom: 0,
        desc: ep.desc,
        requests: result.requests.total,
        throughput: result.throughput.total,
        latencyAvg: result.latency.average,
        latencyP50: result.latency.p50,
        latencyP99: result.latency.p99,
        errors,
      });
    } catch (err) {
      console.log(chalk.red(`Error: ${err.message}`));
    }
  }

  return results;
}

// ─── Results Display ───────────────────────────────────────────────────────────

/**
 * Print single-tile results table
 */
function printResults(results, format) {
  const filtered = results.filter((r) => r.format === format);
  if (filtered.length === 0) {
    return;
  }

  const table = new Table({
    head: [
      chalk.bold('Server'),
      chalk.bold('Zoom'),
      chalk.bold('Location'),
      chalk.bold('Req/sec'),
      chalk.bold('Throughput'),
      chalk.bold('Avg (ms)'),
      chalk.bold('P99 (ms)'),
      chalk.bold('Errors'),
    ],
    colAligns: ['left', 'right', 'left', 'right', 'right', 'right', 'right', 'right'],
  });

  // Group by zoom for comparison
  const byZoom = {};
  for (const r of filtered) {
    if (!byZoom[r.zoom]) byZoom[r.zoom] = [];
    byZoom[r.zoom].push(r);
  }

  for (const zoom of Object.keys(byZoom).sort((a, b) => a - b)) {
    const zoomResults = byZoom[zoom];
    // Sort by req/sec (fastest first)
    zoomResults.sort((a, b) => b.requests - a.requests);

    for (const r of zoomResults) {
      const server = SERVERS[r.serverId];
      const colorFn = server?.color || chalk.white;

      table.push([
        colorFn(r.server),
        `z${r.zoom}`,
        r.desc,
        (r.requests / BENCHMARK_CONFIG.duration).toFixed(0),
        formatBytes(r.throughput / BENCHMARK_CONFIG.duration) + '/s',
        r.latencyAvg.toFixed(2),
        r.latencyP99.toFixed(2),
        r.errors > 0 ? chalk.red(r.errors) : '0',
      ]);
    }
  }

  console.log(`\n${chalk.bold(format.toUpperCase() + ' Results:')}`);
  console.log(table.toString());
}

/**
 * Print single-tile summary comparison
 */
function printSummary(results) {
  console.log(chalk.bold.cyan('\n📊 Summary by Server\n'));

  // Group by server and format
  const summary = {};
  for (const r of results) {
    const key = `${r.serverId}-${r.format}`;
    if (!summary[key]) {
      summary[key] = { server: r.server, serverId: r.serverId, format: r.format, requests: 0, throughput: 0, latency: 0, count: 0, errors: 0 };
    }
    summary[key].requests += r.requests;
    summary[key].throughput += r.throughput;
    summary[key].latency += r.latencyAvg;
    summary[key].errors += r.errors;
    summary[key].count++;
  }

  const summaryTable = new Table({
    head: [chalk.bold('Server'), chalk.bold('Format'), chalk.bold('Avg Req/sec'), chalk.bold('Avg Throughput'), chalk.bold('Avg Latency'), chalk.bold('Errors')],
    colAligns: ['left', 'left', 'right', 'right', 'right', 'right'],
  });

  for (const data of Object.values(summary)) {
    const server = SERVERS[data.serverId];
    const avgReqSec = data.requests / data.count / BENCHMARK_CONFIG.duration;
    const avgThroughput = data.throughput / data.count / BENCHMARK_CONFIG.duration;
    const avgLatency = data.latency / data.count;

    summaryTable.push([
      server.color(data.server),
      data.format.toUpperCase(),
      avgReqSec.toFixed(0),
      formatBytes(avgThroughput) + '/s',
      avgLatency.toFixed(2) + 'ms',
      data.errors > 0 ? chalk.red(data.errors) : '0',
    ]);
  }

  console.log(summaryTable.toString());
}

function printGridResults(results, format) {
  const filtered = results.filter((r) => r.format === format);
  if (filtered.length === 0) return;

  const table = new Table({
    head: [
      chalk.bold('Server'),
      chalk.bold('Zoom'),
      chalk.bold('Grid'),
      chalk.bold('Grids/s'),
      chalk.bold('Avg (ms)'),
      chalk.bold('P50 (ms)'),
      chalk.bold('P99 (ms)'),
      chalk.bold('Throughput'),
      chalk.bold('Errors'),
    ],
    colAligns: ['left', 'right', 'left', 'right', 'right', 'right', 'right', 'right', 'right'],
  });

  const byZoom = {};
  for (const r of filtered) {
    if (!byZoom[r.zoom]) byZoom[r.zoom] = [];
    byZoom[r.zoom].push(r);
  }

  for (const zoom of Object.keys(byZoom).sort((a, b) => a - b)) {
    const zoomResults = byZoom[zoom];
    // Sort by grids/sec (fastest first)
    zoomResults.sort((a, b) => b.gridsPerSec - a.gridsPerSec);

    for (const r of zoomResults) {
      const server = SERVERS[r.serverId];
      const colorFn = server?.color || chalk.white;

      table.push([
        colorFn(r.server),
        `z${r.zoom}`,
        r.grid,
        r.gridsPerSec.toFixed(1),
        r.avg.toFixed(0),
        r.p50.toFixed(0),
        r.p99.toFixed(0),
        formatBytes(r.avgThroughput) + '/grid',
        r.errors > 0 ? chalk.red(r.errors) : '0',
      ]);
    }
  }

  console.log(`\n${chalk.bold(format.toUpperCase() + ' Grid Results:')}`);
  console.log(table.toString());
}

function printGridSummary(results) {
  console.log(chalk.bold.cyan('\n📊 Grid Viewport Summary\n'));

  const summary = {};
  for (const r of results) {
    const key = `${r.serverId}-${r.format}`;
    if (!summary[key]) {
      summary[key] = {
        server: r.server,
        serverId: r.serverId,
        format: r.format,
        gridsPerSec: 0,
        avgLatency: 0,
        avgThroughput: 0,
        count: 0,
        errors: 0,
      };
    }
    summary[key].gridsPerSec += r.gridsPerSec;
    summary[key].avgLatency += r.avg;
    summary[key].avgThroughput += r.avgThroughput;
    summary[key].errors += r.errors;
    summary[key].count++;
  }

  const summaryTable = new Table({
    head: [
      chalk.bold('Server'),
      chalk.bold('Format'),
      chalk.bold('Avg Grids/s'),
      chalk.bold('Avg Grid Latency'),
      chalk.bold('Avg Throughput'),
      chalk.bold('Errors'),
    ],
    colAligns: ['left', 'left', 'right', 'right', 'right', 'right'],
  });

  for (const data of Object.values(summary)) {
    const server = SERVERS[data.serverId];
    summaryTable.push([
      server.color(data.server),
      data.format.toUpperCase(),
      (data.gridsPerSec / data.count).toFixed(1),
      (data.avgLatency / data.count).toFixed(0) + 'ms',
      formatBytes(data.avgThroughput / data.count) + '/grid',
      data.errors > 0 ? chalk.red(data.errors) : '0',
    ]);
  }

  console.log(summaryTable.toString());
}

// ─── Markdown Reports ──────────────────────────────────────────────────────────

/**
 * Generate markdown report for single-tile benchmarks
 */
function generateProtocolMatrix() {
  const servers = ['tileserver-gl', 'tileserver-rs', 'martin', 'titiler'];
  const types = Object.keys(PROTOCOL_SUPPORT);
  let md = `### Protocol Support Matrix\n\n`;
  md += `| Type | ${servers.join(' | ')} |\n`;
  md += `|------|${servers.map(() => '---').join('|')}|\n`;
  for (const t of types) {
    md += `| ${t} | ${servers.map((s) => PROTOCOL_SUPPORT[t][s] ?? '✗').join(' | ')} |\n`;
  }
  md += `\n_Legend: ✓ supported · ✗ not supported · ◐ partial / read-only._\n`;
  md += `_Features as of April 2026. Check competitor docs for latest._\n\n`;
  return md;
}

function generateMarkdownReport(results) {
  const summary = {};
  for (const r of results) {
    const key = `${r.serverId}-${r.format}`;
    if (!summary[key]) {
      summary[key] = { server: r.server, format: r.format, requests: 0, throughput: 0, latency: 0, count: 0, errors: 0 };
    }
    summary[key].requests += r.requests;
    summary[key].throughput += r.throughput;
    summary[key].latency += r.latencyAvg;
    summary[key].errors += r.errors;
    summary[key].count++;
  }

  let md = `## Benchmark Results

**Test Configuration:**
- Mode: Single tile (throughput)
- Duration: ${BENCHMARK_CONFIG.duration} seconds per endpoint
- Connections: ${BENCHMARK_CONFIG.connections} concurrent
- Date: ${new Date().toISOString().split('T')[0]}

${generateProtocolMatrix()}### Summary

| Server | Format | Avg Req/sec | Avg Throughput | Avg Latency | Errors |
|--------|--------|-------------|----------------|-------------|--------|
`;

  for (const data of Object.values(summary)) {
    const avgReqSec = data.requests / data.count / BENCHMARK_CONFIG.duration;
    const avgThroughput = data.throughput / data.count / BENCHMARK_CONFIG.duration;
    const avgLatency = data.latency / data.count;

    md += `| ${data.server} | ${data.format.toUpperCase()} | ${avgReqSec.toFixed(0)} | ${formatBytes(avgThroughput)}/s | ${avgLatency.toFixed(2)}ms | ${data.errors} |\n`;
  }

  md += `\n### Detailed Results\n\n`;

  for (const format of ['pmtiles', 'mbtiles', 'postgres', 'postgres_function', 'cog', 'raster', 'ogc', 'stac']) {
    const filtered = results.filter((r) => r.format === format);
    if (filtered.length === 0) continue;

    md += `#### ${format.toUpperCase()}\n\n`;
    md += `| Server | Zoom | Location | Req/sec | Throughput | Avg Latency | P99 Latency |\n`;
    md += `|--------|------|----------|---------|------------|-------------|-------------|\n`;

    for (const r of filtered) {
      md += `| ${r.server} | z${r.zoom} | ${r.desc} | ${(r.requests / BENCHMARK_CONFIG.duration).toFixed(0)} | ${formatBytes(r.throughput / BENCHMARK_CONFIG.duration)}/s | ${r.latencyAvg.toFixed(2)}ms | ${r.latencyP99.toFixed(2)}ms |\n`;
    }
    md += '\n';
  }

  return md;
}

function generateGridMarkdownReport(results, gridConfig) {
  const summary = {};
  for (const r of results) {
    const key = `${r.serverId}-${r.format}`;
    if (!summary[key]) {
      summary[key] = {
        server: r.server,
        format: r.format,
        gridsPerSec: 0,
        avgLatency: 0,
        avgThroughput: 0,
        count: 0,
        errors: 0,
      };
    }
    summary[key].gridsPerSec += r.gridsPerSec;
    summary[key].avgLatency += r.avg;
    summary[key].avgThroughput += r.avgThroughput;
    summary[key].errors += r.errors;
    summary[key].count++;
  }

  let md = `## Grid Viewport Benchmark Results

**Test Configuration:**
- Mode: Grid viewport (simulates real MapLibre map loads)
- Grid Size: ${gridConfig.width}×${gridConfig.height} (${gridConfig.width * gridConfig.height} tiles per viewport)
- Iterations: ${gridConfig.iterations} per zoom level (+ 3 warmup)
- Date: ${new Date().toISOString().split('T')[0]}

### What This Measures

Unlike single-tile throughput benchmarks, grid mode simulates how a real MapLibre GL JS
map loads tiles. When a user pans or zooms, the browser requests a **grid of tiles**
simultaneously to fill the viewport. This benchmark measures **time to fill one viewport**
— the metric users actually experience.

### Summary

| Server | Format | Avg Grids/s | Avg Grid Latency | Avg Throughput | Errors |
|--------|--------|-------------|------------------|----------------|--------|
`;

  for (const data of Object.values(summary)) {
    md += `| ${data.server} | ${data.format.toUpperCase()} | ${(data.gridsPerSec / data.count).toFixed(1)} | ${(data.avgLatency / data.count).toFixed(0)}ms | ${formatBytes(data.avgThroughput / data.count)}/grid | ${data.errors} |\n`;
  }

  md += `\n### Detailed Results\n\n`;

  for (const format of ['pmtiles', 'mbtiles', 'postgres', 'postgres_function', 'cog', 'raster', 'ogc', 'stac']) {
    const filtered = results.filter((r) => r.format === format);
    if (filtered.length === 0) continue;

    md += `#### ${format.toUpperCase()}\n\n`;
    md += `| Server | Zoom | Grid | Grids/s | Avg Latency | P50 | P99 | Throughput |\n`;
    md += `|--------|------|------|---------|-------------|-----|-----|------------|\n`;

    for (const r of filtered) {
      md += `| ${r.server} | z${r.zoom} | ${r.grid} | ${r.gridsPerSec.toFixed(1)} | ${r.avg.toFixed(0)}ms | ${r.p50.toFixed(0)}ms | ${r.p99.toFixed(0)}ms | ${formatBytes(r.avgThroughput)}/grid |\n`;
    }
    md += '\n';
  }

  return md;
}

// ─── Utilities ─────────────────────────────────────────────────────────────────

/**
 * Format bytes to human readable
 */
function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

function parseGridSize(sizeStr) {
  const match = sizeStr.match(/^(\d+)x(\d+)$/);
  if (!match) {
    console.error(chalk.red(`Invalid grid size: "${sizeStr}". Use format WxH (e.g., 4x4, 5x4)`));
    process.exit(1);
  }
  return { width: parseInt(match[1]), height: parseInt(match[2]) };
}

// ─── Main ──────────────────────────────────────────────────────────────────────

async function main() {
  program
    .option('-s, --server <server>', 'Server to test: tileserver-rs, martin, tileserver-gl, titiler, or all', 'all')
    .option('-f, --format <format>', 'Alias for --type (legacy)', undefined)
    .option('-t, --type <type>', 'Benchmark type: pmtiles, mbtiles, postgres, postgres_function, cog, raster, ogc, stac, or all', 'all')
    .option('-d, --duration <seconds>', 'Test duration in seconds (single mode)', '10')
    .option('-c, --connections <num>', 'Number of connections (single mode)', '100')
    .option('-m, --mode <mode>', 'Benchmark mode: single (throughput) or grid (viewport simulation)', 'single')
    .option('-g, --grid-size <WxH>', 'Grid dimensions for viewport simulation', '4x4')
    .option('-i, --iterations <num>', 'Number of grid load iterations per zoom level', '50')
    .option('--markdown', 'Output markdown report')
    .parse();

  const opts = program.opts();

  BENCHMARK_CONFIG.duration = parseInt(opts.duration);
  BENCHMARK_CONFIG.connections = parseInt(opts.connections);

  const mode = opts.mode;
  const gridConfig = {
    ...parseGridSize(opts.gridSize),
    iterations: parseInt(opts.iterations),
  };

  if (mode === 'grid') {
    console.log(chalk.bold.cyan('\n🗺️  Tile Server Grid Viewport Benchmark\n'));
    console.log(chalk.gray(`Mode: Grid viewport simulation`));
    console.log(chalk.gray(`Grid: ${gridConfig.width}×${gridConfig.height} (${gridConfig.width * gridConfig.height} tiles/viewport)`));
    console.log(chalk.gray(`Iterations: ${gridConfig.iterations} per zoom (+ 3 warmup)`));
  } else {
    console.log(chalk.bold.cyan('\n🚀 Tile Server Benchmark Suite\n'));
    console.log(chalk.gray(`Mode: Single tile throughput`));
    console.log(chalk.gray(`Duration: ${BENCHMARK_CONFIG.duration}s | Connections: ${BENCHMARK_CONFIG.connections}`));
  }
  console.log(chalk.gray(`Servers: tileserver-gl (8900), tileserver-rs (8901), martin (8902), titiler (8903)\n`));

  const serversToTest = opts.server === 'all' ? Object.keys(SERVERS) : [opts.server];
  const typeArg = opts.type ?? opts.format ?? 'all';
  const formatsToTest =
    typeArg === 'all'
      ? ['pmtiles', 'mbtiles', 'postgres', 'postgres_function', 'cog', 'raster', 'ogc', 'stac']
      : [typeArg];

  // Check server availability
  console.log(chalk.bold('Checking server availability...'));
  const availableServers = [];
  for (const serverId of serversToTest) {
    const server = SERVERS[serverId];
    if (!server) {
      console.log(chalk.red(`  Unknown server: ${serverId}`));
      continue;
    }
    const isAvailable = await checkServer(server.healthUrl);
    if (isAvailable) {
      console.log(chalk.green(`  ✓ ${server.name} available at port ${server.port}`));
      availableServers.push(serverId);
    } else {
      console.log(chalk.red(`  ✗ ${server.name} not available at port ${server.port}`));
    }
  }

  if (availableServers.length === 0) {
    console.log(chalk.red('\nNo servers available. Please start the servers first.'));
    console.log(chalk.gray('\nTo start all servers (Docker):'));
    console.log(chalk.gray('  1. Build tileserver-rs: docker build -t tileserver-rs:latest .'));
    console.log(chalk.gray('  2. Start all: docker compose -f benchmarks/docker-compose.yml up -d'));
    process.exit(1);
  }

  let allResults = [];

  // Run benchmarks
  for (const format of formatsToTest) {
    if (mode === 'grid') {
      console.log(chalk.bold.cyan(`\n🗺️  ${format.toUpperCase()} Grid Benchmarks`));
    } else {
      console.log(chalk.bold.cyan(`\n📦 ${format.toUpperCase()} Benchmarks`));
    }

    for (const serverId of availableServers) {
      let results;
      if (format === 'ogc') {
        results = await benchmarkServerOgc(serverId);
      } else if (format === 'stac') {
        results = await benchmarkServerStac(serverId);
      } else if (mode === 'grid') {
        results = await benchmarkServerGrid(serverId, format, gridConfig);
      } else {
        results = await benchmarkServerFormat(serverId, format);
      }
      allResults = allResults.concat(results);
    }

    if (!opts.markdown) {
      if (mode === 'grid') {
        printGridResults(allResults, format);
      } else {
        printResults(allResults, format);
      }
    }
  }

  if (!opts.markdown) {
    console.log(chalk.bold.cyan('\n📋 Protocol Support Matrix\n'));
    const matrix = new Table({
      head: [chalk.bold('Type'), chalk.bold('tileserver-gl'), chalk.bold('tileserver-rs'), chalk.bold('martin'), chalk.bold('titiler')],
    });
    for (const t of Object.keys(PROTOCOL_SUPPORT)) {
      const row = PROTOCOL_SUPPORT[t];
      matrix.push([t, row['tileserver-gl'], row['tileserver-rs'], row.martin, row.titiler]);
    }
    console.log(matrix.toString());
    console.log(chalk.gray('Legend: ✓ supported · ✗ not supported · ◐ partial / read-only'));
    console.log(chalk.gray('Features as of April 2026. Check competitor docs for latest.'));
  }

  // Print summary / report
  if (opts.markdown) {
    if (mode === 'grid') {
      console.log('\n' + generateGridMarkdownReport(allResults, gridConfig));
    } else {
      console.log('\n' + generateMarkdownReport(allResults));
    }
  } else {
    if (mode === 'grid') {
      printGridSummary(allResults);
    } else {
      printSummary(allResults);
    }
  }

  console.log(chalk.gray('\nDone!\n'));
}

main().catch((err) => {
  console.error(chalk.red('Error:'), err);
  process.exit(1);
});
