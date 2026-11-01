<script setup lang="ts">
  import { useAdminAnalytics } from '~/composables/admin/use-admin-analytics';
  import { ADMIN_STATS_REFRESH_MS } from '~/utils/api/admin-stats';

  definePageMeta({ layout: 'admin' });
  useHead({ title: 'Analytics · Admin · tileserver-rs' });

  const {
    breadcrumbs,
    stats,
    isLoading,
    error,
    isDisabled,
    figures,
    rateBars,
    zoomBarsList,
    latencyBarsList,
  } = useAdminAnalytics();
  const refreshSeconds = ADMIN_STATS_REFRESH_MS / 1000;
</script>

<template>
  <div class="flex min-h-dvh flex-col">
    <header class="border-b border-border px-page-x py-5 sm:py-6">
      <AdminBreadcrumb :items="breadcrumbs" />
      <h1
        class="mt-3 font-display text-3xl font-semibold tracking-tight text-foreground sm:text-4xl lg:text-5xl"
      >
        Analytics
      </h1>
      <p class="mt-2 max-w-2xl text-sm text-muted-foreground">
        Tile requests, cache performance and latency since the last start or
        settings change. Refreshes every {{ refreshSeconds }} s.
      </p>
    </header>

    <section v-if="error" class="border-b border-border px-page-x py-6 sm:py-8">
      <div class="max-w-2xl border border-border px-5 py-5 sm:px-6 sm:py-6">
        <p
          class="font-mono text-11 tracking-180 text-muted-foreground uppercase"
        >
          {{ error.title }}
        </p>
        <p class="mt-3 text-sm text-foreground">{{ error.body }}</p>
      </div>
    </section>
    <AdminAnalyticsDisabledNotice v-else-if="isDisabled" />
    <template v-else>
      <AdminAnalyticsOverview :figures="figures" :is-loading="isLoading" />
      <div class="grid grid-cols-1 border-b border-border lg:grid-cols-2">
        <AdminAnalyticsBarChart
          title="Requests per minute"
          caption="last 60 minutes"
          :bars="rateBars"
          :is-loading="isLoading"
          class="border-b border-border lg:border-r lg:border-b-0"
        />
        <AdminAnalyticsBarChart
          title="Latency"
          caption="all sources"
          :bars="latencyBarsList"
          :is-loading="isLoading"
        />
      </div>
      <AdminAnalyticsSourcesTable
        :sources="stats?.sources ?? []"
        :is-loading="isLoading"
        class="border-b border-border"
      />
      <div class="grid grid-cols-1 border-b border-border lg:grid-cols-split">
        <AdminAnalyticsHeatmap
          :points="stats?.heatmap ?? []"
          :zoom="stats?.heatmap_zoom ?? 0"
          :dropped="stats?.heatmap_dropped ?? 0"
          :is-loading="isLoading"
          class="border-b border-border lg:border-r lg:border-b-0"
        />
        <AdminAnalyticsBarChart
          title="Zoom levels"
          caption="requests per zoom"
          :bars="zoomBarsList"
          :is-loading="isLoading"
          show-labels
        />
      </div>
      <div class="grid grid-cols-1 lg:grid-cols-2">
        <AdminAnalyticsTopTiles
          :tiles="stats?.top_tiles ?? []"
          :is-loading="isLoading"
          class="border-b border-border lg:border-r lg:border-b-0"
        />
        <AdminAnalyticsRecentRequests
          :recent="stats?.recent ?? []"
          :is-loading="isLoading"
        />
      </div>
    </template>
  </div>
</template>
