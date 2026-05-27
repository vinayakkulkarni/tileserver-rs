import { FileText, LayoutDashboard, Plug, Smartphone } from '@lucide/vue';
import type { AdminNavGroup } from '~/types';
import { useThemeToggle } from '../use-theme-toggle';

const NAV_GROUPS: AdminNavGroup[] = [
  {
    heading: null,
    items: [
      { label: 'Dashboard', to: '/admin', icon: LayoutDashboard },
      { label: 'Configuration', to: '/admin/config', icon: FileText },
    ],
  },
  {
    heading: 'MCP',
    items: [
      { label: 'Connected apps', to: '/admin/mcp/connected-apps', icon: Plug },
      { label: 'Devices', to: '/admin/mcp/devices', icon: Smartphone },
    ],
  },
];

export function useAdminLayout() {
  const route = useRoute();
  const { isDark, toggle: toggleTheme } = useThemeToggle();

  // Mobile drawer state — sidebar is hidden <lg, opened via hamburger.
  const isMobileNavOpen = ref(false);

  function openMobileNav(): void {
    isMobileNavOpen.value = true;
  }

  function closeMobileNav(): void {
    isMobileNavOpen.value = false;
  }

  // Auto-close drawer when route changes so taps on nav links collapse it.
  watch(
    () => route.path,
    () => closeMobileNav(),
  );

  function isActive(to: string): boolean {
    if (to === '/admin') return route.path === '/admin';
    return route.path === to || route.path.startsWith(`${to}/`);
  }

  return {
    navGroups: NAV_GROUPS,
    isActive,
    isDark,
    toggleTheme,
    isMobileNavOpen,
    openMobileNav,
    closeMobileNav,
  };
}
