<script setup lang="ts">
  import { FileWarning, TriangleAlert } from '@lucide/vue';
  import { useAdminConfig } from '~/composables/admin/use-admin-config';

  definePageMeta({ layout: 'admin' });
  useHead({ title: 'Configuration · tileserver-rs admin' });

  const {
    isPending,
    error,
    friendly,
    sourcePath,
    configHashShort,
    sections,
    breadcrumbs,
  } = useAdminConfig();
</script>

<template>
  <div class="flex min-h-dvh flex-col">
    <header class="border-b border-border px-10 py-6">
      <AdminBreadcrumb :items="breadcrumbs" />
      <div class="mt-3 flex flex-wrap items-baseline gap-x-6 gap-y-2">
        <h1 class="text-2xl font-semibold tracking-tight text-foreground">
          Configuration
        </h1>
        <span
          v-if="sourcePath"
          class="font-mono text-[11px] tracking-wider text-muted-foreground uppercase"
        >
          {{ sourcePath }} · #{{ configHashShort }}
        </span>
      </div>
      <p class="mt-2 max-w-3xl text-sm text-muted-foreground">
        Read-only view of the loaded server config. Each section shows the
        keys you have set, followed by every other key available to add —
        muted, with type, default, and description inline. Edit the source
        file in your editor; tileserver-rs hot-reloads on file changes.
      </p>
    </header>

    <section class="flex-1 px-10 py-8">
      <div v-if="isPending" class="flex flex-col gap-3">
        <div
          v-for="i in 8"
          :key="i"
          class="h-8 w-full max-w-3xl bg-muted"
        ></div>
      </div>

      <div
        v-else-if="error"
        class="flex max-w-2xl flex-col gap-3 border border-destructive/40 bg-destructive/10 p-6"
      >
        <div class="flex items-center gap-2 font-mono text-[11px] tracking-wider text-destructive uppercase">
          <TriangleAlert class="size-4" />
          {{ friendly.title }}
        </div>
        <p class="text-sm text-foreground">{{ friendly.body }}</p>
        <p v-if="friendly.hint" class="text-xs text-muted-foreground">
          {{ friendly.hint }}
        </p>
      </div>

      <div v-else class="flex flex-col gap-10">
        <article
          v-for="section in sections"
          :key="section.schema.header"
          class="flex flex-col gap-3"
        >
          <header class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
            <code class="font-mono text-sm font-semibold text-foreground">
              {{ section.schema.header }}
            </code>
            <span
              v-if="section.schema.featureGate"
              class="border border-border bg-muted px-1.5 py-0.5 font-mono text-[10px] tracking-wider text-muted-foreground uppercase"
            >
              feature: {{ section.schema.featureGate }}
            </span>
            <span
              v-if="!section.isPresent"
              class="font-mono text-[10px] tracking-wider text-muted-foreground uppercase"
            >
              not in your config
            </span>
            <span
              v-else-if="section.occurrences > 1"
              class="font-mono text-[10px] tracking-wider text-muted-foreground uppercase"
            >
              {{ section.occurrences }} entries
            </span>
            <span class="text-xs text-muted-foreground">
              {{ section.schema.blurb }}
            </span>
          </header>

          <pre class="overflow-x-auto border border-border bg-card p-4 font-mono text-[13px] leading-relaxed"><template
            v-for="(line, idx) in section.lines"
            :key="`${section.schema.header}-${idx}`"
          ><span
            v-if="line.kind === 'section-header'"
            class="text-foreground"
          >{{ line.header }}
</span><span
            v-else-if="line.kind === 'set'"
            class="text-foreground"
          >{{ line.rendered }}
</span><span
            v-else-if="line.kind === 'suggestion'"
            class="text-muted-foreground/70"
          ># {{ line.key }} = &lt;{{ line.schema.default ?? line.schema.type }}&gt;<span
            v-if="line.schema.optional"
            class="text-muted-foreground/50"
          > # optional</span> # {{ line.schema.description }}
</span></template></pre>
        </article>

        <footer
          class="mt-4 flex max-w-3xl items-start gap-3 border border-border bg-card/40 p-5"
        >
          <FileWarning class="mt-0.5 size-4 shrink-0 text-muted-foreground" />
          <div class="flex flex-col gap-1.5 text-xs text-muted-foreground">
            <p>
              This view is read-only. To change a key, edit
              <code v-if="sourcePath" class="text-foreground">{{ sourcePath }}</code>
              <span v-else>your config file</span>
              in your editor and restart the server (or send
              <code class="text-foreground">POST /__admin/reload</code>).
            </p>
            <p>
              Type signatures, defaults, and feature gates are sourced from
              <code class="text-foreground">crates/tileserver-rs/src/config.rs</code>.
              See the Configuration reference in the docs for full details.
            </p>
          </div>
        </footer>
      </div>
    </section>
  </div>
</template>
