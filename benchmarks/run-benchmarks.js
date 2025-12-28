#!/usr/bin/env node
/**
 * Benchmark runner for tileserver-rs vs martin vs tileserver-gl
 *
 * Usage:
 *   node run-benchmarks.js                    # Run all benchmarks
 *   node run-benchmarks.js --type vector      # Vector tiles only
 *   node run-benchmarks.js --type raster      # Raster tiles only
 *   node run-benchmarks.js --server tileserver-rs  # Single server
 */

import autocannon from 'autocannon';
import { program } from 'commander';
import chalk from 'chalk';
import Table from 'cli-table3';

// Server configurations
const SERVERS = {
  'tileserver-rs': {
    name: 'tileserver-rs',
    baseUrl: 'http://localhost:8080',
    vectorTile: '/data/{source}/{z}/{x}/{y}.pbf',
    rasterTile: '/styles/{style}/{z}/{x}/{y}.png',
    color: chalk.green,
  },
  martin: {
    name: 'martin',
    baseUrl: 'http://localhost:3000',
    vectorTile: '/{source}/{z}/{x}/{y}',
    rasterTile: null, // Martin doesn't support raster rendering
    color: chalk.blue,
  },
  'tileserver-gl': {
    name: 'tileserver-gl',
    baseUrl: 'http://localhost:8081',
    vectorTile: '/data/{source}/{z}/{x}/{y}.pbf',
    rasterTile: '/styles/{style}/{z}/{x}/{y}.png',
    color: chalk.yellow,
  },
};

// Test tile coordinates (various zoom levels)
const TEST_TILES = [
  // Low zoom - world overview
  { z: 0, x: 0, y: 0 },
  { z: 2, x: 2, y: 1 },
  // Medium zoom - country level
  { z: 6, x: 32, y: 22 },
  { z: 8, x: 132, y: 85 },
  // High zoom - city level
  { z: 10, x: 529, y: 342 },
  { z: 12, x: 2116, y: 1369 },
  // Very high zoom - street level
  { z: 14, x: 8465, y: 5477 },
];

// Benchmark configuration
const BENCHMARK_CONFIG = {
  duration: 10, // seconds
  connections: 10,
  pipelining: 1,
  timeout: 30,
};

/**
 * Run autocannon benchmark
 */
