<script setup lang="ts">
  const { isDark } = useThemeToggle();

  usePageSeo({
    title: 'Tileserver RS - High-Performance Vector Tile Server',
    description:
      'High-performance vector tile server built in Rust with browser-local AI. Serve PMTiles and MBTiles with native MapLibre rendering — no API keys required.',
    path: '/',
  });

  const starColor = computed(() => (isDark.value ? '#a5b4fc' : '#6366f1'));

  const activeLabel = ref('Tileserver RS');

  onMounted(() => {
    const sections = document.querySelectorAll<HTMLElement>(
      'section[data-label]',
    );
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            activeLabel.value = entry.target.dataset.label ?? '';
          }
        }
      },
      { rootMargin: '-40% 0px -40% 0px' },
    );
    sections.forEach((s) => observer.observe(s));
    onUnmounted(() => observer.disconnect());
  });
</script>

<template>
  <div class="relative min-h-dvh overflow-x-hidden bg-background">
    <div class="pointer-events-none fixed inset-0">
      <Galaxy
        :speed="0.3"
        :star-count="1500"
        :star-size="2"
        :star-color="starColor"
      />
    </div>

    <div class="relative z-10">
      <MarketingNavigation />

      <!-- Fixed left gutter -->
      <div
        class="
          fixed top-[72px] bottom-0 left-0 z-20 flex w-[48px] items-center
          justify-center border-r border-border bg-background
          lg:top-[80px] lg:w-[80px]
        "
      >
        <Transition name="fade" mode="out-in">
          <span
            :key="activeLabel"
            class="
              vertical-text font-mono text-[10px] tracking-[0.3em]
              text-muted-foreground uppercase
            "
          >
            {{ activeLabel }}
          </span>
        </Transition>
      </div>

      <!-- Content with left margin -->
      <div
        class="
          mt-[72px] ml-[48px] w-[calc(100vw-48px)] overflow-hidden
          lg:mt-[80px] lg:ml-[80px] lg:w-[calc(100vw-80px)]
        "
      >
        <MarketingHeroSection />
        <MarketingFeaturesSection />
        <MarketingAiSection />
        <MarketingPerformanceSection />
        <MarketingConfigSection />
        <MarketingApiSection />
        <MarketingCtaSection />
        <MarketingFooter />
      </div>
    </div>
  </div>
</template>
