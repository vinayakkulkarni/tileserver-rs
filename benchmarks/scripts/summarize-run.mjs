#!/usr/bin/env node
// Summarise a run-quiet.sh results directory into markdown tables.
// Usage: node scripts/summarize-run.mjs results/<timestamp>
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const dir = process.argv[2];
if (!dir) {
  console.error('usage: summarize-run.mjs <results-dir>');
  process.exit(1);
}

const strip = (s) => s.replace(/\x1b\[[0-9;]*m/g, '');

for (const file of readdirSync(dir).filter((f) => f.endsWith('.log') && f !== 'stability.log').sort()) {
  const lines = strip(readFileSync(join(dir, file), 'utf8')).split('\n');
  const rows = [];
  let server = '';
  // label = the case line printed before each autocannon run, e.g. "z10 (Florence z10)"
  let label = '';
  let p99 = '';
  for (const line of lines) {
    const s = line.match(/Testing (\S+) \(/);
    if (s) server = s[1];
    const z = line.match(/^ {4}(\S.*?)\s*\.\.\.\s*$/);
    if (z && !line.includes('Testing')) {
      label = z[1];
      p99 = '';
    }
    const lat = line.match(/^│ Latency\s*│([^│]*)│([^│]*)│([^│]*)│([^│]*)│/);
    if (lat) p99 = lat[4].trim();
    const r = line.match(/^(\d+) req\/s, ([\d.]+)ms avg(.*)$/);
    if (r) rows.push({ server, label, rps: Number(r[1]), avg: Number(r[2]), p99, extra: r[3].trim() });
  }
  console.log(`\n### ${file.replace('.log', '')}\n`);
  console.log('| Server | Case | Req/s | Avg | P99 | Extra |');
  console.log('| --- | --- | ---: | ---: | ---: | --- |');
  for (const r of rows) {
    console.log(`| ${r.server} | ${r.label} | ${r.rps.toLocaleString('en-US')} | ${r.avg.toFixed(1)}ms | ${r.p99} | ${r.extra.replace(/^,\s*/, '')} |`);
  }
}
