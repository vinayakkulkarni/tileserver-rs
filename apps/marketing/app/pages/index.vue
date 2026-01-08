<script setup lang="ts">
import {
  Github,
  Zap,
  Globe,
  Layers,
  Image,
  Server,
  ArrowRight,
  Copy,
  Check,
  Terminal,
  Sparkles,
  Database,
  MapIcon,
  HardDrive,
} from 'lucide-vue-next';
import { Button } from '~/components/ui/button';
import { Badge } from '~/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '~/components/ui/card';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '~/components/ui/tooltip';

const copied = ref(false);
const installCommand = 'brew install vinayakkulkarni/tap/tileserver-rs';

async function copyToClipboard() {
  await navigator.clipboard.writeText(installCommand);
  copied.value = true;
  setTimeout(() => {
    copied.value = false;
  }, 2000);
}

const features = [
  {
    icon: Zap,
    title: 'Blazing Fast',
    description:
      'Built in Rust for maximum performance. Serve tiles with sub-millisecond latency.',
  },
  {
    icon: Globe,
    title: 'PMTiles & MBTiles',
    description:
      'Native support for modern PMTiles and classic MBTiles tile archives.',
  },
  {
    icon: Database,
    title: 'PostgreSQL / PostGIS',
    description:
      'Serve vector tiles directly from PostGIS tables with optimized spatial queries.',
  },
  {
    icon: MapIcon,
    title: 'Cloud Optimized GeoTIFF',
    description:
      'Serve raster tiles from COG files with on-the-fly reprojection and colormap support.',
  },
  {
    icon: HardDrive,
    title: 'PostgreSQL Out-DB Rasters',
    description:
      'Dynamic VRT/COG tile serving via PostGIS functions with query-based filtering.',
  },
  {
    icon: Layers,
    title: 'Vector & Raster',
    description:
      'Serve vector tiles directly or render them to raster on-the-fly.',
  },
  {
    icon: Image,
    title: 'Static Images',
    description:
      'Generate static map images like Mapbox Static API with native MapLibre rendering.',
  },
  {
    icon: Server,
    title: 'Self-Hosted',
    description:
      'Run on your own infrastructure. No vendor lock-in, no API keys required.',
  },
];

const apiEndpoints = [
  {
    title: 'Vector Tiles',
    endpoints: [
      { method: 'GET', path: '/data/{source}/{z}/{x}/{y}.pbf' },
      { method: 'GET', path: '/data/{source}.json' },
    ],
  },
  {
    title: 'COG/Raster Tiles',
    endpoints: [
      { method: 'GET', path: '/data/{cog}/{z}/{x}/{y}.png' },
      { method: 'GET', path: '/data/{cog}/{z}/{x}/{y}.webp' },
    ],
  },
  {
    title: 'PostgreSQL Out-DB Rasters',
    endpoints: [
      { method: 'GET', path: '/data/{source}/{z}/{x}/{y}.png?satellite=...' },
    ],
  },
  {
    title: 'Style Rendering',
    endpoints: [
      { method: 'GET', path: '/styles/{style}/{z}/{x}/{y}.png' },
      { method: 'GET', path: '/styles/{style}/{z}/{x}/{y}@2x.png' },
    ],
  },
  {
    title: 'Static Images',
    endpoints: [
      {
        method: 'GET',
        path: '/styles/{style}/static/{lon},{lat},{zoom}/{w}x{h}.png',
      },
    ],
  },
];
</script>

