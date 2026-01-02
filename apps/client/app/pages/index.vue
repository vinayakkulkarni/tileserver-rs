<script setup lang="ts">
  import {
    Check,
    ChevronRight,
    Copy,
    Database,
    ExternalLink,
    Globe,
    Grid3x3,
    Image,
    Layers,
    Map,
    Moon,
    Palette,
    Search,
    Sun,
  } from 'lucide-vue-next';
  import { useClipboard } from '@vueuse/core';

  const { isDark, toggle: toggleColorMode } = useThemeToggle();
  const {
    dataSources,
    styles,
    isLoadingData,
    isLoadingStyles,
    hasStyles,
    hasData,
  } = useTileserverData();

  const { copy } = useClipboard();

  // Search filter
  const searchQuery = ref('');

  // Filtered lists
  const filteredStyles = computed(() => {
    if (!searchQuery.value) return styles.value;
    const query = searchQuery.value.toLowerCase();
    return styles.value.filter(
      (s) =>
        s.name.toLowerCase().includes(query) ||
        s.id.toLowerCase().includes(query),
    );
  });

  const filteredDataSources = computed(() => {
    if (!searchQuery.value) return dataSources.value;
    const query = searchQuery.value.toLowerCase();
    return dataSources.value.filter(
      (s) =>
        (s.name || '').toLowerCase().includes(query) ||
        s.id.toLowerCase().includes(query),
    );
  });

  // Track which XYZ URLs are expanded
  const expandedStyleXyz = ref<Set<string>>(new Set());
  const expandedDataXyz = ref<Set<string>>(new Set());

  function toggleStyleXyz(styleId: string) {
    if (expandedStyleXyz.value.has(styleId)) {
      expandedStyleXyz.value.delete(styleId);
    } else {
      expandedStyleXyz.value.add(styleId);
    }
    expandedStyleXyz.value = new Set(expandedStyleXyz.value);
  }

  function toggleDataXyz(dataId: string) {
    if (expandedDataXyz.value.has(dataId)) {
      expandedDataXyz.value.delete(dataId);
    } else {
      expandedDataXyz.value.add(dataId);
    }
    expandedDataXyz.value = new Set(expandedDataXyz.value);
  }

  // Copy with feedback
  const copiedUrl = ref<string | null>(null);
  function copyUrl(url: string) {
    copy(url);
    copiedUrl.value = url;
    setTimeout(() => {
      copiedUrl.value = null;
    }, 2000);
  }

  // Collapsible sections
  const stylesOpen = ref(true);
  const dataOpen = ref(true);

  // Get base URL for XYZ templates
  const baseUrl = computed(() => {
    if (import.meta.client) {
      return window.location.origin;
    }
    return '';
  });
</script>

