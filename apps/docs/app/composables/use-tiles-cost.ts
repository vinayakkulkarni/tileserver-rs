import { cityRtts, cloudflareReach, rttFor } from '~/lib/cost/latency';
import { pricesReadOn } from '~/lib/cost/prices';
import { tileRows } from '~/lib/cost/tiles';
import type {
  CostEstimate,
  CostField,
  CostGroup,
  CostRow,
  CostRowGroup,
  TileWorkloadKey,
} from '~/types';

/** Tile workload inputs; defaults are the Protomaps calculator defaults. */
const fields: readonly CostField[] = [
  { key: 'sessions', label: 'Map sessions / month', step: 1_000 },
  { key: 'tilesPerSession', label: 'Tiles per session', step: 1 },
  { key: 'tileKb', label: 'Average tile size (KB)', step: 1 },
  { key: 'cacheHitPct', label: 'CDN cache hit rate (%)', step: 1, max: 100 },
  { key: 'storageGb', label: 'Storage (GB)', step: 1 },
];

const groups: readonly CostGroup[] = [
  { id: 'edge', label: 'Edge' },
  { id: 'self-host', label: 'Self-host' },
  { id: 'aws', label: 'AWS edge' },
  { id: 'vendor', label: 'Hosted vendors' },
];

const money = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 2,
});
const count = new Intl.NumberFormat('en-US', { maximumFractionDigits: 2 });

/** A finite number in `[min, max]`; empty or bad input counts as `min`. */
function clean(value: number, min = 0, max = Number.POSITIVE_INFINITY): number {
  return Number.isFinite(value) ? Math.min(Math.max(value, min), max) : min;
}

/** Monthly price of an estimate, or "Contact sales". */
function formatCost(estimate: CostEstimate): string {
  return estimate.kind === 'usd' ? money.format(estimate.monthly) : 'Contact sales';
}

/** Dollar amount for one breakdown line or a setup fee. */
function formatMoney(value: number): string {
  return money.format(value);
}

/** Unit count for one breakdown line. */
function formatCount(value: number): string {
  return count.format(value);
}

/** State and derived rows for the map tiles cost calculator. */
export function useTilesCost() {
  const workload = ref<Record<TileWorkloadKey, number>>({
    sessions: 625_000,
    tilesPerSession: 16,
    tileKb: 100,
    cacheHitPct: 50,
    storageGb: 110,
  });
  const includeFree = ref(false);
  const city = ref('Singapore');
  const servers = ref(1);

  const rows = computed<CostRow[]>(() =>
    tileRows({
      sessions: clean(workload.value.sessions),
      tilesPerSession: clean(workload.value.tilesPerSession),
      tileKb: clean(workload.value.tileKb),
      cacheHitPct: clean(workload.value.cacheHitPct, 0, 100),
      storageGb: clean(workload.value.storageGb),
      includeFree: includeFree.value,
      servers: clean(servers.value, 1),
    }),
  );

  const rtt = computed(() => rttFor(city.value));

  /** Rows that belong to one group, in table order. */
  function rowsIn(group: CostRowGroup): CostRow[] {
    return rows.value.filter((row) => row.group === group);
  }

  return {
    workload,
    includeFree,
    city,
    servers,
    rtt,
    rowsIn,
    fields,
    groups,
    cities: cityRtts,
    cloudflareReach,
    pricesReadOn,
    formatCost,
    formatMoney,
    formatCount,
  };
}
