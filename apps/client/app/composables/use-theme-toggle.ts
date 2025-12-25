/**
 * Theme Toggle Composable
 *
 * Provides dark/light mode toggle functionality
 * using Nuxt color mode.
 */

export function useThemeToggle() {
  const colorMode = useColorMode();

  const isDark = computed(() => colorMode.value === 'dark');

  function toggle() {
    colorMode.preference = isDark.value ? 'light' : 'dark';
  }

  return {
    colorMode,
    isDark,
    toggle,
  };
}