<template>
  <div class="flex min-h-dvh flex-col bg-background">
    <!-- Header -->
    <header
      class="
        sticky top-0 z-50 border-b border-border/50 bg-background/80
        backdrop-blur-xl
      "
    >
      <div
        class="mx-auto flex h-14 max-w-5xl items-center justify-between px-4"
      >
        <div class="flex items-center gap-3">
          <div
            class="
              flex size-9 items-center justify-center rounded-xl bg-linear-to-br
              from-primary to-primary/80 shadow-lg shadow-primary/20
            "
          >
            <Globe class="size-5 text-primary-foreground" />
          </div>
          <div>
            <h1 class="text-lg font-semibold tracking-tight">
              Tileserver RS
            </h1>
            <p class="text-xs text-muted-foreground">
              High-performance vector tile server
            </p>
          </div>
        </div>
        <Button
          variant="ghost"
          size="icon"
          class="rounded-xl"
          @click="toggleColorMode"
        >
          <Sun v-if="isDark" class="size-5" />
          <Moon v-else class="size-5" />
        </Button>
      </div>
    </header>

    <!-- Main content -->
    <main class="mx-auto w-full max-w-5xl flex-1 space-y-4 p-4">
      <!-- Search -->
      <div class="relative">
        <Search
          class="
            absolute top-1/2 left-4 size-4 -translate-y-1/2
            text-muted-foreground
          "
        />
        <Input
          v-model="searchQuery"
          placeholder="Search styles and data sources..."
          class="
            h-11 rounded-xl border-border/50 bg-muted/30 pl-11 transition-all
            focus:bg-background
          "
        />
      </div>

      <!-- Styles Section -->
      <Collapsible v-model:open="stylesOpen">
        <Card
          class="
            overflow-hidden rounded-xl border-border/50 bg-card/50
            backdrop-blur-sm
          "
        >
          <CollapsibleTrigger
            class="
              flex w-full items-center gap-3 p-4 transition-colors
              hover:bg-muted/30
            "
          >
            <ChevronRight
              class="
                size-4 text-muted-foreground transition-transform duration-200
              "
              :class="{ 'rotate-90': stylesOpen }"
            />
            <div
              class="
                flex size-8 items-center justify-center rounded-lg bg-primary/10
              "
            >
              <Palette class="size-4 text-primary" />
            </div>
            <span class="font-medium">Styles</span>
            <Badge variant="secondary" class="ml-auto rounded-lg">
              {{ filteredStyles.length }}
            </Badge>
          </CollapsibleTrigger>

          <CollapsibleContent>
            <Separator class="bg-border/50" />
            <div class="p-4">
              <!-- Loading -->
              <div v-if="isLoadingStyles" class="flex justify-center py-12">
                <div
                  class="
                    size-8 animate-spin rounded-full border-2 border-muted
                    border-t-primary
                  "
                ></div>
              </div>

              <!-- Empty state -->
              <div v-else-if="!hasStyles" class="py-12 text-center">
                <div
                  class="
                    mx-auto mb-4 flex size-16 items-center justify-center
                    rounded-2xl bg-muted/50
                  "
                >
                  <Palette class="size-8 text-muted-foreground" />
                </div>
                <p class="font-medium">
                  No styles configured
                </p>
                <p class="mt-1 text-sm text-muted-foreground">
                  Add styles to your config.toml
                </p>
              </div>

              <!-- No results -->
              <div
                v-else-if="filteredStyles.length === 0"
                class="py-12 text-center text-muted-foreground"
              >
                No styles match "{{ searchQuery }}"
              </div>

              <!-- Style list -->
              <div v-else class="space-y-3">
                <div
                  v-for="style in filteredStyles"
                  :key="style.id"
                  class="
                    group rounded-xl border border-border/50 bg-background/50
                    p-4 transition-all
                    hover:border-primary/30 hover:shadow-lg
                    hover:shadow-primary/5
                  "
                >
                  <div class="flex gap-4">
                    <!-- Preview thumbnail -->
                    <div
                      class="
                        size-20 shrink-0 overflow-hidden rounded-xl bg-muted
                        ring-1 ring-border/50
                      "
                    >
                      <img
                        :src="`/styles/${style.id}/static/0,0,1/160x160.png`"
                        :alt="style.name"
                        class="
                          size-full object-cover transition-transform
                          group-hover:scale-105
                        "
                        loading="lazy"
                      />
                    </div>

                    <!-- Content -->
                    <div class="min-w-0 flex-1">
                      <div class="flex items-start justify-between gap-2">
                        <div>
                          <h3 class="font-semibold">
                            {{ style.name }}
                          </h3>
                          <p class="mt-0.5 text-sm text-muted-foreground">
                            <code
                              class="
                                rounded-md bg-muted px-1.5 py-0.5 text-xs
                                font-medium
                              "
                            >{{ style.id }}</code>
                          </p>
                        </div>

                        <!-- Action buttons -->
                        <div class="flex items-center gap-2">
                          <Button as-child size="sm" class="rounded-lg">
                            <NuxtLink :to="`/styles/${style.id}/#2/0/0`">
                              <Map class="mr-1.5 size-4" />
                              Viewer
                            </NuxtLink>
                          </Button>
                        </div>
                      </div>

                      <!-- View mode links -->
                      <div class="mt-2 flex items-center gap-3">
                        <NuxtLink
                          :to="`/styles/${style.id}/?raster#2/0/0`"
                          class="
                            flex items-center gap-1.5 rounded-lg bg-muted/50
                            px-2.5 py-1 text-xs font-medium
                            text-muted-foreground transition-colors
                            hover:bg-muted hover:text-foreground
                          "
                        >
                          <Image class="size-3.5" />
                          Raster
                        </NuxtLink>
                        <NuxtLink
                          :to="`/styles/${style.id}/#2/0/0`"
                          class="
                            flex items-center gap-1.5 rounded-lg bg-muted/50
                            px-2.5 py-1 text-xs font-medium
                            text-muted-foreground transition-colors
                            hover:bg-muted hover:text-foreground
                          "
                        >
                          <Grid3x3 class="size-3.5" />
                          Vector
                        </NuxtLink>
                      </div>

                      <!-- Service links -->
                      <div
                        class="
                          mt-3 flex flex-wrap items-center gap-x-2 gap-y-1
                          text-xs
                        "
                      >
                        <span class="text-muted-foreground">Services:</span>
                        <a
                          :href="`/styles/${style.id}/style.json`"
                          target="_blank"
                          class="
                            text-primary
                            hover:underline
                          "
                        >
                          GL Style
                        </a>
                        <span class="text-muted-foreground/30">•</span>
                        <a
                          :href="`/styles/${style.id}.json`"
                          target="_blank"
                          class="
                            text-primary
                            hover:underline
                          "
                        >
                          TileJSON
                        </a>
                        <span class="text-muted-foreground/30">•</span>
                        <a
                          :href="`/styles/${style.id}/wmts.xml`"
                          target="_blank"
                          class="
                            text-primary
                            hover:underline
                          "
                        >
                          WMTS
                        </a>
                        <span class="text-muted-foreground/30">•</span>
                        <button
                          class="
                            text-primary
                            hover:underline
                          "
                          @click="toggleStyleXyz(style.id)"
                        >
                          XYZ URL
                        </button>
                      </div>

                      <!-- XYZ URL -->
                      <div
                        v-if="expandedStyleXyz.has(style.id)"
                        class="
                          mt-2 flex items-center gap-2 rounded-lg bg-muted/50
                          p-2
                        "
                      >
                        <code
                          class="flex-1 truncate text-xs text-muted-foreground"
                        >
                          {{ baseUrl }}/styles/{{ style.id }}/{z}/{x}/{y}.png
                        </code>
                        <Button
                          variant="ghost"
                          size="icon"
                          class="size-7 shrink-0 rounded-lg"
                          @click="
                            copyUrl(
                              `${baseUrl}/styles/${style.id}/{z}/{x}/{y}.png`,
                            )
                          "
                        >
                          <Check
                            v-if="
                              copiedUrl
                                === `${baseUrl}/styles/${style.id}/{z}/{x}/{y}.png`
                            "
                            class="size-3.5 text-green-500"
                          />
                          <Copy v-else class="size-3.5" />
                        </Button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </CollapsibleContent>
        </Card>
      </Collapsible>

      <!-- Data Sources Section -->
      <Collapsible v-model:open="dataOpen">
        <Card
          class="
            overflow-hidden rounded-xl border-border/50 bg-card/50
            backdrop-blur-sm
          "
        >
          <CollapsibleTrigger
            class="
              flex w-full items-center gap-3 p-4 transition-colors
              hover:bg-muted/30
            "
          >
            <ChevronRight
              class="
                size-4 text-muted-foreground transition-transform duration-200
              "
              :class="{ 'rotate-90': dataOpen }"
            />
            <div
              class="
                flex size-8 items-center justify-center rounded-lg bg-primary/10
              "
            >
              <Database class="size-4 text-primary" />
            </div>
            <span class="font-medium">Data Sources</span>
            <Badge variant="secondary" class="ml-auto rounded-lg">
              {{ filteredDataSources.length }}
            </Badge>
          </CollapsibleTrigger>

          <CollapsibleContent>
            <Separator class="bg-border/50" />
            <div class="p-4">
              <!-- Loading -->
              <div v-if="isLoadingData" class="flex justify-center py-12">
                <div
                  class="
                    size-8 animate-spin rounded-full border-2 border-muted
                    border-t-primary
                  "
                ></div>
              </div>

              <!-- Empty state -->
              <div v-else-if="!hasData" class="py-12 text-center">
                <div
                  class="
                    mx-auto mb-4 flex size-16 items-center justify-center
                    rounded-2xl bg-muted/50
                  "
                >
                  <Database class="size-8 text-muted-foreground" />
                </div>
                <p class="font-medium">
                  No data sources configured
                </p>
                <p class="mt-1 text-sm text-muted-foreground">
                  Add PMTiles or MBTiles to config.toml
                </p>
              </div>

              <!-- No results -->
              <div
                v-else-if="filteredDataSources.length === 0"
                class="py-12 text-center text-muted-foreground"
              >
                No data sources match "{{ searchQuery }}"
              </div>

              <!-- Data source list -->
              <div v-else class="space-y-3">
                <div
                  v-for="source in filteredDataSources"
                  :key="source.id"
                  class="
                    group rounded-xl border border-border/50 bg-background/50
                    p-4 transition-all
                    hover:border-primary/30 hover:shadow-lg
                    hover:shadow-primary/5
                  "
                >
                  <div class="flex items-start gap-4">
                    <div
                      class="
                        flex size-12 shrink-0 items-center justify-center
                        rounded-xl bg-muted ring-1 ring-border/50
                      "
                    >
                      <Layers class="size-6 text-muted-foreground" />
                    </div>

                    <div class="min-w-0 flex-1">
                      <div class="flex items-start justify-between gap-2">
                        <div>
                          <h3 class="font-semibold">
                            {{ source.name || source.id }}
                          </h3>
                          <p
                            class="
                              mt-0.5 flex flex-wrap items-center gap-2 text-sm
                              text-muted-foreground
                            "
                          >
                            <code
                              class="
                                rounded-md bg-muted px-1.5 py-0.5 text-xs
                                font-medium
                              "
                            >{{ source.id }}</code>
                            <Badge
                              variant="outline"
                              class="rounded-md text-[10px]"
                            >
                              z{{ source.minzoom }}-{{ source.maxzoom }}
                            </Badge>
                          </p>
                        </div>

                        <Button
                          as-child
                          variant="secondary"
                          size="sm"
                          class="rounded-lg"
                        >
                          <NuxtLink :to="`/data/${source.id}/#2/0/0`">
                            <Layers class="mr-1.5 size-4" />
                            Inspect
                          </NuxtLink>
                        </Button>
                      </div>

                      <!-- Service links -->
                      <div
                        class="
                          mt-3 flex flex-wrap items-center gap-x-2 gap-y-1
                          text-xs
                        "
                      >
                        <span class="text-muted-foreground">Services:</span>
                        <a
                          :href="`/data/${source.id}.json`"
                          target="_blank"
                          class="
                            text-primary
                            hover:underline
                          "
                        >
                          TileJSON
                        </a>
                        <span class="text-muted-foreground/30">•</span>
                        <button
                          class="
                            text-primary
                            hover:underline
                          "
                          @click="toggleDataXyz(source.id)"
                        >
                          XYZ URL
                        </button>
                      </div>

                      <!-- XYZ URL -->
                      <div
                        v-if="expandedDataXyz.has(source.id)"
                        class="
                          mt-2 flex items-center gap-2 rounded-lg bg-muted/50
                          p-2
                        "
                      >
                        <code
                          class="flex-1 truncate text-xs text-muted-foreground"
                        >
                          {{ baseUrl }}/data/{{ source.id }}/{z}/{x}/{y}.pbf
                        </code>
                        <Button
                          variant="ghost"
                          size="icon"
                          class="size-7 shrink-0 rounded-lg"
                          @click="
                            copyUrl(
                              `${baseUrl}/data/${source.id}/{z}/{x}/{y}.pbf`,
                            )
                          "
                        >
                          <Check
                            v-if="
                              copiedUrl
                                === `${baseUrl}/data/${source.id}/{z}/{x}/{y}.pbf`
                            "
                            class="size-3.5 text-green-500"
                          />
                          <Copy v-else class="size-3.5" />
                        </Button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </CollapsibleContent>
        </Card>
      </Collapsible>

      <!-- API Documentation Link -->
      <a href="/_openapi" target="_blank" class="block">
        <Card
          class="
            overflow-hidden rounded-xl border-border/50 bg-card/50
            backdrop-blur-sm transition-all
            hover:border-primary/30 hover:shadow-lg hover:shadow-primary/5
          "
        >
          <div class="flex items-center gap-3 p-4">
            <div
              class="
                flex size-8 items-center justify-center rounded-lg bg-muted/50
              "
            >
              <ExternalLink class="size-4 text-muted-foreground" />
            </div>
            <div class="flex-1">
              <span class="font-medium">API Documentation</span>
              <p class="text-xs text-muted-foreground">
                OpenAPI 3.1 specification with Swagger UI
              </p>
            </div>
            <ChevronRight class="size-4 text-muted-foreground" />
          </div>
        </Card>
      </a>
    </main>

    <!-- Footer -->
    <footer class="mt-auto border-t border-border/50 py-6">
      <p class="text-center text-sm text-muted-foreground">
        Tileserver RS — Built with Rust + Axum + MapLibre GL JS
      </p>
    </footer>
  </div>
</template>
