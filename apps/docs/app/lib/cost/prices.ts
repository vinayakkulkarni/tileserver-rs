import type { CostSource, PricePlan, PriceTier, TieredPrice } from '~/types/cost';

/**
 * Unit prices for the cost calculator, read from each vendor's own pricing
 * page on 2 Oct 2026. All prices are USD.
 */

/** The date every price on this page was read. */
export const pricesReadOn = '2 Oct 2026';

const google: CostSource = {
  label: 'Google Maps Platform pricing',
  url: 'https://developers.google.com/maps/billing-and-pricing/pricing',
};
const mapbox: CostSource = {
  label: 'Mapbox pricing',
  url: 'https://www.mapbox.com/pricing',
};
const maptiler: CostSource = {
  label: 'MapTiler Cloud pricing',
  url: 'https://www.maptiler.com/cloud/pricing/',
};
const stadia: CostSource = {
  label: 'Stadia Maps pricing',
  url: 'https://stadiamaps.com/pricing/',
};
const geoapify: CostSource = {
  label: 'Geoapify pricing',
  url: 'https://www.geoapify.com/pricing/',
};
const here: CostSource = {
  label: 'HERE pricing',
  url: 'https://www.here.com/get-started/pricing',
};
const workers: CostSource = {
  label: 'Cloudflare Workers pricing',
  url: 'https://developers.cloudflare.com/workers/platform/pricing/',
};
const r2: CostSource = {
  label: 'Cloudflare R2 pricing',
  url: 'https://developers.cloudflare.com/r2/pricing/',
};
const lambda: CostSource = {
  label: 'AWS Lambda pricing',
  url: 'https://aws.amazon.com/lambda/pricing/',
};
const s3: CostSource = {
  label: 'Amazon S3 pricing',
  url: 'https://aws.amazon.com/s3/pricing/',
};
const cloudfront: CostSource = {
  label: 'Amazon CloudFront pricing',
  url: 'https://aws.amazon.com/cloudfront/pricing/',
};
const hetzner: CostSource = {
  label: 'Hetzner AX102 configurator',
  url: 'https://www.hetzner.com/dedicated-rootserver/ax102/configurator/',
};

export const sources = {
  google,
  mapbox,
  maptiler,
  stadia,
  geoapify,
  here,
  workers,
  r2,
  lambda,
  s3,
  cloudfront,
  hetzner,
} as const;

/** Google's five standard bands: to 100k, 500k, 1M, 5M, then above. */
function googleTiers(
  prices: readonly [number, number, number, number, number],
): PriceTier[] {
  return [
    { upTo: 100_000, per1k: prices[0] },
    { upTo: 500_000, per1k: prices[1] },
    { upTo: 1_000_000, per1k: prices[2] },
    { upTo: 5_000_000, per1k: prices[3] },
    { upTo: null, per1k: prices[4] },
  ];
}

export const googlePrices = {
  /** Dynamic Maps, Essentials (FAF4-3B2D-51B2), per map load. */
  dynamicMaps: {
    free: 10_000,
    tiers: googleTiers([7, 5.6, 4.2, 2.1, 0.53]),
    source: google,
  },
} satisfies Record<string, TieredPrice>;

export const mapboxPrices = {
  /** Mapbox GL JS map loads. */
  mapLoads: {
    free: 50_000,
    tiers: [
      { upTo: 100_000, per1k: 5 },
      { upTo: 200_000, per1k: 4 },
      { upTo: 1_000_000, per1k: 3 },
      { upTo: 5_000_000, per1k: 2.5 },
      { upTo: null, per1k: null },
    ],
    source: mapbox,
  },
} satisfies Record<string, TieredPrice>;

export const herePrices = {
  vectorTile: {
    free: 30_000,
    tiers: [
      { upTo: 5_000_000, per1k: 0.088 },
      { upTo: 10_000_000, per1k: 0.07 },
      { upTo: null, per1k: null },
    ],
    source: here,
  },
} satisfies Record<string, TieredPrice>;

/** MapTiler Cloud Flex plan; the Free plan is non-commercial. */
export const maptilerFlex = {
  monthly: 30,
  sessions: { included: 25_000, overPer1k: 2.5 },
  source: maptiler,
} as const;

/** Stadia Maps paid plans in credits; the Free plan is non-commercial. */
export const stadiaPlans: readonly PricePlan[] = [
  { name: 'Starter', monthly: 20, included: 1_000_000, overPer1k: 0.03 },
  { name: 'Standard', monthly: 80, included: 7_500_000, overPer1k: 0.02 },
  {
    name: 'Professional',
    monthly: 250,
    included: 25_000_000,
    overPer1k: 0.015,
  },
];

/** Stadia credits per request. */
export const stadiaCredits = {
  tile: 1,
} as const;

/**
 * Geoapify plans; `included` is credits per day. Geoapify publishes no
 * overage price, so usage above the largest plan is "contact sales".
 */
export const geoapifyPlans: readonly PricePlan[] = [
  { name: 'API 10', monthly: 59, included: 10_000, overPer1k: null },
  { name: 'API 25', monthly: 109, included: 25_000, overPer1k: null },
  { name: 'API 50', monthly: 179, included: 50_000, overPer1k: null },
  { name: 'API 100', monthly: 299, included: 100_000, overPer1k: null },
  { name: 'API 250', monthly: 609, included: 250_000, overPer1k: null },
];

/** Geoapify Free plan: credits per day, commercial use allowed. */
export const geoapifyFreePerDay = 3_000;

/** Geoapify credits per request. */
export const geoapifyCredits = { tile: 0.25 } as const;

export const cloudflarePrices = {
  workersMonthly: 5,
  requestsPerMillion: 0.3,
  requestsIncluded: 10_000_000,
  r2ReadsPerMillion: 0.36,
  r2ReadsFree: 10_000_000,
  r2StoragePerGb: 0.015,
  r2StorageFreeGb: 10,
} as const;

export const awsPrices = {
  /** CloudFront HTTPS requests, United States. */
  cloudfrontPer10k: 0.01,
  cloudfrontRequestsFree: 10_000_000,
  /** CloudFront data out to the internet, per GB, by cumulative band. */
  cloudfrontGb: [
    { upTo: 10_000, perGb: 0.085 },
    { upTo: 50_000, perGb: 0.08 },
    { upTo: 150_000, perGb: 0.06 },
    { upTo: 500_000, perGb: 0.04 },
    { upTo: 1_024_000, perGb: 0.03 },
    { upTo: 5_024_000, perGb: 0.025 },
    { upTo: null, perGb: 0.02 },
  ],
  cloudfrontGbFree: 1_000,
  lambdaPerRequest: 0.0000002,
  lambdaRequestsFree: 1_000_000,
  /** arm64, first 7.5 billion GB-seconds. */
  lambdaPerGbSecond: 0.0000133334,
  lambdaGbSecondsFree: 400_000,
  s3GetPer1k: 0.0004,
  s3StoragePerGb: 0.023,
} as const;

/** Hetzner AX102-1 in Germany or Finland. */
export const hetznerAx102 = {
  monthly: 302.1,
  ipv4Monthly: 1.9,
  setup: 149,
  source: hetzner,
} as const;
