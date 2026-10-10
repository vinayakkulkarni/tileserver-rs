import type { CostEstimate, CostLine, CostSource, PricePlan, TieredPrice } from '~/types/cost';
import {
  geoapifyFreePerDay,
  geoapifyPlans,
  hetznerAx102,
  sources,
} from './prices';

/** Geoapify quotas are per day; a month counts as 30 days. */
const days = 30;

/** A line priced on a tiered list, or `null` for "contact sales". */
export function tieredLine(
  label: string,
  units: number,
  unit: string,
  price: TieredPrice,
  includeFree: boolean,
): CostLine | null {
  const cost = tieredCost(units, price, includeFree);
  return cost === null
    ? null
    : { label, units, unit, cost, source: price.source };
}

/** A line priced at a flat rate per unit. */
export function flatLine(
  label: string,
  units: number,
  unit: string,
  perUnit: number,
  source: CostSource,
): CostLine {
  return { label, units, unit, cost: units * perUnit, source };
}

/** Sum the lines into an estimate; any `null` line makes it "contact sales". */
export function total(
  lines: readonly (CostLine | null)[],
  contact: CostSource,
  setup?: number,
): CostEstimate {
  const priced: CostLine[] = [];
  for (const line of lines) {
    if (line === null) {
      return {
        kind: 'contact',
        reason: 'Usage passes the last published price band.',
        source: contact,
      };
    }
    priced.push(line);
  }
  const monthly = priced.reduce((sum, line) => sum + line.cost, 0);
  return setup === undefined
    ? { kind: 'usd', monthly, lines: priced }
    : { kind: 'usd', monthly, lines: priced, setup };
}

/** Cost of `units` on one plan: the plan fee plus extra units. */
export function planCost(plan: PricePlan, units: number): number | null {
  if (units <= plan.included) {
    return plan.monthly;
  }
  return plan.overPer1k === null
    ? null
    : plan.monthly + ((units - plan.included) / 1_000) * plan.overPer1k;
}

/** The cheapest plan for `units`, or `null` when no plan covers them. */
export function cheapestPlan(
  plans: readonly PricePlan[],
  units: number,
): { plan: PricePlan; cost: number } | null {
  let best: { plan: PricePlan; cost: number } | null = null;
  for (const plan of plans) {
    const cost = planCost(plan, units);
    if (cost !== null && (best === null || cost < best.cost)) {
      best = { plan, cost };
    }
  }
  return best;
}

/** `servers` Hetzner AX102 machines, each with one IPv4 address. */
export function hetznerEstimate(servers: number): CostEstimate {
  const { source } = hetznerAx102;
  return total(
    [
      flatLine(
        'AX102-1 server',
        servers,
        'servers',
        hetznerAx102.monthly,
        source,
      ),
      flatLine(
        'Primary IPv4',
        servers,
        'addresses',
        hetznerAx102.ipv4Monthly,
        source,
      ),
    ],
    source,
    servers * hetznerAx102.setup,
  );
}

/** Units above a free allowance, when free tiers count. */
export function aboveFree(
  units: number,
  free: number,
  includeFree: boolean,
): number {
  return includeFree ? Math.max(0, units - free) : units;
}

/** Geoapify plans are sized by credits per day. */
export function geoapifyEstimate(
  creditsPerMonth: number,
  includeFree: boolean,
): CostEstimate {
  const perDay = creditsPerMonth / days;
  if (includeFree && perDay <= geoapifyFreePerDay) {
    return total(
      [
        {
          label: 'Free plan',
          units: creditsPerMonth,
          unit: 'credits',
          cost: 0,
          source: sources.geoapify,
        },
      ],
      sources.geoapify,
    );
  }
  const plan = geoapifyPlans.find((p) => perDay <= p.included);
  return plan === undefined
    ? {
        kind: 'contact',
        reason: 'Usage passes the largest published plan.',
        source: sources.geoapify,
      }
    : total(
        [
          {
            label: `${plan.name} plan`,
            units: creditsPerMonth,
            unit: 'credits',
            cost: plan.monthly,
            source: sources.geoapify,
          },
        ],
        sources.geoapify,
      );
}

/**
 * Cost of `units` on a tiered price list. With `includeFree` the first
 * `price.free` units cost nothing; without it they cost the first band's
 * price. Returns `null` when units reach a "contact sales" band.
 */
export function tieredCost(
  units: number,
  price: TieredPrice,
  includeFree: boolean,
): number | null {
  const start = includeFree ? Math.min(price.free, units) : 0;
  let cost = 0;
  let lower = 0;
  for (const tier of price.tiers) {
    const upper = tier.upTo ?? Number.POSITIVE_INFINITY;
    const inBand = Math.min(units, upper) - Math.max(start, lower);
    if (inBand > 0) {
      if (tier.per1k === null) {
        return null;
      }
      cost += (inBand / 1_000) * tier.per1k;
    }
    if (units <= upper) {
      break;
    }
    lower = upper;
  }
  return cost;
}
