<script setup lang="ts">
  import { Database, Palette } from '@lucide/vue';
  import HomeToast from '~/components/ui/toast/Toast.vue';

  const {
    isLoadingData,
    isLoadingStyles,
    hasStyles,
    hasData,
    searchQuery,
    activeFilter,
    allChips,
    setFilter,
    filteredStyles,
    filteredDataSources,
    expandedStyleXyz,
    expandedDataXyz,
    toggleStyleXyz,
    toggleDataXyz,
    toastVisible,
    toastMessage,
    copiedUrl,
    copyUrl,
    stylesOpen,
    dataOpen,
    baseUrl,
    pingQuery,
  } = useHomePage();

  const handleSearch = (value: string) => {
    searchQuery.value = value;
  };

  const handleFilter = (category: FilterCategory) => {
    setFilter(category);
  };
</script>

<template>
  <div class="flex min-h-dvh flex-col bg-background">
    <a class="skip-link" href="#main">Skip to content</a>

    <HomeHeader />
    <HomeHero />

    <HomeToolbar
      :search-query="searchQuery"
      :all-chips="allChips"
      :active-filter="activeFilter"
      @update:search-query="handleSearch"
      @select-filter="handleFilter"
    />

    <main id="main" class="w-full flex-1" role="main">
      <div
        class="mx-auto max-w-[1600px] px-[clamp(12px,4vw,24px)] py-5 flex flex-col gap-4"
      >
        <HomeSection
          title="Map styles"
          :count="filteredStyles.length"
          :icon="Palette"
          :is-open="stylesOpen"
          @toggle-section="stylesOpen = !stylesOpen"
        >
          <template v-if="isLoadingStyles">
            <HomeStyleCardSkeleton v-for="i in 5" :key="i" />
          </template>
          <template v-else-if="!hasStyles">
            <div class="col-span-full py-12 text-center">
              <div
                class="mx-auto mb-4 flex size-16 items-center justify-center bg-surface-2"
              >
                <Palette class="size-8 text-muted-foreground" />
              </div>
              <p class="font-medium">No styles configured</p>
              <p class="mt-1 text-sm text-muted-foreground">
                Add styles to your config.toml
              </p>
            </div>
          </template>
          <template v-else-if="filteredStyles.length === 0">
            <div class="col-span-full py-12 text-center text-muted-foreground">
              No styles match "{{ searchQuery }}"
            </div>
          </template>
          <template v-else>
            <HomeStyleCard
              v-for="(style, i) in filteredStyles"
              :key="style.id"
              :style="style"
              :index="i"
              :base-url="baseUrl"
              :is-xyz-expanded="expandedStyleXyz.has(style.id)"
              :copied-url="copiedUrl"
              @toggle-xyz="toggleStyleXyz"
              @copy-url="copyUrl"
            />
          </template>
        </HomeSection>

        <HomeSection
          title="Data sources"
          :count="filteredDataSources.length"
          :icon="Database"
          :is-open="dataOpen"
          @toggle-section="dataOpen = !dataOpen"
        >
          <template v-if="isLoadingData">
            <HomeDataCardSkeleton v-for="i in 5" :key="i" />
          </template>
          <template v-else-if="!hasData">
            <div class="col-span-full py-12 text-center">
              <div
                class="mx-auto mb-4 flex size-16 items-center justify-center bg-surface-2"
              >
                <Database class="size-8 text-muted-foreground" />
              </div>
              <p class="font-medium">No data sources configured</p>
              <p class="mt-1 text-sm text-muted-foreground">
                Add PMTiles or MBTiles to config.toml
              </p>
            </div>
          </template>
          <template v-else-if="filteredDataSources.length === 0">
            <div class="col-span-full py-12 text-center text-muted-foreground">
              No data sources match "{{ searchQuery }}"
            </div>
          </template>
          <template v-else>
            <HomeDataCard
              v-for="(source, i) in filteredDataSources"
              :key="source.id"
              :source="source"
              :index="i"
              :base-url="baseUrl"
              :is-xyz-expanded="expandedDataXyz.has(source.id)"
              :copied-url="copiedUrl"
              @toggle-xyz="toggleDataXyz"
              @copy-url="copyUrl"
            />
          </template>
        </HomeSection>

        <HomeApiLink />
      </div>
    </main>

    <HomeFooter
      :version-label="
        pingQuery.data.value?.version ? `v${pingQuery.data.value.version}` : ''
      "
    />

    <HomeToast :visible="toastVisible" :message="toastMessage" />
  </div>
</template>
