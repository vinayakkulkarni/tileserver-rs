import type { CityRtt, CostSource } from '~/types/cost';

function pings(from: string, to: string): CostSource {
  return {
    label: `WonderNetwork pings ${from} → ${to}`,
    url: `https://wondernetwork.com/pings/${from}/${encodeURIComponent(to)}`,
  };
}

/** Both pings for one destination: from Frankfurt, then from Helsinki. */
function both(to: string): readonly [CostSource, CostSource] {
  return [pings('Frankfurt', to), pings('Helsinki', to)];
}

export const cityRtts: readonly CityRtt[] = [
  { city: 'London', germanyMs: 18, finlandMs: 33, sources: both('London') },
  {
    city: 'New York',
    germanyMs: 83,
    finlandMs: 112,
    sources: both('New York'),
  },
  {
    city: 'San Francisco',
    germanyMs: 158,
    finlandMs: 159,
    sources: both('San Francisco'),
  },
  {
    city: 'São Paulo',
    germanyMs: 198,
    finlandMs: 200,
    sources: both('Sao Paulo'),
  },
  {
    city: 'Johannesburg',
    germanyMs: 177,
    finlandMs: 189,
    sources: both('Johannesburg'),
  },
  { city: 'Mumbai', germanyMs: 136, finlandMs: 164, sources: both('Mumbai') },
  {
    city: 'Singapore',
    germanyMs: 157,
    finlandMs: 175,
    sources: both('Singapore'),
  },
  { city: 'Tokyo', germanyMs: 225, finlandMs: 260, sources: both('Tokyo') },
  { city: 'Sydney', germanyMs: 301, finlandMs: 306, sources: both('Sydney') },
];

/** Cloudflare's published distance from users to its network. */
export const cloudflareReach = {
  ms: 50,
  text: '95% of the world’s Internet-connected population is within 50 ms of a Cloudflare data center.',
  source: {
    label: 'Cloudflare network',
    url: 'https://www.cloudflare.com/network/',
  },
} as const;

/** Find a city's round trips; an unknown name has none. */
export function rttFor(city: string): CityRtt | undefined {
  return cityRtts.find((row) => row.city === city);
}
