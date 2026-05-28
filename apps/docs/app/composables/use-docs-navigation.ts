import type { NavSection } from '~/types';

export function useDocsNavigation() {
  const route = useRoute();

  const sections: NavSection[] = [
    {
      title: 'Getting Started',
      path: '/getting-started',
      children: [
        { title: 'Installation', path: '/getting-started/installation' },
        { title: 'Quickstart', path: '/getting-started/quickstart' },
        { title: 'Configuration', path: '/getting-started/config' },
      ],
    },
    {
      title: 'Benchmarks',
      path: '/benchmarks',
      children: [{ title: 'Performance', path: '/benchmarks/performance' }],
    },
    {
      title: 'API',
      path: '/api',
      children: [{ title: 'Endpoints', path: '/api/endpoints' }],
    },
    {
      title: 'Guides',
      path: '/guides',
      children: [
        { title: 'Static Images', path: '/guides/static-images' },
        { title: 'Vector Tiles', path: '/guides/vector-tiles' },
        { title: 'Docker', path: '/guides/docker' },
        {
          title: 'PostgreSQL Out-DB Rasters',
          path: '/guides/postgres-outdb-rasters',
        },
        { title: 'Telemetry', path: '/guides/telemetry' },
        { title: 'Auto-Detect', path: '/guides/auto-detect' },
        { title: 'Hot Reload', path: '/guides/hot-reload' },
        { title: 'MLT Tiles', path: '/guides/mlt' },
        { title: 'Cloud Deployment', path: '/guides/deploy' },
        { title: 'Drag & Drop Files', path: '/guides/file-drop' },
        { title: 'AI Chat', path: '/guides/ai-chat' },
      ],
    },
    {
      title: 'Integrations',
      path: '/integrations',
      children: [{ title: 'MapLibre', path: '/integrations/maplibre' }],
    },
    {
      title: 'Development',
      path: '/development',
      children: [{ title: 'Testing', path: '/development/testing' }],
    },
  ];

  const sidebarOpen = ref(false);

  function toggleSidebar() {
    sidebarOpen.value = !sidebarOpen.value;
  }

  function closeSidebar() {
    sidebarOpen.value = false;
  }

  const isActive = computed(() => (path: string) => route.path === path);

  const isInSection = computed(
    () => (sectionPath: string) => route.path.startsWith(sectionPath),
  );

  return {
    sections,
    sidebarOpen,
    toggleSidebar,
    closeSidebar,
    isActive,
    isInSection,
  };
}
