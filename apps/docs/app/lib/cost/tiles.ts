import {
  aboveFree,
  cheapestPlan,
  flatLine,
  geoapifyEstimate,
  hetznerEstimate,
  tieredLine,
  total,
} from './build';
import type { CostEstimate, CostLine, CostRow, TileInputs } from '~/types/cost';
import {
  awsPrices,
  cloudflarePrices,
  geoapifyCredits,
  googlePrices,
  herePrices,
  mapboxPrices,
  maptilerFlex,
  sources,
  stadiaCredits,
  stadiaPlans,
} from './prices';

/**
 * The Lambda size and run time the Protomaps calculator assumes for one
 * tile request: 512 MB for 200 ms.
 */
export const protomapsLambda = { memoryGb: 0.5, seconds: 0.2 } as const;

function cloudflare(
  requests: number,
  misses: number,
  input: TileInputs,
): CostEstimate {
  const p = cloudflarePrices;
  const free = input.includeFree;
  return total(
    [
      flatLine(
        'Workers Paid plan',
        1,
        'month',
        p.workersMonthly,
        sources.workers,
      ),
      flatLine(
        'Workers requests',
        aboveFree(requests, p.requestsIncluded, free),
        'requests',
        p.requestsPerMillion / 1e6,
        sources.workers,
      ),
      flatLine(
        'R2 reads (cache misses)',
        aboveFree(misses, p.r2ReadsFree, free),
        'reads',
        p.r2ReadsPerMillion / 1e6,
        sources.r2,
      ),
      flatLine(
        'R2 storage',
        aboveFree(input.storageGb, p.r2StorageFreeGb, free),
        'GB',
        p.r2StoragePerGb,
        sources.r2,
      ),
    ],
    sources.r2,
  );
}

/** CloudFront data out, billed band by band (1 TB = 1,000 GB here). */
function cloudfrontBandwidth(gb: number): number {
  let cost = 0;
  let lower = 0;
  for (const band of awsPrices.cloudfrontGb) {
    const upper = band.upTo ?? Number.POSITIVE_INFINITY;
    const inBand = Math.min(gb, upper) - lower;
    if (inBand > 0) {
      cost += inBand * band.perGb;
    }
    lower = upper;
  }
  return cost;
}

function aws(
  requests: number,
  misses: number,
  gb: number,
  input: TileInputs,
): CostEstimate {
  const p = awsPrices;
  const free = input.includeFree;
  const billedGb = aboveFree(gb, p.cloudfrontGbFree, free);
  const gbSeconds = misses * protomapsLambda.seconds * protomapsLambda.memoryGb;
  const lines: CostLine[] = [
    flatLine(
      'CloudFront HTTPS requests',
      aboveFree(requests, p.cloudfrontRequestsFree, free),
      'requests',
      p.cloudfrontPer10k / 10_000,
      sources.cloudfront,
    ),
    {
      label: 'CloudFront data out',
      units: billedGb,
      unit: 'GB',
      cost: cloudfrontBandwidth(billedGb),
      source: sources.cloudfront,
    },
    flatLine(
      'Lambda requests',
      aboveFree(misses, p.lambdaRequestsFree, free),
      'requests',
      p.lambdaPerRequest,
      sources.lambda,
    ),
    flatLine(
      'Lambda duration',
      aboveFree(gbSeconds, p.lambdaGbSecondsFree, free),
      'GB-seconds',
      p.lambdaPerGbSecond,
      sources.lambda,
    ),
    flatLine(
      'S3 GET requests',
      misses,
      'requests',
      p.s3GetPer1k / 1_000,
      sources.s3,
    ),
    flatLine('S3 storage', input.storageGb, 'GB', p.s3StoragePerGb, sources.s3),
  ];
  return total(lines, sources.cloudfront);
}

function maptiler(sessions: number): CostEstimate {
  const f = maptilerFlex;
  return total(
    [
      flatLine('Flex plan', 1, 'month', f.monthly, f.source),
      flatLine(
        'Extra map sessions',
        Math.max(0, sessions - f.sessions.included),
        'sessions',
        f.sessions.overPer1k / 1_000,
        f.source,
      ),
    ],
    f.source,
  );
}

