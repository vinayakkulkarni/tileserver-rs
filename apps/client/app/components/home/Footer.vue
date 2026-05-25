<script setup lang="ts">
  defineProps<{
    versionLabel: string;
  }>();
</script>

<template>
  <footer
    class="footer border-t border-border py-5 flex flex-wrap gap-3 items-center justify-between text-[12px] text-muted-foreground"
    role="contentinfo"
  >
    <div>tileserver-rs · MIT · © 2026</div>
    <div v-if="versionLabel" class="font-mono text-[11px] tracking-[0.12em]">
      {{ versionLabel }}
    </div>
  </footer>
</template>