<template>
  <TooltipProvider>
    <div class="relative min-h-screen overflow-hidden bg-background">
      <!-- Animated background orbs -->
      <div class="pointer-events-none fixed inset-0 overflow-hidden">
        <div
          class="orb orb-primary absolute top-1/4 -left-32 size-[500px]"
        ></div>
        <div
          class="orb orb-secondary absolute top-1/2 -right-32 size-[600px]"
        ></div>
        <div
          class="orb orb-accent absolute bottom-0 left-1/3 size-[400px]"
        ></div>
      </div>

      <!-- Grid overlay -->
      <div class="bg-grid pointer-events-none fixed inset-0"></div>

      <!-- Content -->
      <div class="relative z-10">
        <!-- Navigation -->
        <nav
          class="
            fixed top-0 z-50 w-full border-b border-primary/10 bg-background/60
            backdrop-blur-xl
          "
        >
          <div
            class="
              mx-auto flex h-16 max-w-6xl items-center justify-between px-4
            "
          >
            <NuxtLink to="/" class="group flex items-center gap-3">
              <div
                class="
                  glow-primary-sm flex size-9 items-center justify-center
                  rounded-lg bg-primary/10 ring-1 ring-primary/30 transition-all
                  group-hover:ring-primary/60
                "
              >
                <Globe class="size-5 text-primary" />
              </div>
              <span class="text-lg font-semibold tracking-tight">
                <span class="text-foreground">Tileserver</span>
                <span class="text-primary"> RS</span>
              </span>
            </NuxtLink>
            <div class="flex items-center gap-3">
              <Button
                variant="ghost"
                size="sm"
                as="a"
                href="https://docs.tileserver.app"
                class="
                  text-muted-foreground
                  hover:text-primary
                "
              >
                Docs
              </Button>
              <Button
                variant="outline"
                size="sm"
                as="a"
                href="https://github.com/vinayakkulkarni/tileserver-rs"
                class="
                  border-primary/30 bg-primary/5
                  hover:border-primary/60 hover:bg-primary/10
                "
              >
                <Github class="size-4" />
                GitHub
              </Button>
            </div>
          </div>
        </nav>

        <!-- Hero Section -->
        <section class="relative pt-32 pb-20">
          <div class="relative mx-auto max-w-6xl px-4">
            <div class="mx-auto max-w-4xl text-center">
              <!-- Badge -->
              <Badge
                variant="outline"
                class="
                  mb-8 gap-2 border-primary/30 bg-primary/5 px-4 py-2 text-sm
                  backdrop-blur-sm
                "
              >
                <span class="status-online"></span>
                <span class="text-muted-foreground">Now available on</span>
                <a
                  href="https://github.com/vinayakkulkarni/tileserver-rs/releases"
                  class="
                    font-medium text-primary
                    hover:underline
                  "
                >
                  GitHub Releases
                </a>
              </Badge>

              <!-- Main heading - fixed text cutoff with pb-2 and proper line-height -->
              <h1
                class="
                  mb-8 text-5xl font-bold tracking-tight
                  sm:text-6xl
                  lg:text-7xl
                "
              >
                <span class="text-foreground">Serve Vector Tiles</span>
                <span
                  class="text-gradient-animated mt-2 block pb-2 leading-tight"
                >At Lightning Speed</span>
              </h1>

              <!-- Description -->
              <p
                class="
                  mx-auto mb-10 max-w-2xl text-lg text-muted-foreground
                  sm:text-xl
                "
              >
                High-performance tile server built in
                <span class="font-semibold text-primary">Rust</span>. Serve
                PMTiles, MBTiles, PostGIS, and Cloud Optimized GeoTIFF with native MapLibre rendering for
                static images.
              </p>

              <!-- CTA buttons -->
              <div
                class="
                  flex flex-col items-center justify-center gap-4
                  sm:flex-row
                "
              >
                <Button
                  size="lg"
                  as="a"
                  href="https://docs.tileserver.app/getting-started/quickstart"
                  class="
                    btn-glow group relative gap-2 bg-primary px-8
                    text-primary-foreground
                    hover:bg-primary/90
                  "
                >
                  <Sparkles class="size-4" />
                  Get Started
                  <ArrowRight
                    class="
                      size-4 transition-transform
                      group-hover:translate-x-1
                    "
                  />
                </Button>
                <Button
                  variant="outline"
                  size="lg"
                  as="a"
                  href="https://github.com/vinayakkulkarni/tileserver-rs"
                  class="
                    border-border/50 bg-card/50 backdrop-blur-sm
                    hover:border-primary/30 hover:bg-card
                  "
                >
                  <Github class="size-4" />
                  View on GitHub
                </Button>
              </div>

              <!-- Install command -->
              <div class="mx-auto mt-14 max-w-xl">
                <Card class="card-glow overflow-hidden">
                  <CardContent class="flex items-center gap-4 p-4">
                    <div
                      class="
                        flex size-8 items-center justify-center rounded-md
                        bg-primary/10
                      "
                    >
                      <Terminal class="size-4 text-primary" />
                    </div>
                    <code
                      class="flex-1 text-left font-mono text-sm text-foreground"
                    >{{ installCommand }}</code>
                    <Tooltip>
                      <TooltipTrigger as-child>
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          class="
                            text-muted-foreground
                            hover:text-primary
                          "
                          @click="copyToClipboard"
                        >
                          <Check
                            v-if="copied"
                            class="size-4 text-emerald-400"
                          />
                          <Copy v-else class="size-4" />
                        </Button>
                      </TooltipTrigger>
                      <TooltipContent class="border-primary/20 bg-card">
                        <p>{{ copied ? "Copied!" : "Copy to clipboard" }}</p>
                      </TooltipContent>
                    </Tooltip>
                  </CardContent>
                </Card>
              </div>
            </div>
          </div>
        </section>

        <!-- Features Section -->
        <section class="relative py-24">
          <div class="mx-auto max-w-6xl px-4">
            <div class="mb-16 text-center">
              <p class="hud-label mb-4">
                Features
              </p>
              <h2
                class="
                  mb-4 text-3xl font-bold
                  sm:text-4xl
                "
              >
                Everything You <span class="text-gradient">Need</span>
              </h2>
              <p class="mx-auto max-w-xl text-lg text-muted-foreground">
                A complete solution for serving vector tiles in production.
              </p>
            </div>

            <div
              class="
                grid gap-6
                sm:grid-cols-2
                lg:grid-cols-3
              "
            >
              <Card
                v-for="feature in features"
                :key="feature.title"
                class="
                  card-glow group relative overflow-hidden transition-all
                  duration-300
                "
              >
                <CardHeader>
                  <div
                    class="
                      mb-3 inline-flex size-12 items-center justify-center
                      rounded-xl bg-primary/10 text-primary transition-all
                      duration-300
                      group-hover:bg-primary/20
                    "
                  >
                    <component :is="feature.icon" class="size-6" />
                  </div>
                  <CardTitle class="text-lg">
                    {{ feature.title }}
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <p class="text-muted-foreground">
                    {{ feature.description }}
                  </p>
                </CardContent>
              </Card>
            </div>
          </div>
        </section>

        <!-- Divider -->
        <div class="divider-glow mx-auto max-w-4xl"></div>

        <!-- Code Example Section -->
        <section class="relative py-24">
          <div class="mx-auto max-w-6xl px-4">
            <div
              class="
                grid items-center gap-12
                lg:grid-cols-2
              "
            >
              <div>
                <p class="hud-label mb-4">
                  Configuration
                </p>
                <h2
                  class="
                    mb-4 text-3xl font-bold
                    sm:text-4xl
                  "
                >
                  Simple <span class="text-gradient">Setup</span>
                </h2>
                <p class="mb-8 text-lg text-muted-foreground">
                  Get started with a simple TOML configuration file. Define your
                  tile sources, styles, and server settings in one place.
                </p>
                <ul class="space-y-4">
                  <li class="flex items-center gap-3">
                    <div
                      class="
                        flex size-6 items-center justify-center rounded-full
                        bg-primary/20
                      "
                    >
                      <div class="size-1.5 rounded-full bg-primary"></div>
                    </div>
                    <span class="text-foreground">Multiple tile sources (PMTiles, MBTiles, PostGIS, COG)</span>
                  </li>
                  <li class="flex items-center gap-3">
                    <div
                      class="
                        flex size-6 items-center justify-center rounded-full
                        bg-primary/20
                      "
                    >
                      <div class="size-1.5 rounded-full bg-primary"></div>
                    </div>
                    <span class="text-foreground">Custom MapLibre GL styles</span>
                  </li>
                  <li class="flex items-center gap-3">
                    <div
                      class="
                        flex size-6 items-center justify-center rounded-full
                        bg-primary/20
                      "
                    >
                      <div class="size-1.5 rounded-full bg-primary"></div>
                    </div>
                    <span class="text-foreground">In-memory tile caching with TTL</span>
                  </li>
                  <li class="flex items-center gap-3">
                    <div
                      class="
                        flex size-6 items-center justify-center rounded-full
                        bg-primary/20
                      "
                    >
                      <div class="size-1.5 rounded-full bg-primary"></div>
                    </div>
                    <span class="text-foreground">Configurable CORS and connection pooling</span>
                  </li>
                </ul>
              </div>
              <Card class="card-glow overflow-hidden">
                <CardHeader
                  class="
                    flex-row items-center gap-2 border-b border-border/50 px-4
                    py-3
                  "
                >
                  <div class="size-3 rounded-full bg-red-500/80"></div>
                  <div class="size-3 rounded-full bg-yellow-500/80"></div>
                  <div class="size-3 rounded-full bg-green-500/80"></div>
                  <span class="ml-2 font-mono text-xs text-muted-foreground">config.toml</span>
                </CardHeader>
                <CardContent class="p-0">
                  <pre
                    class="overflow-x-auto p-5 font-mono text-sm/relaxed"
                  ><code><span
                           class="token-comment"
                         ># Tile sources</span>