function stadia(tiles: number): CostEstimate {
  const credits = tiles * stadiaCredits.tile;
  const best = cheapestPlan(stadiaPlans, credits);
  return best === null
    ? {
        kind: 'contact',
        reason: 'No published plan covers this usage.',
        source: sources.stadia,
      }
    : total(
        [
          {
            label: `${best.plan.name} plan`,
            units: credits,
            unit: 'credits',
            cost: best.cost,
            source: sources.stadia,
          },
        ],
        sources.stadia,
      );
}

/** Monthly cost of serving map tiles for every option. */
export function tileRows(input: TileInputs): CostRow[] {
  const requests = input.sessions * input.tilesPerSession;
  const misses = requests * (1 - input.cacheHitPct / 100);
  const gb = (requests * input.tileKb) / 1e6;
  const free = input.includeFree;
  return [
    {
      id: 'cloudflare',
      group: 'edge',
      name: 'Cloudflare: R2 + Workers',
      basis:
        'PMTiles on R2; a Worker reads tile ranges. The Protomaps cost model.',
      estimate: cloudflare(requests, misses, input),
      latency: 'edge',
    },
    {
      id: 'hetzner',
      group: 'self-host',
      name: 'Hetzner AX102 + tileserver-rs',
      basis:
        'Dedicated server in Germany or Finland. Traffic is unmetered at 1 Gbit/s.',
      estimate: hetznerEstimate(input.servers),
      latency: 'hetzner',
    },
    {
      id: 'aws',
      group: 'aws',
      name: 'AWS: S3 + Lambda + CloudFront',
      basis:
        'PMTiles on S3; Lambda reads tile ranges. Lambda at 512 MB for 200 ms, as Protomaps assumes.',
      estimate: aws(requests, misses, gb, input),
      latency: 'unmeasured',
    },
    {
      id: 'google',
      group: 'vendor',
      name: 'Google Maps',
      basis: 'Dynamic Maps, billed per map load.',
      estimate: total(
        [
          tieredLine(
            'Dynamic Maps loads',
            input.sessions,
            'loads',
            googlePrices.dynamicMaps,
            free,
          ),
        ],
        sources.google,
      ),
      latency: 'unmeasured',
    },
    {
      id: 'mapbox',
      group: 'vendor',
      name: 'Mapbox',
      basis: 'Mapbox GL JS, billed per map load. Tile requests are included.',
      estimate: total(
        [
          tieredLine(
            'GL JS map loads',
            input.sessions,
            'loads',
            mapboxPrices.mapLoads,
            free,
          ),
        ],
        sources.mapbox,
      ),
      latency: 'unmeasured',
    },
    {
      id: 'maptiler',
      group: 'vendor',
      name: 'MapTiler Cloud',
      basis:
        'Flex plan: 25,000 map sessions included, then billed per session.',
      estimate: maptiler(input.sessions),
      latency: 'unmeasured',
    },
    {
      id: 'stadia',
      group: 'vendor',
      name: 'Stadia Maps',
      basis: 'One credit per vector tile, on the cheapest paid plan.',
      estimate: stadia(requests),
      latency: 'unmeasured',
    },
    {
      id: 'geoapify',
      group: 'vendor',
      name: 'Geoapify',
      basis:
        'Four tiles per credit, on the plan that covers the daily credits.',
      estimate: geoapifyEstimate(requests * geoapifyCredits.tile, free),
      latency: 'unmeasured',
    },
    {
      id: 'here',
      group: 'vendor',
      name: 'HERE',
      basis: 'Vector tiles, billed per tile.',
      estimate: total(
        [
          tieredLine(
            'Vector tiles',
            requests,
            'tiles',
            herePrices.vectorTile,
            free,
          ),
        ],
        sources.here,
      ),
      latency: 'unmeasured',
    },
  ];
}
