<script setup lang="ts">
  import { useLiveQuery } from '@tanstack/vue-db'
  import type { Style } from '~/types'
  import {
    useDataSourcesCollection,
    useMapStylesCollection,
  } from '~/utils/collections'

  const router = useRouter()

  const { dataSourcesCollection } = useDataSourcesCollection()
  const { mapStylesCollection } = useMapStylesCollection()

  const { data: dataSources, isLoading: isLoadingData } = useLiveQuery(
    dataSourcesCollection,
  )
  const { data: styles, isLoading: isLoadingStyles } =
    useLiveQuery(mapStylesCollection)

  function navigateToService(type: string, style: Style) {
    if (type === 'gl-style') {
      router.push(`/styles/${style.id}/style.json`)
    } else if (type === 'tilejson') {
      router.push(`/styles/${style.id}.json`)
    } else if (type === 'wmts') {
      router.push(`/styles/${style.id}/wmts.xml`)
    }
  }
</script>

<template>
  <main
    class="min-h-screen bg-linear-to-br from-slate-50 to-slate-100 dark:from-slate-900 dark:to-slate-800"
  >
    <div class="container mx-auto px-4 py-16">
      <!-- Header -->
      <header class="text-center mb-16">
        <h1 class="text-5xl font-light text-slate-800 dark:text-slate-100 mb-4">
          Tileserver RS
        </h1>
        <p class="text-2xl font-light text-slate-600 dark:text-slate-400">
          Vector maps with GL styles
        </p>
      </header>

      <!-- Styles Section -->
      <section class="mb-16">
        <div
          class="bg-white dark:bg-slate-800 rounded-lg shadow-lg overflow-hidden max-w-4xl mx-auto"
        >
          <div
            class="px-8 py-4 border-b border-slate-200 dark:border-slate-700"
          >
            <h2
              class="text-xl font-bold text-slate-800 dark:text-slate-100 uppercase tracking-wide"
            >
              Styles
            </h2>
          </div>

          <div
            v-if="isLoadingStyles"
            class="p-8 text-center text-slate-500 dark:text-slate-400"
          >
            Loading styles...
          </div>

          <div
            v-else-if="!styles || styles.length === 0"
            class="p-8 text-center text-slate-500 dark:text-slate-400"
          >
            No styles available
          </div>

          <div
            v-for="(style, idx) in styles"
            :key="style.id"
            class="p-8 border-b last:border-b-0 border-slate-100 dark:border-slate-700"
            :class="{ 'bg-slate-50 dark:bg-slate-800/50': idx % 2 === 0 }"
          >
            <div class="flex items-center justify-between gap-6">
              <div class="flex items-center gap-6">
                <img
                  :src="`/styles/${style.id}/0/0/0.png`"
                  :alt="`${style.name} preview`"
                  class="size-32 object-cover rounded-lg border border-slate-200 dark:border-slate-600 shadow"
                />
                <div class="space-y-2">
                  <h3
                    class="text-lg font-bold text-slate-800 dark:text-slate-100"
                  >
                    {{ style.name }}
                  </h3>
                  <p class="text-sm text-slate-500 dark:text-slate-400">
                    identifier: {{ style.id }}
                  </p>
                  <div class="flex items-center gap-2 text-sm">
                    <span class="text-slate-500 dark:text-slate-400"
                      >services:</span
                    >
                    <button
                      type="button"
                      class="text-blue-600 dark:text-blue-400 hover:underline"
                      @click="navigateToService('gl-style', style)"
                    >
                      GL Style
                    </button>
                    <span class="text-slate-300 dark:text-slate-600">|</span>
                    <button
                      type="button"
                      class="text-blue-600 dark:text-blue-400 hover:underline"
                      @click="navigateToService('tilejson', style)"
                    >
                      TileJSON
                    </button>
                    <span class="text-slate-300 dark:text-slate-600">|</span>
                    <button
                      type="button"
                      class="text-blue-600 dark:text-blue-400 hover:underline"
                      @click="navigateToService('wmts', style)"
                    >
                      WMTS
                    </button>
                  </div>
                </div>
              </div>

              <NuxtLink
                :to="`/styles/${style.id}/#2/0.00000/0.00000`"
                class="inline-flex items-center rounded-lg bg-blue-600 px-6 py-3 text-base font-medium text-white shadow-sm hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 transition-colors"
              >
                Viewer
              </NuxtLink>
            </div>
          </div>
        </div>
      </section>

      <!-- Data Section -->
      <section>
        <div
          class="bg-white dark:bg-slate-800 rounded-lg shadow-lg overflow-hidden max-w-4xl mx-auto"
        >
          <div
            class="px-8 py-4 border-b border-slate-200 dark:border-slate-700"
          >
            <h2
              class="text-xl font-bold text-slate-800 dark:text-slate-100 uppercase tracking-wide"
            >
              Data
            </h2>
          </div>

          <div
            v-if="isLoadingData"
            class="p-8 text-center text-slate-500 dark:text-slate-400"
          >
            Loading data sources...
          </div>

          <div
            v-else-if="!dataSources || dataSources.length === 0"
            class="p-8 text-center text-slate-500 dark:text-slate-400"
          >
            No data sources available
          </div>

          <div
            v-for="(source, idx) in dataSources"
            :key="source.id"
            class="p-8 border-b last:border-b-0 border-slate-100 dark:border-slate-700"
            :class="{ 'bg-slate-50 dark:bg-slate-800/50': idx % 2 === 0 }"
          >
            <div class="flex items-center justify-between gap-6">
              <div class="flex items-center gap-6">
                <div
                  class="size-32 rounded-lg border border-slate-200 dark:border-slate-600 shadow bg-linear-to-br from-blue-100 to-blue-200 dark:from-blue-900 dark:to-blue-800 flex items-center justify-center"
                >
                  <svg
                    class="size-12 text-blue-500 dark:text-blue-400"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="1.5"
                      d="M9 20l-5.447-2.724A1 1 0 013 16.382V5.618a1 1 0 011.447-.894L9 7m0 13l6-3m-6 3V7m6 10l4.553 2.276A1 1 0 0021 18.382V7.618a1 1 0 00-.553-.894L15 4m0 13V4m0 0L9 7"
                    />
                  </svg>
                </div>
                <div class="space-y-2">
                  <h3
                    class="text-lg font-bold text-slate-800 dark:text-slate-100"
                  >
                    {{ source.name || source.id }}
                  </h3>
                  <p class="text-sm text-slate-500 dark:text-slate-400">
                    identifier: {{ source.id }} | type: vector data
                  </p>
                  <div class="flex items-center gap-2 text-sm">
                    <span class="text-slate-500 dark:text-slate-400"
                      >services:</span
                    >
                    <NuxtLink
                      :to="`/data/${source.id}.json`"
                      class="text-blue-600 dark:text-blue-400 hover:underline"
                    >
                      TileJSON
                    </NuxtLink>
                  </div>
                </div>
              </div>

              <NuxtLink
                :to="`/data/${source.id}/#2/0.00000/0.00000`"
                class="inline-flex items-center rounded-lg bg-blue-600 px-6 py-3 text-base font-medium text-white shadow-sm hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 transition-colors"
              >
                Inspect
              </NuxtLink>
            </div>
          </div>
        </div>
      </section>
    </div>
  </main>
</template>
