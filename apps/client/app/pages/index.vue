<script setup lang="ts">
  import HomeToast from '~/components/ui/toast/Toast.vue';

  const {
    isDark,
    toggleColorMode,
    isLoadingData,
    isLoadingStyles,
    hasStyles,
    hasData,
    ping,
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
  } = useHomePage();
</script>

<template>
  <div class="flex min-h-dvh flex-col bg-background">
    <a class="skip-link" href="#main"> Skip to content </a>

    <HomeHero :is-dark="isDark" @toggle-theme="toggleColorMode" />

    <HomeToolbar
      :search-query="searchQuery"
      :all-chips="allChips"
      :active-filter="activeFilter"
      @update:search-query="searchQuery = $event"
      @select-filter="setFilter"
    />

    <main id="main" class="w-full flex-1">
      <div class="mx-auto max-w-7xl space-y-4 px-4 py-6 sm:px-6 lg:px-8">
        <HomeStyleList
          :styles="filteredStyles"
          :is-loading="isLoadingStyles"
          :has-styles="hasStyles"
          :is-open="stylesOpen"
          :search-query="searchQuery"
          :base-url="baseUrl"
          :expanded-xyz="expandedStyleXyz"
          :copied-url="copiedUrl"
          @update:is-open="stylesOpen = $event"
          @toggle-xyz="toggleStyleXyz"
          @copy-url="copyUrl"
        />

        <HomeDataList
          :sources="filteredDataSources"
          :is-loading="isLoadingData"
          :has-data="hasData"
          :is-open="dataOpen"
          :search-query="searchQuery"
          :base-url="baseUrl"
          :expanded-xyz="expandedDataXyz"
          :copied-url="copiedUrl"
          @update:is-open="dataOpen = $event"
          @toggle-xyz="toggleDataXyz"
          @copy-url="copyUrl"
        />

        <HomeApiLink />
      </div>
    </main>

    <HomeFooter :version-label="ping?.version ? `v${ping.version}` : ''" />

    <HomeToast :visible="toastVisible" :message="toastMessage" />
  </div>
</template>
