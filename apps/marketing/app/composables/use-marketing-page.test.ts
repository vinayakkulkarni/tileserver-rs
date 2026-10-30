import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it, vi } from 'vitest';

vi.mock('./use-analytics', () => ({
  useAnalytics: () => ({ track: vi.fn() }),
}));
vi.stubGlobal('useClipboard', () => ({ copied: false, copy: vi.fn() }));

const { useMarketingPage } = await import('./use-marketing-page');

/**
 * Column counts of every `grid-cols-N` class in a component, read from the
 * template so the check follows the real layout.
 */
function gridColumns(component: string): number[] {
  const path = fileURLToPath(
    new URL(`../components/marketing/${component}`, import.meta.url),
  );
  const source = readFileSync(path, 'utf8');
  return [...source.matchAll(/(?:^|\s)(?:[a-z]+:)?grid-cols-(\d+)\b/g)].map(
    (match) => Number(match[1]),
  );
}

describe('marketing grids have no empty cells', () => {
  const page = useMarketingPage();

  it('fills every row of each feature category', () => {
    const columns = gridColumns('FeaturesSection.vue');
    expect(columns.length).toBeGreaterThan(0);
    for (const group of page.featuresByCategory) {
      for (const cols of columns) {
        expect(
          group.features.length % cols,
          `${group.category}: ${group.features.length} cards in ${cols} columns`,
        ).toBe(0);
      }
    }
  });

  it('fills every row of the performance stats', () => {
    const columns = gridColumns('PerformanceSection.vue');
    expect(columns.length).toBeGreaterThan(0);
    for (const cols of columns) {
      expect(
        page.performanceStats.length % cols,
        `${page.performanceStats.length} stats in ${cols} columns`,
      ).toBe(0);
    }
  });

  it('fills every row of the AI benefits', () => {
    for (const cols of gridColumns('AiSection.vue')) {
      if (cols === 2) continue; // the chat-demo layout, not the benefit grid
      expect(page.aiBenefits.length % cols).toBe(0);
    }
  });

  it('fills every row of the API endpoint groups', () => {
    for (const cols of gridColumns('ApiSection.vue')) {
      expect(page.apiEndpoints.length % cols).toBe(0);
    }
  });
});
