<script setup lang="ts">
  import {
    Database,
    ExternalLink,
    FileJson,
    Globe,
    Layers,
    Map,
    Moon,
    Palette,
    Sun,
  } from 'lucide-vue-next';

  const { isDark, toggle: toggleColorMode } = useThemeToggle();
  const {
    dataSources,
    styles,
    isLoadingData,
    isLoadingStyles,
    hasStyles,
    hasData,
  } = useTileserverData();

  const apiEndpoints = [
    { method: 'GET', path: '/data.json', description: 'List all data sources' },
    {
      method: 'GET',
      path: '/data/{source}.json',
      description: 'TileJSON for a source',
    },
    {
      method: 'GET',
      path: '/data/{source}/{z}/{x}/{y}.pbf',
      description: 'Vector tiles',
    },
    { method: 'GET', path: '/styles.json', description: 'List all styles' },
    {
      method: 'GET',
      path: '/styles/{style}/style.json',
      description: 'MapLibre style spec',
    },
    {
      method: 'GET',
      path: '/styles/{style}/{z}/{x}/{y}[@{scale}x].{format}',
      description: 'Raster tiles (PNG/JPEG/WebP)',
    },
    {
      method: 'GET',
      path: '/styles/{style}/static/{type}/{size}[@{scale}x].{format}',
      description: 'Static map images',
    },
    { method: 'GET', path: '/health', description: 'Health check' },
  ];
</script>

