/** Where a price comes from. */
export interface CostSource {
  label: string;
  url: string;
}

/**
 * One price band. `upTo` is the cumulative monthly unit count where the band
 * ends (`null` = no end). `per1k` is the price per 1,000 units; `null` means
 * the vendor asks you to contact sales.
 */
export interface PriceTier {
  upTo: number | null;
  per1k: number | null;
}

/** A tiered price list with a monthly free allowance at the bottom. */
export interface TieredPrice {
  free: number;
  tiers: readonly PriceTier[];
  source: CostSource;
}

/** A monthly plan with included units and a price for extra units. */
export interface PricePlan {
  name: string;
  monthly: number;
  included: number;
  overPer1k: number | null;
}

/** One priced item inside an estimate. */
export interface CostLine {
  label: string;
  units: number;
  unit: string;
  cost: number;
  source: CostSource;
}

/** A monthly cost, or a "contact sales" answer. */
export type CostEstimate =
  | { kind: 'usd'; monthly: number; lines: CostLine[]; setup?: number }
  | { kind: 'contact'; reason: string; source: CostSource };

/** The column group a calculator row sits in. */
export type CostRowGroup = 'edge' | 'self-host' | 'aws' | 'vendor';

/** How the latency column describes a row. */
export type CostLatency = 'edge' | 'hetzner' | 'unmeasured';

/** One hosting option in the calculator table. */
export interface CostRow {
  id: string;
  group: CostRowGroup;
  name: string;
  basis: string;
  estimate: CostEstimate;
  latency: CostLatency;
}

/** What a site owner enters on the tiles calculator. */
export interface TileInputs {
  /** Map viewer sessions (map loads) per month. */
  sessions: number;
  tilesPerSession: number;
  /** Average tile size in kilobytes (1 GB = 1,000,000 KB). */
  tileKb: number;
  /** Share of tile requests answered by the CDN cache, 0–100. */
  cacheHitPct: number;
  storageGb: number;
  includeFree: boolean;
  /** Hetzner servers you run. */
  servers: number;
}

/** The tile workload fields a user can edit. */
export type TileWorkloadKey =
  | 'sessions'
  | 'tilesPerSession'
  | 'tileKb'
  | 'cacheHitPct'
  | 'storageGb';

/** A numeric input on the calculator form. */
export interface CostField {
  key: TileWorkloadKey;
  label: string;
  step: number;
  max?: number;
}

/** A group heading in the calculator table. */
export interface CostGroup {
  id: CostRowGroup;
  label: string;
}

/**
 * Network round trip from a user's city to a Hetzner location, in ms.
 * `sources` holds the Frankfurt ping, then the Helsinki ping.
 */
export interface CityRtt {
  city: string;
  germanyMs: number;
  finlandMs: number;
  sources: readonly [CostSource, CostSource];
}
