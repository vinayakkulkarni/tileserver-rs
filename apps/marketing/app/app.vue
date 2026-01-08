<script setup lang="ts">
useHead({
  titleTemplate: (titleChunk) => {
    return titleChunk ? `${titleChunk} - Tileserver RS` : 'Tileserver RS';
  },
});
</script>

<template>
  <NuxtPage />
</template>