async function runBenchmark(url, name) {
  return new Promise((resolve, reject) => {
    const instance = autocannon(
      {
        url,
        ...BENCHMARK_CONFIG,
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
async function checkServer(baseUrl) {
  try {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 2000);

    const response = await fetch(`${baseUrl}/health`, {
      signal: controller.signal,
    });
    clearTimeout(timeout);
    return response.ok;
  } catch {
    return false;
  }
}

/**
 * Run vector tile benchmarks
 */
async function benchmarkVectorTiles(servers, source = 'protomaps') {
  console.log(chalk.bold('\n📦 Vector Tile Benchmarks\n'));

  const results = [];

  for (const [serverId, server] of Object.entries(servers)) {
    if (!server.vectorTile) {
      console.log(chalk.gray(`  ${server.name}: No vector tile support, skipping`));
      continue;
    }

    const isAvailable = await checkServer(server.baseUrl);
    if (!isAvailable) {
      console.log(chalk.red(`  ${server.name}: Server not available at ${server.baseUrl}`));
      continue;
    }

    console.log(server.color(`  Testing ${server.name}...`));

    // Test each zoom level
    for (const tile of TEST_TILES) {
      const path = server.vectorTile
        .replace('{source}', source)
        .replace('{z}', tile.z)
        .replace('{x}', tile.x)
        .replace('{y}', tile.y);

      const url = `${server.baseUrl}${path}`;

      try {
        const result = await runBenchmark(url, `${server.name} z${tile.z}`);
        results.push({
          server: server.name,
          type: 'vector',
          zoom: tile.z,
          requests: result.requests.total,
          throughput: result.throughput.total,
          latencyAvg: result.latency.average,
          latencyP99: result.latency.p99,
          errors: result.errors,
        });
      } catch (err) {
        console.log(chalk.red(`    Error at z${tile.z}: ${err.message}`));
      }
    }
  }

  return results;
}

/**
 * Run raster tile benchmarks
 */
async function benchmarkRasterTiles(servers, style = 'protomaps') {
  console.log(chalk.bold('\n🖼️  Raster Tile Benchmarks\n'));

  const results = [];

  for (const [serverId, server] of Object.entries(servers)) {
    if (!server.rasterTile) {
      console.log(chalk.gray(`  ${server.name}: No raster tile support, skipping`));
      continue;
    }

    const isAvailable = await checkServer(server.baseUrl);
    if (!isAvailable) {
      console.log(chalk.red(`  ${server.name}: Server not available at ${server.baseUrl}`));
      continue;
    }

    console.log(server.color(`  Testing ${server.name}...`));

    // Test each zoom level (fewer for raster since it's slower)
    const rasterTiles = TEST_TILES.filter((t) => t.z <= 12);

    for (const tile of rasterTiles) {
      const path = server.rasterTile
        .replace('{style}', style)
        .replace('{z}', tile.z)
        .replace('{x}', tile.x)
        .replace('{y}', tile.y);

      const url = `${server.baseUrl}${path}`;

      try {
        const result = await runBenchmark(url, `${server.name} z${tile.z}`);
        results.push({
          server: server.name,
          type: 'raster',
          zoom: tile.z,
          requests: result.requests.total,
          throughput: result.throughput.total,
          latencyAvg: result.latency.average,
          latencyP99: result.latency.p99,
          errors: result.errors,
        });
      } catch (err) {
        console.log(chalk.red(`    Error at z${tile.z}: ${err.message}`));
      }
    }
  }

  return results;
}

/**
 * Print results table
 */
function printResults(results, type) {
  if (results.length === 0) {
    console.log(chalk.yellow('\nNo results to display'));
    return;
  }

  const table = new Table({
    head: [
      chalk.bold('Server'),
      chalk.bold('Zoom'),
      chalk.bold('Req/sec'),
      chalk.bold('Throughput'),
      chalk.bold('Latency (avg)'),
      chalk.bold('Latency (p99)'),
      chalk.bold('Errors'),
    ],
    colAligns: ['left', 'right', 'right', 'right', 'right', 'right', 'right'],
  });

  const filteredResults = results.filter((r) => r.type === type);

  // Group by zoom level for comparison
  const byZoom = {};
  for (const r of filteredResults) {
    if (!byZoom[r.zoom]) byZoom[r.zoom] = [];
    byZoom[r.zoom].push(r);
  }

  for (const [zoom, zoomResults] of Object.entries(byZoom)) {
    // Sort by requests per second (best first)
    zoomResults.sort((a, b) => b.requests / BENCHMARK_CONFIG.duration - a.requests / BENCHMARK_CONFIG.duration);

    for (const r of zoomResults) {
      const server = SERVERS[r.server.toLowerCase().replace('-', '')] || SERVERS['tileserver-rs'];
      const colorFn = server?.color || chalk.white;

      table.push([
        colorFn(r.server),
        `z${r.zoom}`,
        (r.requests / BENCHMARK_CONFIG.duration).toFixed(0),
        formatBytes(r.throughput / BENCHMARK_CONFIG.duration) + '/s',
        r.latencyAvg.toFixed(2) + 'ms',
        r.latencyP99.toFixed(2) + 'ms',
        r.errors || 0,
      ]);
    }

    // Add separator between zoom levels
    if (Object.keys(byZoom).indexOf(zoom) < Object.keys(byZoom).length - 1) {
      table.push([{ colSpan: 7, content: '', hAlign: 'center' }]);
    }
  }

  console.log(`\n${chalk.bold(type.charAt(0).toUpperCase() + type.slice(1) + ' Tile Results:')}`);
  console.log(table.toString());
}

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

/**
 * Main
 */
async function main() {
  program
    .option('-t, --type <type>', 'Benchmark type: vector, raster, or all', 'all')
    .option('-s, --server <server>', 'Single server to test')
    .option('-d, --duration <seconds>', 'Test duration in seconds', '10')
    .option('-c, --connections <num>', 'Number of connections', '10')
    .option('--source <source>', 'Data source name', 'protomaps')
    .option('--style <style>', 'Style name for raster', 'protomaps')
    .parse();

  const opts = program.opts();

  BENCHMARK_CONFIG.duration = parseInt(opts.duration);
  BENCHMARK_CONFIG.connections = parseInt(opts.connections);

  console.log(chalk.bold.cyan('\n🚀 Tileserver Benchmark Suite\n'));
  console.log(chalk.gray(`Duration: ${BENCHMARK_CONFIG.duration}s | Connections: ${BENCHMARK_CONFIG.connections}`));

  // Filter servers if specified
  let servers = SERVERS;
  if (opts.server) {
    const key = opts.server.toLowerCase().replace('-', '');
    if (SERVERS[key]) {
      servers = { [key]: SERVERS[key] };
    } else if (SERVERS[opts.server]) {
      servers = { [opts.server]: SERVERS[opts.server] };
    } else {
      console.log(chalk.red(`Unknown server: ${opts.server}`));
      console.log(chalk.gray(`Available: ${Object.keys(SERVERS).join(', ')}`));
      process.exit(1);
    }
  }

  let allResults = [];

  // Run benchmarks
  if (opts.type === 'all' || opts.type === 'vector') {
    const vectorResults = await benchmarkVectorTiles(servers, opts.source);
    allResults = allResults.concat(vectorResults);
    printResults(allResults, 'vector');
  }

  if (opts.type === 'all' || opts.type === 'raster') {
    const rasterResults = await benchmarkRasterTiles(servers, opts.style);
    allResults = allResults.concat(rasterResults);
    printResults(allResults, 'raster');
  }

  // Summary
  console.log(chalk.bold.cyan('\n📊 Summary\n'));

  const serverTotals = {};
  for (const r of allResults) {
    if (!serverTotals[r.server]) {
      serverTotals[r.server] = { requests: 0, throughput: 0, count: 0 };
    }
    serverTotals[r.server].requests += r.requests;
    serverTotals[r.server].throughput += r.throughput;
    serverTotals[r.server].count++;
  }

  for (const [server, totals] of Object.entries(serverTotals)) {
    const avgReqSec = totals.requests / totals.count / BENCHMARK_CONFIG.duration;
    const avgThroughput = totals.throughput / totals.count / BENCHMARK_CONFIG.duration;
    console.log(
      `  ${server}: ${chalk.green(avgReqSec.toFixed(0))} req/s avg, ${chalk.blue(formatBytes(avgThroughput))}/s throughput`
    );
  }

  console.log(chalk.gray('\nDone!\n'));
}

main().catch((err) => {
  console.error(chalk.red('Error:'), err);
  process.exit(1);
});
