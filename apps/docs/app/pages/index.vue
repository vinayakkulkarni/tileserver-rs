<script setup lang="ts">
  import {
    ArrowRight,
    Zap,
    Globe,
    Layers,
    Database,
    Image,
    RefreshCw,
    ExternalLink,
    Map,
    Camera,
    Container,
    Sun,
    Moon,
    RotateCw,
    Server,
    FileSpreadsheet,
  } from '@lucide/vue';

  definePageMeta({ layout: false });

  const { isDark, toggle: toggleTheme } = useThemeToggle();
  const activeLabel = ref('Documentation');

  const starColor = computed(() =>
    isDark.value ? '#a5b4fc' : '#6366f1',
  );

  const backgroundColor = computed(() =>
    isDark.value ? '#030014' : '#f8fafc',
  );

  usePageSeo({
    title: 'Tileserver RS Docs - High-Performance Vector Tile Server',
    description:
      'Documentation for tileserver-rs — a high-performance vector tile server built in Rust.',
    path: '/',
  });

  const features = [
    {
      icon: Zap,
      title: 'Built in Rust',
      description:
        'High-performance tile serving with the safety and speed of Rust and Axum.',
    },
    {
      icon: Map,
      title: 'PMTiles Support',
      description:
        'Cloud-optimized tile archives with HTTP range request support.',
    },
    {
      icon: Database,
      title: 'MBTiles Support',
      description: 'SQLite-based tile storage for easy local development.',
    },
    {
      icon: Layers,
      title: 'MapLibre GL JS',
      description:
        'Built-in map viewer and data inspector powered by MapLibre GL JS.',
    },
    {
      icon: Globe,
      title: 'TileJSON 3.0',
      description:
        'Full TileJSON metadata API for seamless integration with mapping clients.',
    },
    {
      icon: Image,
      title: 'Native Raster Rendering',
      description:
        'Generate PNG/JPEG/WebP tiles from vector styles using MapLibre Native (~100ms/tile).',
    },
    {
      icon: Camera,
      title: 'Static Map Images',
      description:
        'Create embeddable map screenshots like Mapbox/Maptiler static API.',
    },
    {
      icon: Container,
      title: 'Docker Ready',
      description:
        'Easy deployment with Docker Compose v2 and multi-stage builds.',
    },
    {
      icon: RefreshCw,
      title: 'MLT Transcoding',
      description:
        'On-the-fly MLT↔MVT transcoding — serve next-gen MapLibre Tiles from existing MVT sources.',
    },
    {
      icon: RotateCw,
      title: 'Hot Reload',
      description:
        'Live configuration reload via SIGHUP or admin API — zero downtime source and style updates.',
    },
    {
      icon: Server,
      title: 'PostGIS Support',
      description:
        'Serve vector tiles directly from PostgreSQL/PostGIS with on-the-fly MVT generation.',
    },
    {
      icon: FileSpreadsheet,
      title: 'GeoParquet Source',
      description:
        'Query GeoParquet files via DuckDB with spatial filtering and automatic MVT encoding.',
    },
  ];

  const sections = [
    {
      title: 'Getting Started',
      path: '/getting-started/installation',
      description: 'Install, configure, and run your first tile server.',
    },
    {
      title: 'API Reference',
      path: '/api/endpoints',
      description: 'Full endpoint documentation for tiles, styles, and metadata.',
    },
    {
      title: 'Guides',
      path: '/guides/static-images',
      description: 'Static images, Docker, PostgreSQL, hot reload, and more.',
    },
    {
      title: 'Benchmarks',
      path: '/benchmarks/performance',
      description: 'Performance benchmarks and comparison with tileserver-gl.',
    },
  ];

  onMounted(() => {
    const sectionEls = document.querySelectorAll<HTMLElement>(
      'section[data-label]',
    );
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            activeLabel.value = entry.target.dataset.label ?? '';
          }
        }
      },
      { rootMargin: '-40% 0px -40% 0px' },
    );
    sectionEls.forEach((s) => observer.observe(s));
    onUnmounted(() => observer.disconnect());
  });
</script>

