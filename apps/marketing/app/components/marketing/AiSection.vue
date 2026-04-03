<script setup lang="ts">
  import {
    ArrowRight,
    ExternalLink,
    BotMessageSquare,
    User,
  } from 'lucide-vue-next';

  const { aiBenefits, aiChatExample } = useMarketingPage();
</script>

<template>
  <section data-label="AI Assistant" class="border-b border-border">
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
        AI Assistant
      </p>
      <h2
        class="
          max-w-2xl font-display text-3xl font-semibold
          lg:text-4xl
        "
        style="letter-spacing: -0.03em; line-height: 1.15"
      >
        AI Without the Landlord
      </h2>
      <p
        class="
          mt-4 max-w-2xl font-sans text-sm/relaxed text-muted-foreground
          lg:text-base
        "
      >
        Your maps, your GPU, your data. The built-in AI assistant runs entirely
        in your browser &mdash; no API keys, no cloud inference, no per-token
        billing. Just open the chat and talk to your maps.
      </p>
    </div>

    <!-- Benefits grid -->
    <div
class="
  grid gap-px border-y border-border bg-border
  md:grid-cols-4
">
      <div
        v-for="benefit in aiBenefits"
        :key="benefit.title"
        class="
          bg-background p-6
          lg:p-8
        "
      >
        <component :is="benefit.icon" class="mb-4 size-5 text-primary" />
        <h3 class="mb-2 font-display text-sm font-semibold tracking-tight">
          {{ benefit.title }}
        </h3>
        <p class="font-sans text-sm/relaxed text-muted-foreground">
          {{ benefit.description }}
        </p>
      </div>
    </div>

    <!-- Chat example + detail -->
    <div
      class="
        grid items-start gap-12 px-6 py-12
        md:px-12
        lg:grid-cols-2 lg:px-20
      "
    >
      <div class="overflow-hidden border border-border bg-muted/30">
        <div class="flex items-center gap-2 border-b border-border px-4 py-2.5">
          <BotMessageSquare class="size-3.5 text-primary" />
          <span class="font-mono text-xs text-muted-foreground">
            AI Chat — browser-local, WebGPU
          </span>
        </div>
        <div class="space-y-4 p-5">
          <div v-for="(msg, i) in aiChatExample" :key="i" class="flex gap-3">
            <div
              class="
                mt-0.5 flex size-6 shrink-0 items-center justify-center border
              "
              :class="
                msg.role === 'user'
                  ? 'border-border bg-muted text-muted-foreground'
                  : 'border-primary/30 bg-primary/10 text-primary'
              "
            >
              <User v-if="msg.role === 'user'" class="size-3" />
              <BotMessageSquare v-else class="size-3" />
            </div>
            <div class="min-w-0 flex-1">
              <p
                class="mb-0.5 font-mono text-[10px] tracking-wider uppercase"
                :class="
                  msg.role === 'user' ? 'text-muted-foreground' : 'text-primary'
                "
              >
                {{ msg.role === 'user' ? 'You' : 'AI' }}
              </p>
              <p
                class="font-mono text-sm whitespace-pre-line"
                :class="
                  msg.role === 'user'
                    ? 'text-foreground'
                    : 'text-muted-foreground'
                "
              >
                {{ msg.text }}
              </p>
            </div>
          </div>
        </div>
      </div>

      <div class="flex flex-col justify-center">
        <h3
          class="mb-4 font-display text-2xl font-semibold"
          style="letter-spacing: -0.02em; line-height: 1.15"
        >
          No tokens. No telemetry. No landlords.
        </h3>
        <p
          class="
            mb-4 font-sans text-sm/relaxed text-muted-foreground
            lg:text-base
          "
        >
          The AI runs a quantized LLM directly on your GPU via WebGPU &mdash;
          the same technology that powers browser gaming. First load downloads
          the model (~2 GB), then it&rsquo;s cached in your browser forever.
        </p>
        <ul class="mb-8 space-y-2">
          <li class="flex items-center gap-3 text-sm">
            <span class="size-1.5 bg-emerald-400"></span>
            <span class="text-foreground"
              >Works offline after first model download</span
            >
          </li>
          <li class="flex items-center gap-3 text-sm">
            <span class="size-1.5 bg-emerald-400"></span>
            <span class="text-foreground"
              >10+ map tools: navigate, filter, style, query</span
            >
          </li>
          <li class="flex items-center gap-3 text-sm">
            <span class="size-1.5 bg-emerald-400"></span>
            <span class="text-foreground"
              >Spatial queries against your tile data</span
            >
          </li>
          <li class="flex items-center gap-3 text-sm">
            <span class="size-1.5 bg-emerald-400"></span>
            <span class="text-foreground"
              >Chat history persisted locally in IndexedDB</span
            >
          </li>
        </ul>
        <div class="flex flex-wrap gap-3">
          <NuxtLink
            to="https://demo.tileserver.app"
            external
            class="
              group inline-flex items-center gap-2 border border-foreground
              bg-foreground px-5 py-2.5 font-mono text-xs tracking-wider
              text-background uppercase transition-colors
              hover:bg-transparent hover:text-foreground
            "
          >
            <ExternalLink class="size-3.5" />
            Try It Live
            <ArrowRight
              class="
                size-3.5 transition-transform
                group-hover:translate-x-0.5
              "
            />
          </NuxtLink>
          <NuxtLink
            to="https://docs.tileserver.app/guides/ai-chat"
            external
            class="
              inline-flex items-center gap-2 border border-border px-5 py-2.5
              font-mono text-xs tracking-wider text-muted-foreground uppercase
              transition-colors
              hover:border-foreground/20 hover:text-foreground
            "
          >
            Learn More
          </NuxtLink>
        </div>
      </div>
    </div>
  </section>
</template>
