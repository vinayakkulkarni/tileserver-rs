<script setup lang="ts">
  defineProps<{
    versionLabel: string;
  }>();

  const currentYear = new Date().getFullYear();
</script>

<template>
  <footer class="footer shrink-0 border-t border-border" role="contentinfo">
    <div
      class="mx-auto flex max-w-screen-2xl flex-wrap items-center justify-between gap-3 px-[clamp(12px,4vw,24px)] py-5 text-[12px] text-muted-foreground"
    >
      <div>tileserver-rs · MIT · © {{ currentYear }}</div>
      <div v-if="versionLabel" class="font-mono text-[11px] tracking-[0.12em]">
        {{ versionLabel }}
      </div>
    </div>
  </footer>
</template>
