<script setup lang="ts">
  const configFeatures = [
    'Multiple tile sources (PMTiles, MBTiles, PostGIS, COG)',
    'Custom MapLibre GL styles',
    'In-memory tile caching with TTL',
    'Configurable CORS and connection pooling',
  ];
</script>

<template>
  <section
    data-label="Configuration"
    class="border-b border-border bg-background"
  >
    <div
class="
  px-6 pt-16 pb-10
  md:px-12
  lg:px-20
">
      <p
        class="
          mb-3 font-mono text-[10px] tracking-[0.3em] text-muted-foreground
          uppercase
          lg:text-xs
        "
      >
        Configuration
      </p>
      <h2
        class="
          max-w-2xl font-display text-3xl font-semibold
          lg:text-4xl
        "
        style="letter-spacing: -0.03em; line-height: 1.15"
      >
        Simple Setup
      </h2>
    </div>

    <div
class="
  grid gap-px border-y border-border bg-border
  lg:grid-cols-2
">
      <!-- Left: description -->
      <div
class="
  bg-background p-6
  lg:p-8
">
        <p
          class="
            mb-6 font-sans text-sm/relaxed text-muted-foreground
            lg:text-base
          "
        >
          Get started with a simple TOML configuration file. Define your tile
          sources, styles, and server settings in one place.
        </p>
        <ul class="space-y-3">
          <li
            v-for="item in configFeatures"
            :key="item"
            class="flex items-center gap-3 text-sm"
          >
            <span class="size-1.5 bg-primary"></span>
            <span class="text-foreground">{{ item }}</span>
          </li>
        </ul>
      </div>

      <!-- Right: code block -->
      <div class="overflow-hidden bg-background">
        <div class="flex items-center gap-2 border-b border-border px-4 py-2.5">
          <span class="size-2.5 rounded-full bg-red-500/60"></span>
          <span class="size-2.5 rounded-full bg-yellow-500/60"></span>
          <span class="size-2.5 rounded-full bg-green-500/60"></span>
          <span class="ml-2 font-mono text-xs text-muted-foreground">
            config.toml
          </span>
        </div>
        <!-- eslint-disable vue/no-v-html -->
        <pre
          class="overflow-x-auto bg-background p-6 font-mono text-sm/relaxed"
        ><code><span class="token-comment"># Tile sources</span>
<span class="token-keyword">[[sources]]</span>
id = <span class="token-string">"openmaptiles"</span>
type = <span class="token-string">"pmtiles"</span>
path = <span class="token-string">"/data/tiles.pmtiles"</span>
serve_as = <span class="token-string">"mlt"</span>  <span class="token-comment"># MVT→MLT on the fly</span>

<span class="token-comment"># PostgreSQL / PostGIS</span>
<span class="token-keyword">[postgres]</span>
connection_string = <span class="token-string">"postgresql://user:pass@localhost/db"</span>

<span class="token-keyword">[[postgres.tables]]</span>
id = <span class="token-string">"buildings"</span>
table = <span class="token-string">"buildings"</span>
geometry_column = <span class="token-string">"geom"</span></code></pre>
      </div>
    </div>
  </section>
</template>
