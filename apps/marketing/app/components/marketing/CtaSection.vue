<script setup lang="ts">
  import { Github, ArrowRight } from 'lucide-vue-next';
</script>

<template>
  <section data-label="Get Started" class="section-full">
    <div
      class="
        flex min-h-[inherit] flex-col items-center justify-center px-6 py-20
        text-center
        md:px-12
        lg:px-20 lg:py-28
      "
    >
      <h2
        class="
          mb-6 font-display text-3xl font-semibold
          lg:text-4xl
        "
        style="letter-spacing: -0.03em; line-height: 1.15"
      >
        Ready to Deploy?
      </h2>
      <p
        class="
          mb-10 max-w-lg font-sans text-sm text-muted-foreground
          lg:text-base
        "
      >
        Deploy your own tile server in minutes with our comprehensive
        documentation.
      </p>
      <div class="flex flex-wrap justify-center gap-3">
        <NuxtLink
          to="https://docs.tileserver.app/getting-started/quickstart"
          external
          class="
            group inline-flex items-center gap-2 border border-foreground
            bg-foreground px-5 py-2.5 font-mono text-xs tracking-wider
            text-background uppercase transition-colors
            hover:bg-transparent hover:text-foreground
          "
        >
          Read the Docs
          <ArrowRight
            class="
              size-3.5 transition-transform
              group-hover:translate-x-0.5
            "
          />
        </NuxtLink>
        <NuxtLink
          to="https://github.com/vinayakkulkarni/tileserver-rs"
          external
          class="
            inline-flex items-center gap-2 border border-border px-5 py-2.5
            font-mono text-xs tracking-wider text-muted-foreground uppercase
            transition-colors
            hover:border-foreground/20 hover:text-foreground
          "
        >
          <Github class="size-3.5" />
          Star on GitHub
        </NuxtLink>
      </div>
    </div>
  </section>
</template>
