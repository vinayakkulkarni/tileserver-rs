import { useClipboard, useTimeoutFn } from '@vueuse/core';
import type { Data } from '~/types/data';
import type { Style } from '~/types/style';
import { useHomeFilters } from './use-home-filters';
import { usePingStats } from './use-server-info';
import { useThemeToggle } from './use-theme-toggle';
import { useTileserverData } from './use-tileserver-data';

export function useHomePage() {
  const { isDark, toggle: toggleColorMode } = useThemeToggle();
  const {
    dataSources,
    styles,
    isLoadingData,
    isLoadingStyles,
    hasStyles,
    hasData,
  } = useTileserverData();

  const pingQuery = usePingStats();

  const { copy } = useClipboard();

  const {
    activeFilter,
    allChips,
    filteredStyles: typeFilteredStyles,
    filteredDataSources: typeFilteredDataSources,
    setFilter,
  } = useHomeFilters();

  const searchQuery = ref('');

  const filteredStyles = computed(() => {
    const list = typeFilteredStyles.value;
    if (!searchQuery.value) return list;
    const query = searchQuery.value.toLowerCase();
    return list.filter(
      (s: Style) =>
        s.name.toLowerCase().includes(query) ||
        s.id.toLowerCase().includes(query),
    );
  });

  const filteredDataSources = computed(() => {
    const list = typeFilteredDataSources.value;
    if (!searchQuery.value) return list;
    const query = searchQuery.value.toLowerCase();
    return list.filter(
      (s: Data) =>
        (s.name || '').toLowerCase().includes(query) ||
        s.id.toLowerCase().includes(query),
    );
  });

  const expandedStyleXyz = ref<Set<string>>(new Set());
  const expandedDataXyz = ref<Set<string>>(new Set());

  function toggleStyleXyz(styleId: string) {
    if (expandedStyleXyz.value.has(styleId)) {
      expandedStyleXyz.value.delete(styleId);
    } else {
      expandedStyleXyz.value.add(styleId);
    }
    expandedStyleXyz.value = new Set(expandedStyleXyz.value);
  }

  function toggleDataXyz(dataId: string) {
    if (expandedDataXyz.value.has(dataId)) {
      expandedDataXyz.value.delete(dataId);
    } else {
      expandedDataXyz.value.add(dataId);
    }
    expandedDataXyz.value = new Set(expandedDataXyz.value);
  }

  const toastVisible = ref(false);
  const toastMessage = ref('');

  function showToast(message: string, duration = 1800) {
    toastMessage.value = message;
    toastVisible.value = true;
    setTimeout(() => {
      toastVisible.value = false;
    }, duration);
  }

  const copiedUrl = ref<string | null>(null);
  const { start: startCopyTimer } = useTimeoutFn(
    () => {
      copiedUrl.value = null;
    },
    2000,
    { immediate: false },
  );

  function copyUrl(url: string) {
    copy(url);
    copiedUrl.value = url;
    showToast('XYZ URL copied');
    startCopyTimer();
  }

  const stylesOpen = ref(true);
  const dataOpen = ref(true);

  const baseUrl = computed(() => useRequestURL().origin);

  return {
    isDark,
    toggleColorMode,

    dataSources,
    styles,
    isLoadingData,
    isLoadingStyles,
    hasStyles,
    hasData,

    pingQuery,

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
    showToast,

    stylesOpen,
    dataOpen,

    baseUrl,
  };
}
