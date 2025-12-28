#!/usr/bin/env node
/**
 * Benchmark runner for tileserver-rs vs martin vs tileserver-gl
 *
 * Tests PMTiles and MBTiles performance across all three servers
 *
 * Usage:
 *   node run-benchmarks.js                     # Run all benchmarks
 *   node run-benchmarks.js --server tileserver-rs  # Single server
 *   node run-benchmarks.js --format mbtiles    # MBTiles only
 *   node run-benchmarks.js --duration 30       # 30 second tests
 */

import autocannon from 'autocannon';
import { program } from 'commander';
import chalk from 'chalk';
import Table from 'cli-table3';

// Server configurations
// Use 127.0.0.1 instead of localhost to avoid proxy issues
const SERVERS = {
  'tileserver-rs': {
    name: 'tileserver-rs',
    port: 8000,
    color: chalk.green,
    // PMTiles source endpoints
    pmtiles: {
      source: 'pmtiles',
      tileUrl: (z, x, y) => `http://127.0.0.1:8000/data/pmtiles/${z}/${x}/${y}.pbf`,
    },
    // MBTiles source endpoints
    mbtiles: {
      source: 'mbtiles',
      tileUrl: (z, x, y) => `http://127.0.0.1:8000/data/mbtiles/${z}/${x}/${y}.pbf`,
    },
    healthUrl: 'http://127.0.0.1:8000/health',
  },
  martin: {
    name: 'martin',
    port: 8001,
    color: chalk.blue,
    // Martin uses source name directly in URL
    pmtiles: {
      source: 'protomaps-sample',
      tileUrl: (z, x, y) => `http://127.0.0.1:8001/protomaps-sample/${z}/${x}/${y}`,
    },
    mbtiles: {
      source: 'zurich_switzerland',
      tileUrl: (z, x, y) => `http://127.0.0.1:8001/zurich_switzerland/${z}/${x}/${y}`,
    },
    healthUrl: 'http://127.0.0.1:8001/catalog',
  },
  'tileserver-gl': {
    name: 'tileserver-gl',
    port: 8002,
    color: chalk.yellow,
    // tileserver-gl only supports MBTiles
    pmtiles: null, // Not supported
    mbtiles: {
      source: 'v3',
      tileUrl: (z, x, y) => `http://127.0.0.1:8002/data/v3/${z}/${x}/${y}.pbf`,
    },
    healthUrl: 'http://127.0.0.1:8002/health',
  },
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
// All coordinates verified to return 200 OK with real tile data
const TEST_TILES = {
  pmtiles: [
    // Florence center tiles - verified working
    { z: 10, x: 544, y: 373, desc: 'Florence z10' },
    { z: 11, x: 1088, y: 746, desc: 'Florence z11' },
    { z: 12, x: 2176, y: 1493, desc: 'Florence z12' },
    { z: 13, x: 4352, y: 2986, desc: 'Florence z13' },
    { z: 14, x: 8704, y: 5972, desc: 'Florence z14' },
    { z: 15, x: 17408, y: 11944, desc: 'Florence z15' },
  ],
  mbtiles: [
    // Zurich center tiles - verified working
    { z: 10, x: 536, y: 358, desc: 'Zurich z10' },
    { z: 11, x: 1072, y: 717, desc: 'Zurich z11' },
    { z: 12, x: 2145, y: 1434, desc: 'Zurich z12' },
    { z: 13, x: 4290, y: 2868, desc: 'Zurich z13' },
    { z: 14, x: 8580, y: 5737, desc: 'Zurich z14' },
  ],
};

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
 * Run benchmarks for a server and format
 */
async function benchmarkServerFormat(serverId, format) {
  const server = SERVERS[serverId];
  if (!server) {
    console.log(chalk.red(`Unknown server: ${serverId}`));
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
      const result = await runBenchmark(url, `${server.name} ${format} z${tile.z}`);

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
        serverId,
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

/**
 * Print results table
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
 * Print summary comparison
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

/**
 * Generate markdown report
 */
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
- Duration: ${BENCHMARK_CONFIG.duration} seconds per endpoint
- Connections: ${BENCHMARK_CONFIG.connections} concurrent
- Date: ${new Date().toISOString().split('T')[0]}

### Summary

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

  for (const format of ['pmtiles', 'mbtiles']) {
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
    .option('-s, --server <server>', 'Server to test: tileserver-rs, martin, tileserver-gl, or all', 'all')
    .option('-f, --format <format>', 'Format to test: pmtiles, mbtiles, or all', 'all')
    .option('-d, --duration <seconds>', 'Test duration in seconds', '10')
    .option('-c, --connections <num>', 'Number of connections', '100')
    .option('--markdown', 'Output markdown report')
    .parse();

  const opts = program.opts();

  BENCHMARK_CONFIG.duration = parseInt(opts.duration);
  BENCHMARK_CONFIG.connections = parseInt(opts.connections);

  console.log(chalk.bold.cyan('\n🚀 Tile Server Benchmark Suite\n'));
  console.log(chalk.gray(`Duration: ${BENCHMARK_CONFIG.duration}s | Connections: ${BENCHMARK_CONFIG.connections}`));
  console.log(chalk.gray(`Servers: tileserver-rs (8000), martin (8001), tileserver-gl (8002)\n`));

  // Determine which servers to test
  const serversToTest = opts.server === 'all' ? Object.keys(SERVERS) : [opts.server];
  const formatsToTest = opts.format === 'all' ? ['pmtiles', 'mbtiles'] : [opts.format];

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
    console.log(chalk.gray('\nTo start servers:'));
    console.log(chalk.gray('  tileserver-rs: ./target/release/tileserver-rs --config config.benchmark.toml --port 8000'));
    console.log(chalk.gray('  martin + tileserver-gl: cd benchmarks && docker compose up -d'));
    process.exit(1);
  }

  let allResults = [];

  // Run benchmarks
  for (const format of formatsToTest) {
    console.log(chalk.bold.cyan(`\n📦 ${format.toUpperCase()} Benchmarks`));

    for (const serverId of availableServers) {
      const results = await benchmarkServerFormat(serverId, format);
      allResults = allResults.concat(results);
    }

    if (!opts.markdown) {
      printResults(allResults, format);
    }
  }

  // Print summary
  if (opts.markdown) {
    console.log('\n' + generateMarkdownReport(allResults));
  } else {
    printSummary(allResults);
  }

  console.log(chalk.gray('\nDone!\n'));
}

main().catch((err) => {
  console.error(chalk.red('Error:'), err);
  process.exit(1);
});