<template>
  <div class="min-h-dvh bg-background text-foreground">
    <!-- ═══ Header: Grid-cell nav (full-width, above gutter) ═══ -->
    <header class="fixed top-0 z-50 w-full bg-background">
      <div
        class="grid h-frame-inner grid-cols-frame-sm border-b border-border lg:h-frame-outer lg:grid-cols-frame"
      >
        <!-- Logo cell -->
        <div class="flex items-center justify-center border-r border-border">
          <NuxtLink
            to="/"
            class="flex items-center justify-center"
          >
            <Globe class="size-6 text-primary lg:size-7" />
          </NuxtLink>
        </div>

        <!-- Nav + title -->
        <div class="flex items-center px-5 lg:px-6">
          <NuxtLink
            to="/"
            class="flex items-center gap-2.5"
          >
            <span class="font-display text-sm font-semibold uppercase tracking-150">
              <span class="text-foreground">Tileserver</span><span class="text-primary"> RS</span>
            </span>
            <span
              class="border border-border bg-muted px-1.5 py-0.5 font-mono text-10 uppercase tracking-wider text-muted-foreground"
            >
              docs
            </span>
          </NuxtLink>
          <div class="ml-auto hidden items-center gap-6 lg:flex">
            <NuxtLink
              to="https://tileserver.app"
              external
              class="font-mono text-xs uppercase tracking-wider text-muted-foreground transition-colors hover:text-foreground"
            >
              Home
            </NuxtLink>
          </div>
        </div>

        <!-- Theme toggle cell (desktop) -->
        <div class="hidden items-center justify-center border-l border-border px-4 lg:flex">
          <button
            class="flex size-9 items-center justify-center text-muted-foreground transition-colors hover:text-foreground"
            aria-label="Toggle theme"
            @click="toggleTheme"
          >
            <Sun
              v-if="isDark"
              class="size-4"
            />
            <Moon
              v-else
              class="size-4"
            />
          </button>
        </div>

        <!-- Placeholder (mobile) -->
        <div class="flex items-center justify-center border-l border-border lg:hidden">
          <span class="font-mono text-xs text-muted-foreground">docs</span>
        </div>

        <!-- GitHub cell (desktop) -->
        <NuxtLink
          to="https://github.com/vinayakkulkarni/tileserver-rs"
          external
          class="hidden items-center justify-center border-l border-border text-muted-foreground transition-colors hover:text-foreground lg:flex"
        >
          <Icon
            name="simple-icons:github"
            class="size-5"
          />
        </NuxtLink>
      </div>
    </header>

    <!-- ═══ Fixed left gutter: persistent structural frame ═══ -->
    <div
      class="fixed left-0 top-frame-inner bottom-0 z-20 flex w-frame-gutter items-center justify-center border-r border-border bg-background lg:top-frame-outer lg:w-frame-outer"
    >
      <Transition
        name="fade"
        mode="out-in"
      >
        <span
          :key="activeLabel"
          class="vertical-text font-mono text-10 uppercase tracking-300 text-muted-foreground"
        >
          {{ activeLabel }}
        </span>
      </Transition>
    </div>

    <!-- Galaxy background -->
    <div class="pointer-events-none fixed inset-0">
      <GalaxyBackground
        :speed="0.3"
        :star-count="1500"
        :star-size="2"
        :star-color="starColor"
        :background-color="backgroundColor"
      />
    </div>

    <!-- ═══ Scrollable content area ═══ -->
    <div class="relative z-10 ml-frame-gutter mt-frame-inner lg:ml-frame-outer lg:mt-frame-outer">
      <!-- ═══ Hero: Full-height ═══ -->
      <section
        data-label="Documentation"
        class="section-full relative border-b border-border"
      >
        <div
          class="geometric-grid-bg pointer-events-none absolute inset-0 opacity-30"
          aria-hidden="true"
        />
        <div class="relative flex h-full min-h-[inherit] flex-col justify-center px-6 py-16 md:px-12 lg:px-20">
          <p class="mb-4 font-mono text-10 uppercase tracking-300 text-muted-foreground lg:text-xs">
            Documentation
          </p>
          <h1
            class="mb-6 max-w-3xl font-display text-5xl font-bold md:text-6xl"
            style="letter-spacing: -0.04em; line-height: 1.1"
          >
            Tileserver
            <span class="text-primary">RS</span>
          </h1>
          <p class="mb-10 max-w-2xl font-sans text-lg/relaxed text-muted-foreground">
            High-performance vector tile server built in Rust. Serve PMTiles and
            MBTiles with native raster rendering, static map images, and TileJSON
            3.0 metadata.
          </p>
          <div class="flex flex-wrap gap-3">
            <NuxtLink
              to="/getting-started/installation"
              class="group inline-flex items-center gap-2 border border-foreground bg-foreground px-5 py-2.5 font-mono text-xs uppercase tracking-wider text-background transition-colors hover:bg-transparent hover:text-foreground"
            >
              Get Started
              <ArrowRight
                class="size-3.5 transition-transform group-hover:translate-x-0.5"
              />
            </NuxtLink>
            <NuxtLink
              to="https://github.com/vinayakkulkarni/tileserver-rs"
              external
              class="inline-flex items-center gap-2 border border-border px-5 py-2.5 font-mono text-xs uppercase tracking-wider text-muted-foreground transition-colors hover:border-foreground/20 hover:text-foreground"
            >
              <Icon
                name="simple-icons:github"
                class="size-3.5"
              />
              View Source
            </NuxtLink>
            <NuxtLink
              to="https://demo.tileserver.app"
              external
              class="inline-flex items-center gap-2 border border-border px-5 py-2.5 font-mono text-xs uppercase tracking-wider text-muted-foreground transition-colors hover:border-foreground/20 hover:text-foreground"
            >
              <ExternalLink class="size-3.5" />
              Live Demo
            </NuxtLink>
          </div>
        </div>
      </section>

      <!-- ═══ Features ═══ -->
      <section
        data-label="Features"
        class="border-b border-border"
      >
        <div class="px-6 pt-16 pb-10 md:px-12 lg:px-20">
          <p class="mb-3 font-mono text-10 uppercase tracking-300 text-muted-foreground lg:text-xs">
            Features
          </p>
          <h2
            class="max-w-2xl font-display text-3xl font-semibold lg:text-4xl"
            style="letter-spacing: -0.03em; line-height: 1.15"
          >
            Everything you need to serve tiles
          </h2>
        </div>

        <!-- Feature grid -->
        <div class="grid gap-px border-t border-border bg-border md:grid-cols-3">
          <div
            v-for="feature in features"
            :key="feature.title"
            class="bg-background p-6 lg:p-8"
          >
            <component
              :is="feature.icon"
              class="mb-4 size-5 text-primary"
            />
            <h3 class="mb-2 font-display text-sm font-semibold tracking-tight">
              {{ feature.title }}
            </h3>
            <p class="font-sans text-sm/relaxed text-muted-foreground">
              {{ feature.description }}
            </p>
          </div>
        </div>
      </section>

      <!-- ═══ Explore (Quick links) ═══ -->
      <section
        data-label="Explore"
        class="border-b border-border"
      >
        <div class="px-6 pt-16 pb-10 md:px-12 lg:px-20">
          <p class="mb-3 font-mono text-10 uppercase tracking-300 text-muted-foreground lg:text-xs">
            Explore
          </p>
          <h2
            class="max-w-2xl font-display text-3xl font-semibold lg:text-4xl"
            style="letter-spacing: -0.03em; line-height: 1.15"
          >
            Jump into the docs
          </h2>
        </div>

        <!-- Links grid -->
        <div class="grid gap-px border-t border-border bg-border md:grid-cols-2">
          <NuxtLink
            v-for="section in sections"
            :key="section.path"
            :to="section.path"
            class="group bg-background p-6 transition-colors hover:bg-muted/30 lg:p-8"
          >
            <h3 class="mb-2 font-display text-sm font-semibold tracking-tight group-hover:text-primary">
              {{ section.title }}
            </h3>
            <p class="mb-4 font-sans text-sm/relaxed text-muted-foreground">
              {{ section.description }}
            </p>
            <span
              class="inline-flex items-center gap-1.5 font-mono text-10 uppercase tracking-wider text-muted-foreground group-hover:text-foreground"
            >
              Read more
              <ArrowRight
                class="size-3 transition-transform group-hover:translate-x-0.5"
              />
            </span>
          </NuxtLink>
        </div>
      </section>

      <!-- ═══ Footer ═══ -->
      <footer>
        <div class="flex items-center justify-between px-5 py-6 lg:px-8">
          <div class="flex items-center gap-3">
            <Globe class="size-4 text-muted-foreground" />
            <span class="font-mono text-10 uppercase tracking-wider text-muted-foreground lg:text-xs">
              &copy; {{ new Date().getFullYear() }}
              <NuxtLink
                to="https://vinayakkulkarni.dev"
                external
                class="transition-colors hover:text-foreground"
              >Vinayak Kulkarni</NuxtLink>
            </span>
          </div>
          <div class="flex items-center gap-4">
            <NuxtLink
              to="https://tileserver.app"
              external
              class="font-mono text-10 uppercase tracking-wider text-muted-foreground transition-colors hover:text-foreground lg:text-xs"
            >
              Home
            </NuxtLink>
            <NuxtLink
              to="https://github.com/vinayakkulkarni/tileserver-rs"
              external
              class="font-mono text-10 uppercase tracking-wider text-muted-foreground transition-colors hover:text-foreground lg:text-xs"
            >
              GitHub
            </NuxtLink>
            <NuxtLink
              to="https://github.com/vinayakkulkarni/tileserver-rs/blob/main/LICENSE"
              external
              class="font-mono text-10 uppercase tracking-wider text-muted-foreground transition-colors hover:text-foreground lg:text-xs"
            >
              MIT License
            </NuxtLink>
          </div>
        </div>
      </footer>
    </div>
  </div>
</template>

<style scoped>
  .fade-enter-active,
  .fade-leave-active {
    transition: opacity 0.15s ease;
  }

  .fade-enter-from,
  .fade-leave-to {
    opacity: 0;
  }
</style>
