<script setup lang="ts">
  defineProps<{
    versionLabel: string;
  }>();

  const currentYear = new Date().getFullYear();
</script>

<template>
  <footer class="footer shrink-0 border-t border-border" role="contentinfo">
    <div
      class="mx-auto flex max-w-screen-2xl flex-wrap items-center justify-between gap-3 px-page-x-sm py-5 text-12 text-muted-foreground"
    >
      <div>tileserver-rs · MIT · © {{ currentYear }}</div>
      <div v-if="versionLabel" class="font-mono text-11 tracking-120">
        {{ versionLabel }}
      </div>
    </div>
  </footer>
</template>
