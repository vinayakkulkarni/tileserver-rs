<script setup lang="ts"></script>

<template>
  <article class="card border border-border bg-background p-3.5">
    <div class="flex gap-3.5">
      <Skeleton class="size-14 shrink-0" />
      <div class="min-w-0 flex-1">
        <Skeleton class="h-4 w-2/3" />
        <div class="mt-1.5 flex gap-2">
          <Skeleton class="h-3 w-20" />
          <Skeleton class="h-3 w-12" />
        </div>
        <div class="mt-3 flex gap-2">
          <Skeleton class="h-6 w-16" />
        </div>
      </div>
    </div>
  </article>
</template>
