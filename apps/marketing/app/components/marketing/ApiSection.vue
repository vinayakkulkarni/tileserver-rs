<script setup lang="ts">
  import { FileJson2, ExternalLink } from 'lucide-vue-next';

  const { apiEndpoints } = useMarketingPage();
</script>

<template>
  <section data-label="API Reference" class="border-b border-border">
    <div
      class="
        grid items-start gap-8 px-6 pt-16 pb-10
        md:px-12
        lg:grid-cols-2 lg:px-20
      "
    >
      <!-- Left: section header -->
      <div>
        <p
          class="
            mb-3 font-mono text-[10px] tracking-[0.3em] text-muted-foreground
            uppercase
            lg:text-xs
          "
        >
          API Reference
        </p>
        <h2
          class="
            mb-4 font-display text-3xl font-semibold
            lg:text-4xl
          "
          style="letter-spacing: -0.03em; line-height: 1.15"
        >
          RESTful API
        </h2>
        <p
class="
  font-sans text-sm/relaxed text-muted-foreground
  lg:text-base
">
          Simple, standards-compliant API with a built-in OpenAPI spec —
          something neither tileserver-gl nor martin offer.
          <NuxtLink
            to="https://demo.tileserver.app/_openapi/"
            external
            class="
              group ml-1 inline-flex items-center gap-1 font-mono text-xs
              tracking-wider text-primary uppercase transition-colors
              hover:text-primary/80
            "
          >
            Explore the API
            <ExternalLink
              class="
                size-3 transition-transform
                group-hover:translate-x-0.5
              "
            />
          </NuxtLink>
        </p>
      </div>

      <!-- Right: OpenAPI callout -->
      <div class="flex h-full items-center">
        <div class="w-full border border-primary/30 bg-primary/5 p-6">
          <div class="flex items-start gap-4">
            <FileJson2 class="mt-0.5 size-6 shrink-0 text-primary" />
            <div class="flex-1">
              <h3
                class="mb-1 font-display text-base font-semibold tracking-tight"
              >
                Interactive OpenAPI Spec
              </h3>
              <p class="font-sans text-sm/relaxed text-muted-foreground">
                Every endpoint fully documented with request/response schemas.
                Generate client SDKs, import into Postman, or browse the Scalar
                UI.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Endpoint groups grid -->
    <div
class="
  grid gap-px border-y border-border bg-border
  sm:grid-cols-2
">
      <div
        v-for="group in apiEndpoints"
        :key="group.title"
        class="
          bg-background p-6
          lg:p-8
        "
      >
        <h3
          class="
            mb-4 font-mono text-sm font-semibold tracking-wider text-foreground
            uppercase
          "
        >
          {{ group.title }}
        </h3>
        <div class="space-y-2">
          <div
            v-for="endpoint in group.endpoints"
            :key="endpoint.path"
            class="
              flex items-center gap-3 border border-border bg-muted/30 px-3 py-2
              font-mono text-sm
            "
          >
            <span
              class="
                bg-emerald-500/20 px-1.5 py-0.5 text-xs font-medium
                text-emerald-400
              "
            >
              {{ endpoint.method }}
            </span>
            <span class="text-muted-foreground">{{ endpoint.path }}</span>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
