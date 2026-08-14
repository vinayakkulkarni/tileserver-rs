<script setup lang="ts">
  useHead({
    script: [
      {
        type: 'application/ld+json',
        innerHTML: JSON.stringify({
          '@context': 'https://schema.org',
          '@type': 'WebSite',
          'name': 'Tileserver RS Docs',
          'url': 'https://docs.tileserver.app',
          'description':
            'Documentation for tileserver-rs, a high-performance vector tile server built in Rust.',
          'potentialAction': {
            '@type': 'SearchAction',
            'target': {
              '@type': 'EntryPoint',
              'urlTemplate':
                'https://docs.tileserver.app/search?q={search_term_string}',
            },
            'query-input': 'required name=search_term_string',
          },
        }),
      },
    ],
  });
</script>

<template>
  <NuxtLayout>
    <NuxtPage />
  </NuxtLayout>
</template>
