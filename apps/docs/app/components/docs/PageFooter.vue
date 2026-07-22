<script setup lang="ts">
  import {
    Pencil,
    CircleDot,
    ArrowLeft,
    ArrowRight,
    FileText,
  } from '@lucide/vue';
  import type { NavItem } from '~/types';

  defineProps<{
    prev: NavItem | null;
    next: NavItem | null;
    editUrl: string;
    issueUrl: string;
  }>();
</script>

<template>
  <footer class="mt-16 pt-8">
    <!-- Edit & Report -->
    <div
      class="flex items-center justify-center gap-1.5 text-xs lg:gap-2 lg:text-sm"
    >
      <div class="h-px flex-1 bg-border/50" />
      <a
        :href="editUrl"
        target="_blank"
        rel="noopener noreferrer"
        class="inline-flex items-center gap-1 border border-border/50 bg-card/50 px-2 py-1 text-muted-foreground backdrop-blur-sm transition-all hover:border-primary/30 hover:bg-primary/5 hover:text-primary lg:gap-1.5 lg:px-3 lg:py-1.5"
      >
        <Pencil class="size-3 lg:size-3.5" />
        Edit
      </a>
      <a
        :href="issueUrl"
        target="_blank"
        rel="noopener noreferrer"
        class="inline-flex items-center gap-1 border border-border/50 bg-card/50 px-2 py-1 text-muted-foreground backdrop-blur-sm transition-all hover:border-primary/30 hover:bg-primary/5 hover:text-primary lg:gap-1.5 lg:px-3 lg:py-1.5"
      >
        <CircleDot class="size-3 lg:size-3.5" />
        Report
      </a>
      <div class="h-px flex-1 bg-border/50" />
    </div>

    <!-- Prev / Next -->
    <div
      v-if="prev || next"
      class="mt-4 grid grid-cols-2 gap-2 lg:mt-6 lg:gap-4"
    >
      <NuxtLink
        v-if="prev"
        :to="prev.path"
        :aria-label="`Previous: ${prev.title}`"
        class="group relative flex flex-col overflow-hidden border border-border/50 bg-card/50 p-3 backdrop-blur-md transition-all duration-300 hover:border-primary/30 hover:bg-card/80 hover:shadow-lg hover:shadow-primary/5 lg:p-5"
      >
        <div
          class="pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-300 group-hover:opacity-100"
        >
          <div
            class="absolute -inset-px bg-linear-to-b from-primary/10 via-transparent to-transparent"
          />
        </div>
        <div
          class="relative mb-1.5 flex items-center gap-1.5 lg:mb-3 lg:gap-2"
        >
          <ArrowLeft
            class="size-3.5 text-muted-foreground transition-all duration-300 group-hover:-translate-x-1 group-hover:text-primary lg:size-4"
          />
          <span
            class="font-mono text-10 uppercase tracking-wider text-muted-foreground"
          >
            Previous
          </span>
        </div>
        <div class="relative flex items-center gap-2 lg:gap-3">
          <div
            class="flex size-7 shrink-0 items-center justify-center border border-border/50 bg-muted/50 transition-all duration-300 group-hover:border-primary/30 group-hover:bg-primary/10 lg:size-9"
          >
            <FileText
              class="size-3.5 text-muted-foreground transition-colors duration-300 group-hover:text-primary lg:size-4"
            />
          </div>
          <span
            class="font-display text-sm font-semibold tracking-tight transition-colors duration-300 group-hover:text-primary lg:text-base"
          >
            {{ prev.title }}
          </span>
        </div>
      </NuxtLink>
      <div v-else />

      <NuxtLink
        v-if="next"
        :to="next.path"
        :aria-label="`Next: ${next.title}`"
        class="group relative flex flex-col items-end overflow-hidden border border-border/50 bg-card/50 p-3 text-right backdrop-blur-md transition-all duration-300 hover:border-primary/30 hover:bg-card/80 hover:shadow-lg hover:shadow-primary/5 lg:p-5"
      >
        <div
          class="pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-300 group-hover:opacity-100"
        >
          <div
            class="absolute -inset-px bg-linear-to-b from-primary/10 via-transparent to-transparent"
          />
        </div>
        <div
          class="relative mb-1.5 flex items-center gap-1.5 lg:mb-3 lg:gap-2"
        >
          <span
            class="font-mono text-10 uppercase tracking-wider text-muted-foreground"
          >
            Next
          </span>
          <ArrowRight
            class="size-3.5 text-muted-foreground transition-all duration-300 group-hover:translate-x-1 group-hover:text-primary lg:size-4"
          />
        </div>
        <div class="relative flex items-center gap-2 lg:gap-3">
          <span
            class="font-display text-sm font-semibold tracking-tight transition-colors duration-300 group-hover:text-primary lg:text-base"
          >
            {{ next.title }}
          </span>
          <div
            class="flex size-7 shrink-0 items-center justify-center border border-border/50 bg-muted/50 transition-all duration-300 group-hover:border-primary/30 group-hover:bg-primary/10 lg:size-9"
          >
            <FileText
              class="size-3.5 text-muted-foreground transition-colors duration-300 group-hover:text-primary lg:size-4"
            />
          </div>
        </div>
      </NuxtLink>
    </div>
  </footer>
</template>
