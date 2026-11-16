<script setup lang="ts">
  import { ExternalLink } from '@lucide/vue';

  const { trackCta } = useMarketingTracking();
</script>

<template>
  <section data-label="Demo" class="border-b border-border">
    <div class="px-6 pt-16 pb-10 md:px-12 lg:px-20">
      <p
        class="mb-3 font-mono text-10 tracking-300 text-muted-foreground uppercase lg:text-xs"
      >
        30-second tour
      </p>
      <h2
        class="mb-4 max-w-2xl font-display text-3xl font-semibold lg:text-4xl"
        style="letter-spacing: -0.03em; line-height: 1.15"
      >
        Three servers&rsquo; worth of maps, one binary
      </h2>
      <p
        class="max-w-2xl font-sans text-sm/relaxed text-muted-foreground lg:text-base"
      >
        Recorded on the live demo: vector tiles, server-side rendering,
        Sentinel-2 imagery from STAC, terrain and the data inspector &mdash; all
        served by the same process.
      </p>
    </div>

    <div class="border-y border-border bg-muted/30 px-3 py-6 sm:px-6 lg:px-20">
      <video
        class="mx-auto block w-full max-w-5xl border border-border"
        src="/videos/tileserver-rs-demo.mp4"
        poster="/videos/tileserver-rs-demo-poster.jpg"
        autoplay
        muted
        loop
        playsinline
        preload="metadata"
        aria-label="tileserver-rs demo: vector tiles, static rendering, Sentinel-2 imagery, terrain and the data inspector"
      ></video>
    </div>

    <div class="px-6 py-8 md:px-12 lg:px-20">
      <a
        href="https://demo.tileserver.app"
        target="_blank"
        class="inline-flex items-center gap-2 font-mono text-xs tracking-wider text-primary uppercase hover:underline"
        @click="trackCta('demo_video', 'demo')"
      >
        <ExternalLink class="size-3.5" />
        Try the live demo
      </a>
    </div>
  </section>
</template>
