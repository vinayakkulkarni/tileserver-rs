import { describe, expect, it } from 'vitest';
import type { TieredPrice } from '~/types/cost';
import { tieredCost } from './build';

const source = { label: 'test', url: 'https://example.com' };
// 0–1,000 at $2 per 1,000, 1,000–3,000 at $1, then contact sales; 500 free.
const price: TieredPrice = {
  free: 500,
  tiers: [
    { upTo: 1_000, per1k: 2 },
    { upTo: 3_000, per1k: 1 },
    { upTo: null, per1k: null },
  ],
  source,
};

describe('tieredCost', () => {
  it('bills the free units at the first band when free tiers are off', () => {
    // 1,000 × $2/1k + 1,000 × $1/1k = $2 + $1
    expect(tieredCost(2_000, price, false)).toBeCloseTo(3, 10);
  });

  it('removes the free units from the bottom band when free tiers are on', () => {
    // 500 × $2/1k + 1,000 × $1/1k = $1 + $1
    expect(tieredCost(2_000, price, true)).toBeCloseTo(2, 10);
  });

  it('costs nothing below the free allowance', () => {
    expect(tieredCost(400, price, true)).toBe(0);
  });

  it('includes the last unit of a band in that band', () => {
    // 1,000 × $2/1k + 2,000 × $1/1k
    expect(tieredCost(3_000, price, false)).toBeCloseTo(4, 10);
  });

  it('answers contact sales past the last priced band', () => {
    expect(tieredCost(3_001, price, false)).toBeNull();
  });
});