<template>
  <div class="min-h-screen bg-background">
    <!-- Header -->
    <header
      class="
        sticky top-0 z-50 border-b border-border/40 bg-background/95
        backdrop-blur-sm
        supports-backdrop-filter:bg-background/60
      "
    >
      <div class="mx-auto flex h-16 max-w-5xl items-center justify-between px-6">
        <div class="flex items-center gap-3">
          <div
            class="
              flex size-10 items-center justify-center rounded-xl bg-primary
              shadow-lg shadow-primary/25
            "
          >
            <Globe class="size-5 text-primary-foreground" />
          </div>
          <div>
            <h1 class="text-xl font-semibold tracking-tight">
              Tileserver RS
            </h1>
            <p class="text-xs text-muted-foreground">
              High-performance vector tile server
            </p>
          </div>
        </div>

        <Button variant="ghost" size="icon" @click="toggleColorMode">
          <Sun v-if="isDark" class="size-5" />
          <Moon v-else class="size-5" />
          <span class="sr-only">Toggle theme</span>
        </Button>
      </div>
    </header>

    <main class="mx-auto max-w-5xl space-y-8 px-6 py-8">
      <!-- Styles Section -->
      <section class="space-y-4">
        <div class="flex items-center gap-2">
          <Palette class="size-5 text-primary" />
          <h2 class="text-lg font-semibold">
            Map Styles
          </h2>
        </div>

        <div
          v-if="isLoadingStyles"
          class="flex items-center justify-center py-8"
        >
          <div
            class="
              size-6 animate-spin rounded-full border-2 border-muted
              border-t-primary
            "
          ></div>
        </div>

        <Card v-else-if="!hasStyles" class="border-dashed">
          <CardContent
            class="flex flex-col items-center justify-center py-8 text-center"
          >
            <Palette class="mb-3 size-10 text-muted-foreground/50" />
            <CardTitle class="text-base">
              No styles configured
            </CardTitle>
            <CardDescription class="mt-1">
              Add styles to your config.toml to enable styled map views
            </CardDescription>
          </CardContent>
        </Card>

        <div v-else class="grid gap-4">
          <Card
            v-for="style in styles"
            :key="style.id"
            class="
              group overflow-hidden transition-all
              hover:border-primary/50 hover:shadow-lg hover:shadow-primary/5
            "
          >
            <CardContent class="flex items-center justify-between p-5">
              <div class="flex items-center gap-4">
                <div
                  class="relative size-14 overflow-hidden rounded-lg bg-muted"
                >
                  <img
                    :src="`/styles/${style.id}/static/0,0,1/112x112.png`"
                    :alt="`${style.name} preview`"
                    class="size-full object-cover"
                    loading="lazy"
                  />
                </div>
                <div class="space-y-1">
                  <CardTitle class="text-base">
                    {{ style.name }}
                  </CardTitle>
                  <div class="flex items-center gap-2">
                    <Badge variant="secondary" class="font-mono text-xs">
                      {{ style.id }}
                    </Badge>
                    <NuxtLink
                      :to="`/styles/${style.id}/style.json`"
                      class="
                        flex items-center gap-1 text-xs text-muted-foreground
                        transition-colors
                        hover:text-primary
                      "
                    >
                      <FileJson class="size-3" />
                      style.json
                    </NuxtLink>
                  </div>
                </div>
              </div>
              <Button as-child>
                <NuxtLink :to="`/styles/${style.id}/#2/0/0`">
                  <Map class="size-4" />
                  View Map
                </NuxtLink>
              </Button>
            </CardContent>
          </Card>
        </div>
      </section>

      <Separator />

      <!-- Data Sources Section -->
      <section class="space-y-4">
        <div class="flex items-center gap-2">
          <Database class="size-5 text-primary" />
          <h2 class="text-lg font-semibold">
            Data Sources
          </h2>
        </div>

        <div v-if="isLoadingData" class="flex items-center justify-center py-8">
          <div
            class="
              size-6 animate-spin rounded-full border-2 border-muted
              border-t-primary
            "
          ></div>
        </div>

        <Card v-else-if="!hasData" class="border-dashed">
          <CardContent
            class="flex flex-col items-center justify-center py-8 text-center"
          >
            <Database class="mb-3 size-10 text-muted-foreground/50" />
            <CardTitle class="text-base">
              No data sources available
            </CardTitle>
            <CardDescription class="mt-1">
              Configure PMTiles or MBTiles sources in your config.toml
            </CardDescription>
          </CardContent>
        </Card>

        <div v-else class="grid gap-4">
          <Card
            v-for="source in dataSources"
            :key="source.id"
            class="
              group overflow-hidden transition-all
              hover:border-primary/50 hover:shadow-lg hover:shadow-primary/5
            "
          >
            <CardContent class="flex items-center justify-between p-5">
              <div class="flex items-center gap-4">
                <div
                  class="
                    flex size-14 items-center justify-center rounded-lg bg-muted
                  "
                >
                  <Layers class="size-6 text-muted-foreground" />
                </div>
                <div class="space-y-1">
                  <CardTitle class="text-base">
                    {{ source.name || source.id }}
                  </CardTitle>
                  <div class="flex flex-wrap items-center gap-2">
                    <Badge variant="secondary" class="font-mono text-xs">
                      {{ source.id }}
                    </Badge>
                    <Badge variant="outline" class="text-xs">
                      z{{ source.minzoom }}-{{ source.maxzoom }}
                    </Badge>
                    <NuxtLink
                      :to="`/data/${source.id}.json`"
                      class="
                        flex items-center gap-1 text-xs text-muted-foreground
                        transition-colors
                        hover:text-primary
                      "
                    >
                      <FileJson class="size-3" />
                      tilejson
                    </NuxtLink>
                  </div>
                </div>
              </div>
              <Button as-child variant="secondary">
                <NuxtLink :to="`/data/${source.id}/#2/0/0`">
                  <Layers class="size-4" />
                  Inspect
                </NuxtLink>
              </Button>
            </CardContent>
          </Card>
        </div>
      </section>

      <Separator />

      <!-- API Endpoints Section -->
      <section class="space-y-4">
        <div class="flex items-center gap-2">
          <ExternalLink class="size-5 text-muted-foreground" />
          <h2 class="text-lg font-semibold">
            API Endpoints
          </h2>
        </div>

        <Card>
          <CardContent class="divide-y divide-border p-0">
            <div
              v-for="(endpoint, index) in apiEndpoints"
              :key="index"
              class="
                flex items-center justify-between px-5 py-3 transition-colors
                hover:bg-muted/50
              "
            >
              <div class="flex items-center gap-3">
                <Badge
                  variant="outline"
                  class="
                    font-mono text-xs text-emerald-600
                    dark:text-emerald-400
                  "
                >
                  {{ endpoint.method }}
                </Badge>
                <code
                  class="text-sm text-foreground"
                  v-text="endpoint.path"
                ></code>
              </div>
              <span class="text-xs text-muted-foreground">
                {{ endpoint.description }}
              </span>
            </div>
          </CardContent>
        </Card>
      </section>
    </main>

    <!-- Footer -->
    <footer class="border-t border-border/40">
      <div class="mx-auto max-w-5xl p-6">
        <p class="text-center text-sm text-muted-foreground">
          Tileserver RS — Built with Rust + Axum + MapLibre GL JS
        </p>
      </div>
    </footer>
  </div>
</template>