<span class="token-keyword">[[sources]]</span>
id = <span class="token-string">"openmaptiles"</span>
type = <span class="token-string">"pmtiles"</span>
path = <span class="token-string">"/data/tiles.pmtiles"</span>

<span class="token-comment"># PostgreSQL / PostGIS</span>
<span class="token-keyword">[postgres]</span>
connection_string = <span class="token-string">"postgresql://user:pass@localhost/db"</span>

<span class="token-keyword">[[postgres.tables]]</span>
id = <span class="token-string">"buildings"</span>
table = <span class="token-string">"buildings"</span>
geometry_column = <span class="token-string">"geom"</span></code></pre>
                </CardContent>
              </Card>
            </div>
          </div>
        </section>

        <!-- Divider -->
        <div class="divider-glow mx-auto max-w-4xl"></div>

        <!-- API Endpoints Section -->
        <section class="relative py-24">
          <div class="mx-auto max-w-6xl px-4">
            <div class="mb-16 text-center">
              <p class="hud-label mb-4">
                API Reference
              </p>
              <h2
                class="
                  mb-4 text-3xl font-bold
                  sm:text-4xl
                "
              >
                RESTful <span class="text-gradient">API</span>
              </h2>
              <p class="mx-auto max-w-xl text-lg text-muted-foreground">
                Simple, standards-compliant API for all your tile serving needs.
              </p>
            </div>

            <div
              class="
                grid gap-6
                sm:grid-cols-2
              "
            >
              <Card
                v-for="group in apiEndpoints"
                :key="group.title"
                class="card-glow"
              >
                <CardHeader class="pb-3">
                  <CardTitle class="text-base font-semibold">
                    {{ group.title }}
                  </CardTitle>
                </CardHeader>
                <CardContent class="space-y-2">
                  <div
                    v-for="endpoint in group.endpoints"
                    :key="endpoint.path"
                    class="
                      flex items-center gap-3 rounded-lg bg-background/50 px-3
                      py-2 font-mono text-sm
                    "
                  >
                    <Badge
                      class="
                        bg-emerald-500/20 text-xs text-emerald-400
                        hover:bg-emerald-500/20
                      "
                    >
                      {{ endpoint.method }}
                    </Badge>
                    <span class="text-muted-foreground">{{
                      endpoint.path
                    }}</span>
                  </div>
                </CardContent>
              </Card>
            </div>
          </div>
        </section>

        <!-- Divider -->
        <div class="divider-glow mx-auto max-w-4xl"></div>

        <!-- CTA Section -->
        <section class="relative py-24">
          <div class="mx-auto max-w-6xl px-4 text-center">
            <div class="mx-auto max-w-2xl">
              <p class="hud-label mb-4">
                Get Started
              </p>
              <h2
                class="
                  mb-4 text-3xl font-bold
                  sm:text-4xl
                "
              >
                Ready to <span class="text-gradient">Deploy</span>?
              </h2>
              <p class="mb-10 text-lg text-muted-foreground">
                Deploy your own tile server in minutes with our comprehensive
                documentation.
              </p>
              <div
                class="
                  flex flex-col items-center justify-center gap-4
                  sm:flex-row
                "
              >
                <Button
                  size="lg"
                  as="a"
                  href="https://docs.tileserver.app/getting-started/quickstart"
                  class="
                    btn-glow group relative gap-2 bg-primary px-8
                    text-primary-foreground
                    hover:bg-primary/90
                  "
                >
                  Read the Docs
                  <ArrowRight
                    class="
                      size-4 transition-transform
                      group-hover:translate-x-1
                    "
                  />
                </Button>
                <Button
                  variant="outline"
                  size="lg"
                  as="a"
                  href="https://github.com/vinayakkulkarni/tileserver-rs"
                  class="
                    border-border/50 bg-card/50 backdrop-blur-sm
                    hover:border-primary/30 hover:bg-card
                  "
                >
                  <Github class="size-4" />
                  Star on GitHub
                </Button>
              </div>
            </div>
          </div>
        </section>

        <!-- Footer -->
        <footer class="relative border-t border-border/30 py-8">
          <div class="mx-auto max-w-6xl px-4">
            <div
              class="
                flex flex-col items-center justify-between gap-4
                sm:flex-row
              "
            >
              <div class="flex items-center gap-2">
                <div
                  class="
                    flex size-7 items-center justify-center rounded-lg
                    bg-primary/10 ring-1 ring-primary/30
                  "
                >
                  <Globe class="size-4 text-primary" />
                </div>
                <span class="font-medium">
                  <span class="text-foreground">Tileserver</span>
                  <span class="text-primary"> RS</span>
                </span>
              </div>
              <p class="text-sm text-muted-foreground">
                Built by
                <a
                  href="https://vinayakkulkarni.dev"
                  class="
                    text-primary
                    hover:underline
                  "
                >Vinayak Kulkarni</a>
                . Open source under MIT License.
              </p>
            </div>
          </div>
        </footer>
      </div>
    </div>
  </TooltipProvider>
</template>
