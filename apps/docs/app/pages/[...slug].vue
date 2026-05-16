<script setup lang="ts">
  import { ArrowLeft } from '@lucide/vue';
  import { useDocsNavigation } from '~/composables/use-docs-navigation';

  const route = useRoute();

  const path = computed(() =>
    route.path.endsWith('/') && route.path !== '/'
      ? route.path.slice(0, -1)
      : route.path,
  );

  const { data: page } = await useAsyncData(`page-${path.value}`, () =>
    queryCollection('content').path(path.value).first(),
  );

  if (!page.value) {
    throw createError({
      statusCode: 404,
      statusMessage: 'Page not found',
      fatal: true,
    });
  }

  const { sections } = useDocsNavigation();

  const allItems = computed(() => sections.flatMap((s) => s.children));
  const currentIndex = computed(() =>
    allItems.value.findIndex((item) => item.path === path.value),
  );
  const prevItem = computed(() =>
    currentIndex.value > 0
      ? allItems.value[currentIndex.value - 1] ?? null
      : null,
  );
  const nextItem = computed(() =>
    currentIndex.value < allItems.value.length - 1
      ? allItems.value[currentIndex.value + 1] ?? null
      : null,
  );

  const editUrl = computed(() => {
    const slug = path.value.replace(/^\//, '');
    return `https://github.com/vinayakkulkarni/tileserver-rs/edit/main/apps/docs/content/${slug}.md`;
  });

  const issueUrl = computed(() => {
    const title = page.value?.title ?? 'Unknown';
    return `https://github.com/vinayakkulkarni/tileserver-rs/issues/new?title=${encodeURIComponent(`[Docs]: ${title} - `)}`;
  });

  useSeoMeta({
    title: page.value?.title,
    description: page.value?.description,
    ogTitle: page.value?.title,
    ogDescription: page.value?.description,
  });
</script>

<template>
  <div
    v-if="page"
    class="mx-auto max-w-3xl px-6 py-12"
  >
    <!-- Breadcrumb -->
    <NuxtLink
      to="/"
      class="mb-8 inline-flex items-center gap-1.5 font-mono text-xs uppercase tracking-wider text-muted-foreground transition-colors hover:text-foreground"
    >
      <ArrowLeft class="size-3" />
      Documentation
    </NuxtLink>

    <!-- Content -->
    <article class="prose">
      <ContentRenderer :value="page" />
    </article>

    <!-- Footer: Edit/Report + Prev/Next -->
    <DocsPageFooter
      :prev="prevItem"
      :next="nextItem"
      :edit-url="editUrl"
      :issue-url="issueUrl"
    />
  </div>
</template>
