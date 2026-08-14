<script setup lang="ts">
  useHead({
    titleTemplate: (titleChunk) => {
      return titleChunk ? `${titleChunk} - Tileserver RS` : 'Tileserver RS';
    },
    script: [
      {
        type: 'application/ld+json',
        innerHTML: JSON.stringify({
          '@context': 'https://schema.org',
          '@type': 'SoftwareApplication',
          applicationCategory: 'DeveloperApplication',
          operatingSystem: 'Linux, macOS, Windows, Docker',
          name: 'Tileserver RS',
          url: 'https://tileserver.app',
          description:
            'One Rust binary for all geospatial data. Serve vector tiles, raster tiles, PostGIS collections, COG, STAC, and OGC APIs, with a browser-local AI assistant and native MapLibre rendering.',
          codeRepository: 'https://github.com/vinayakkulkarni/tileserver-rs',
          programmingLanguage: 'Rust',
          license: 'https://opensource.org/licenses/MIT',
          author: {
            '@type': 'Person',
            name: 'Vinayak Kulkarni',
            url: 'https://vinayakkulkarni.dev',
          },
        }),
      },
    ],
  });
</script>

<template>
  <NuxtPage />
</template>
