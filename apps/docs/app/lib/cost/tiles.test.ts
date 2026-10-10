import { describe, expect, it } from 'vitest';
import type { CostRow, TileInputs } from '~/types/cost';
import { tileRows } from './tiles';

// The Protomaps cost calculator defaults: 625,000 sessions × 16 tiles =
// 10,000,000 tile requests; 100 KB tiles = 1,000 GB; 50% cache hits; 110 GB.
const protomaps: TileInputs = {
  sessions: 625_000,
  tilesPerSession: 16,
  tileKb: 100,
  cacheHitPct: 50,
  storageGb: 110,
  includeFree: false,
  servers: 1,
};

function row(rows: CostRow[], id: string): CostRow {
  const found = rows.find((r) => r.id === id);
  if (!found) {
    throw new Error(`no row ${id}`);
  }
  return found;
}

function monthly(rows: CostRow[], id: string): number {
  const { estimate } = row(rows, id);
  if (estimate.kind !== 'usd') {
    throw new Error(`${id} is ${estimate.kind}`);
  }
  return estimate.monthly;
}

describe('tileRows, Protomaps defaults, free tiers off', () => {
  const rows = tileRows(protomaps);

  it('prices Cloudflare like the Protomaps calculator', () => {
    // 10M requests × $0.30/M = 3.00; plan 5.00; 5M R2 reads × $0.36/M = 1.80;
    // 110 GB × $0.015 = 1.65. Protomaps shows $11.45.
    expect(monthly(rows, 'cloudflare')).toBeCloseTo(11.45, 6);
  });

  it('prices AWS from the current price list', () => {
    // CloudFront 10M / 10k × $0.01 = 10.00; 1,000 GB × $0.085 = 85.00;
    // Lambda 5M × $0.0000002 = 1.00; 5M × 0.2 s × 0.5 GB = 500,000 GB-s ×
    // $0.0000133334 = 6.6667; S3 GET 5M / 1k × $0.0004 = 2.00;
    // S3 110 GB × $0.023 = 2.53.
    expect(monthly(rows, 'aws')).toBeCloseTo(107.1967, 4);
  });

  it('prices one Hetzner AX102 with its IPv4 address', () => {
    // 302.10 + 1.90
    expect(monthly(rows, 'hetzner')).toBeCloseTo(304, 6);
  });

  it('bills Google map loads through its bands', () => {
    // 100k × $7 + 400k × $5.60 + 125k × $4.20 = 700 + 2,240 + 525
    expect(monthly(rows, 'google')).toBeCloseTo(3_465, 6);
  });

  it('bills Mapbox map loads through its bands', () => {
    // 100k × $5 + 100k × $4 + 425k × $3 = 500 + 400 + 1,275
    expect(monthly(rows, 'mapbox')).toBeCloseTo(2_175, 6);
  });

  it('bills MapTiler sessions above the Flex allowance', () => {
    // $30 + 600k extra sessions × $2.50/1k
    expect(monthly(rows, 'maptiler')).toBeCloseTo(1_530, 6);
  });

  it('picks the cheapest Stadia plan for 10M tile credits', () => {
    // Starter 20 + 9M × $0.03/1k = 290; Standard 80 + 2.5M × $0.02/1k = 130;
    // Professional 250.
    expect(monthly(rows, 'stadia')).toBeCloseTo(130, 6);
  });

  it('picks the Geoapify plan that covers the daily credits', () => {
    // 10M tiles × 0.25 = 2.5M credits / 30 days = 83,334 per day → API 100.
    expect(monthly(rows, 'geoapify')).toBeCloseTo(299, 6);
  });

  it('bills HERE vector tiles through its bands', () => {
    // 5M × $0.088/1k + 5M × $0.070/1k = 440 + 350
    expect(monthly(rows, 'here')).toBeCloseTo(790, 6);
  });
});

describe('tileRows, free tiers on', () => {
  const rows = tileRows({ ...protomaps, includeFree: true });

  it('removes the Workers and R2 allowances', () => {
    // requests (10M − 10M) = 0; plan 5.00; reads 5M ≤ 10M free = 0;
    // storage (110 − 10) × $0.015 = 1.50
    expect(monthly(rows, 'cloudflare')).toBeCloseTo(6.5, 6);
  });

  it('removes the CloudFront and Lambda allowances', () => {
    // CloudFront requests 0; bandwidth (1,000 − 1,000) GB = 0;
    // Lambda 4M × $0.0000002 = 0.80; (500,000 − 400,000) GB-s × $0.0000133334
    // = 1.33334; S3 GET 2.00; S3 storage 2.53
    expect(monthly(rows, 'aws')).toBeCloseTo(6.66334, 5);
  });

  it('removes Google’s 10,000 free loads', () => {
    // 90k × $7 + 400k × $5.60 + 125k × $4.20 = 630 + 2,240 + 525
    expect(monthly(rows, 'google')).toBeCloseTo(3_395, 6);
  });

  it('removes Mapbox’s 50,000 free loads', () => {
    // 50k × $5 + 100k × $4 + 425k × $3 = 250 + 400 + 1,275
    expect(monthly(rows, 'mapbox')).toBeCloseTo(1_925, 6);
  });

  it('removes HERE’s 30,000 free tiles', () => {
    // 4.97M × $0.088/1k + 5M × $0.070/1k = 437.36 + 350
    expect(monthly(rows, 'here')).toBeCloseTo(787.36, 6);
  });
});

describe('tileRows, other cases', () => {
  it('multiplies the Hetzner cost by the server count', () => {
    expect(
      monthly(tileRows({ ...protomaps, servers: 2 }), 'hetzner'),
    ).toBeCloseTo(608, 6);
  });

  it('answers contact sales when Mapbox loads pass 5M', () => {
    const rows = tileRows({ ...protomaps, sessions: 6_000_000 });
    expect(row(rows, 'mapbox').estimate.kind).toBe('contact');
  });

  it('shows the one-time Hetzner setup fee per server', () => {
    const { estimate } = row(tileRows({ ...protomaps, servers: 2 }), 'hetzner');
    expect(estimate.kind === 'usd' ? estimate.setup : undefined).toBe(298);
  });

  it('lists the four option groups', () => {
    const groups = new Set(tileRows(protomaps).map((r) => r.group));
    expect([...groups].sort()).toEqual(['aws', 'edge', 'self-host', 'vendor']);
  });
});
