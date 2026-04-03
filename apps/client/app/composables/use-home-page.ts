/**
 * Home Page Composable
 *
 * Manages all state and logic for the landing page:
 * search filtering, collapsible sections, XYZ URL expansion, clipboard copy.
 */

import { useClipboard, useTimeoutFn } from '@vueuse/core';

import type { Data } from '~/types/data';
import type { Style } from '~/types/style';

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
  const { versionLabel } = useServerInfo();

  const { copy } = useClipboard();

  // Search filter
  const searchQuery = ref('');

  // Filtered lists
  const filteredStyles = computed(() => {
    if (!searchQuery.value) return styles.value;
    const query = searchQuery.value.toLowerCase();
    return styles.value.filter(
      (s: Style) =>
        s.name.toLowerCase().includes(query) ||
        s.id.toLowerCase().includes(query),
    );
  });

  const filteredDataSources = computed(() => {
    if (!searchQuery.value) return dataSources.value;
    const query = searchQuery.value.toLowerCase();
    return dataSources.value.filter(
      (s: Data) =>
        (s.name || '').toLowerCase().includes(query) ||
        s.id.toLowerCase().includes(query),
    );
  });

  // Track which XYZ URLs are expanded
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

  // Copy with feedback
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
    startCopyTimer();
  }

  // Collapsible sections
  const stylesOpen = ref(true);
  const dataOpen = ref(true);

  // Base URL for XYZ templates
  const baseUrl = computed(() => {
    if (import.meta.client) {
      return window.location.origin;
    }
    return '';
  });

  return {
    // Theme
    isDark,
    toggleColorMode,

    // Data
    dataSources,
    styles,
    isLoadingData,
    isLoadingStyles,
    hasStyles,
    hasData,

    // Server
    versionLabel,

    // Search
    searchQuery,
    filteredStyles,
    filteredDataSources,

    // XYZ expansion
    expandedStyleXyz,
    expandedDataXyz,
    toggleStyleXyz,
    toggleDataXyz,

    // Clipboard
    copiedUrl,
    copyUrl,

    // Sections
    stylesOpen,
    dataOpen,

    // URL
    baseUrl,
  };
}
