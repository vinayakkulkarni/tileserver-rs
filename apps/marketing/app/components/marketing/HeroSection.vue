<script setup lang="ts">
  import {
    Github,
    Sparkles,
    ArrowRight,
    ExternalLink,
    Terminal,
    Copy,
    Check,
    BotMessageSquare,
  } from 'lucide-vue-next';

  const { copied, installCommand, copyToClipboard } = useMarketingPage();
</script>

<template>
  <section
    data-label="Tileserver RS"
    class="section-full relative border-b border-border"
  >
    <!-- Geometric grid background -->
    <div
      class="geometric-grid-bg pointer-events-none absolute inset-0 opacity-30"
      aria-hidden="true"
    ></div>

    <div
      class="
        relative flex h-full min-h-[inherit] flex-col justify-center px-3 py-16
        sm:px-6
        md:px-12
        lg:px-20
      "
    >
      <div class="mx-auto w-full max-w-4xl min-w-0 text-center">
        <div
          class="
            mb-8 flex flex-col items-center justify-center gap-2 font-mono
            text-[10px] tracking-wider uppercase
            md:flex-row md:gap-4 md:text-xs
          "
        >
          <span
            class="
              inline-flex flex-wrap items-center justify-center gap-1.5
              text-muted-foreground
              sm:gap-2
            "
          >
            <span class="status-online"></span>
            Available on
            <a
              href="https://github.com/vinayakkulkarni/tileserver-rs/releases"
              class="
                text-primary
                hover:underline
              "
            >
              GitHub Releases
            </a>
          </span>

          <span
            class="
              inline-flex flex-wrap items-center justify-center gap-1.5
              text-muted-foreground
              sm:gap-2
            "
          >
            <BotMessageSquare class="size-3 text-primary" />
            <span class="text-primary">Browser-Local AI</span>
            &mdash; no API keys
          </span>
        </div>

        <h1
          class="
            mb-8 font-display text-3xl leading-[1.1] font-bold
            tracking-[-0.04em]
            sm:text-4xl
            md:text-5xl
            lg:text-6xl
            xl:text-7xl
          "
        >
          <span class="text-foreground">Serve Vector Tiles</span>
          <ShinyText
            text="At Lightning Speed"
            :speed="4"
            class="mt-2 block pb-2 font-display leading-[1.1]"
          />
        </h1>

        <SplitText
          text="High-performance tile server built in Rust. Serve PMTiles, MBTiles, PostGIS, and Cloud Optimized GeoTIFF with a browser-local AI assistant and native MapLibre rendering."
          by="words"
          :delay="30"
          :duration="0.4"
          class="
            mx-auto mb-10 max-w-2xl justify-center font-sans text-sm/relaxed
            text-muted-foreground
            sm:text-base/relaxed
            md:text-lg/relaxed
            lg:text-xl
          "
        />

        <div
          class="
            flex w-full flex-col items-center justify-center gap-3
            sm:flex-row
          "
        >
          <Button
            size="lg"
            as="a"
            href="https://docs.tileserver.app/getting-started/quickstart"
            class="
              group w-full gap-2 bg-primary px-6 text-primary-foreground
              hover:bg-primary/90
              sm:w-auto sm:px-8
            "
          >
            <Sparkles class="size-4" />
            Get Started
            <ArrowRight
              class="
                size-4 transition-transform
                group-hover:translate-x-0.5
              "
            />
          </Button>
          <Button
            variant="outline"
            size="lg"
            as="a"
            href="https://github.com/vinayakkulkarni/tileserver-rs"
            class="
              w-full gap-2 border-border bg-transparent
              hover:border-foreground/20 hover:bg-accent
              sm:w-auto
            "
          >
            <Github class="size-4" />
            View on GitHub
          </Button>
          <GlareHover
            width="auto"
            height="auto"
            background="transparent"
            border-radius="0"
            border-color="rgba(16, 185, 129, 0.2)"
            glare-color="#10b981"
            :glare-opacity="0.3"
            :glare-angle="-45"
            :glare-size="200"
            :transition-duration="500"
            class="
              w-full
              sm:w-auto
            "
          >
            <Button
              variant="outline"
              size="lg"
              as="a"
              href="https://demo.tileserver.app"
              target="_blank"
              class="
                w-full gap-2 border-0 bg-transparent text-emerald-400
                hover:bg-transparent
                sm:w-auto
              "
            >
              <ExternalLink class="size-4" />
              Live Demo
            </Button>
          </GlareHover>
        </div>

        <div
class="
  mx-auto mt-10 w-full
  sm:mt-14 sm:max-w-xl
">
          <div
            class="
              flex items-center gap-2 border border-border bg-muted/30 p-2.5
              sm:gap-4 sm:p-4
            "
          >
            <Terminal class="size-4 shrink-0 text-primary" />
            <code
              class="
                min-w-0 flex-1 truncate text-left font-mono text-[11px]
                text-foreground
                sm:text-sm
              "
            >
              {{ installCommand }}
            </code>
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger as-child>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    class="
                      shrink-0 text-muted-foreground
                      hover:text-primary
                    "
                    @click="copyToClipboard()"
                  >
                    <Check v-if="copied" class="size-4 text-emerald-400" />
                    <Copy v-else class="size-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent class="border-border bg-card">
                  <p>{{ copied ? 'Copied!' : 'Copy to clipboard' }}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
