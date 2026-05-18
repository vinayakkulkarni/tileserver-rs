import {
  Zap,
  Globe,
  RefreshCw,
  Database,
  MapIcon,
  HardDrive,
  Layers,
  Image,
  Server,
  Sparkles,
  Terminal,
  BarChart3,
  Upload,
  Cloud,
  BotMessageSquare,
  ShieldCheck,
  Cpu,
  MessageSquare,
  Puzzle,
  FileSpreadsheet,
  Braces,
  Timer,
  Satellite,
  FileCode2,
  Plug,
} from '@lucide/vue';
import type {
  Feature,
  FeatureCategory,
  FeatureGroup,
  ComparisonColumn,
  ComparisonRow,
  AiBenefit,
  AiChatMessage,
  ApiEndpointGroup,
  PerformanceStat,
} from '~/types/marketing';

export function useMarketingPage() {
  const installCommand = 'brew install vinayakkulkarni/tap/tileserver-rs';
  const { copied, copy } = useClipboard({ copiedDuring: 2000 });

  async function copyToClipboard() {
    await copy(installCommand);
  }

  const features: Feature[] = [
    {
      icon: Zap,
      title: 'Blazing Fast',
      description:
        'Built in Rust for maximum performance. Serve tiles with sub-millisecond latency.',
      category: 'Rendering',
    },
    {
      icon: Globe,
      title: 'PMTiles & MBTiles',
      description:
        'Native support for modern PMTiles and classic MBTiles tile archives with MVT and MLT format support.',
      category: 'Data formats',
    },
    {
      icon: RefreshCw,
      title: 'MLT Transcoding',
      description:
        'On-the-fly MLT↔MVT transcoding. Serve next-gen MapLibre Tiles from existing MVT sources — up to 6x smaller tiles.',
      category: 'Rendering',
    },
    {
      icon: Database,
      title: 'PostgreSQL / PostGIS',
      description:
        'Serve vector tiles directly from PostGIS tables with optimized spatial queries.',
      category: 'Data formats',
    },
    {
      icon: MapIcon,
      title: 'Cloud Optimized GeoTIFF',
      description:
        'Serve raster tiles from COG files with on-the-fly reprojection and colormap support.',
      category: 'Data formats',
    },
    {
      icon: HardDrive,
      title: 'PostgreSQL Out-DB Rasters',
      description:
        'Dynamic VRT/COG tile serving via PostGIS functions with query-based filtering.',
      category: 'Data formats',
    },
    {
      icon: Layers,
      title: 'Vector & Raster',
      description:
        'Serve vector tiles directly or render them to raster on-the-fly.',
      category: 'Rendering',
    },
    {
      icon: Image,
      title: 'Static Images',
      description:
        'Generate static map images like Mapbox Static API with native MapLibre rendering.',
      category: 'Rendering',
    },
    {
      icon: Server,
      title: 'Self-Hosted',
      description:
        'Run on your own infrastructure. No vendor lock-in, no API keys required.',
      category: 'Deployment',
    },
    {
      icon: Sparkles,
      title: 'Zero-Config Startup',
      description:
        'Point at a directory or file and start serving. Auto-detects PMTiles, MBTiles, styles, and fonts.',
      category: 'Developer experience',
    },
    {
      icon: Terminal,
      title: 'Hot-Reload',
      description:
        'Reload configuration without downtime via SIGHUP or admin API. Zero-request-drop with ArcSwap.',
      category: 'Developer experience',
    },
    {
      icon: Upload,
      title: 'Drag & Drop',
      description:
        'Drop GeoJSON, KML, GPX, CSV, Shapefile, PMTiles, MBTiles, or COG files onto the map for instant visualization with auto-styling.',
      category: 'Developer experience',
    },
    {
      icon: Cloud,
      title: 'One-Click Deploy',
      description:
        'Deploy to Railway, Render, DigitalOcean, or Fly.io in minutes. Sample data auto-downloads on first start.',
      category: 'Deployment',
    },
    {
      icon: BotMessageSquare,
      title: 'Browser-Local AI',
      description:
        'Talk to your maps with a built-in LLM. Runs entirely in your browser via WebGPU — no API keys, no cloud, no token costs.',
      category: 'Intelligence',
    },
    {
      icon: FileSpreadsheet,
      title: 'GeoParquet Source',
      description:
        'Serve vector tiles directly from GeoParquet files — no preprocessing. Point at Overture Maps data and get instant tiles.',
      category: 'Data formats',
    },
    {
      icon: Braces,
      title: 'DuckDB Backend',
      description:
        'Generate tiles from SQL queries against embedded DuckDB. Query GeoParquet, CSV, or any format DuckDB reads — PostGIS power, zero ops.',
      category: 'Data formats',
    },
    {
      icon: BarChart3,
      title: 'OpenAPI & Analytics',
      description:
        'Interactive OpenAPI spec with Scalar UI. Generate SDKs, import into Postman, or track usage with built-in telemetry.',
      category: 'Developer experience',
    },
    {
      icon: Timer,
      title: 'Configurable Caching',
      description:
        'Per-source cache control headers with configurable max-age, stale-while-revalidate, and CDN-friendly strategies for optimal tile delivery.',
      category: 'Developer experience',
    },
    {
      icon: FileCode2,
      title: 'OGC API Features',
      description:
        'Serve PostGIS tables as OGC-compliant feature collections. CRS reprojection, CQL2 filtering with SQL-injection defence, full CRUD write operations, and JSON Schema introspection. QGIS, ArcGIS, and FME ready.',
      category: 'Data formats',
    },
    {
      icon: Satellite,
      title: 'STAC Catalog Sources',
      description:
        'Point at any STAC API (Element84, Planetary Computer, USGS) and serve COGs as tiles. Static discovery, dynamic per-tile bbox search, and multi-asset mosaic compositing — all without preprocessing.',
      category: 'Data formats',
    },
    {
      icon: Plug,
      title: 'MCP Server',
      description:
        'First Rust tile server with built-in Model Context Protocol. Wire your deployment into Claude Desktop, Cursor, or claude.ai — 11 tools spanning introspection, rendering, CQL2 feature queries, and STAC search.',
      category: 'Intelligence',
    },
  ];

  const CATEGORY_ORDER: FeatureCategory[] = [
    'Data formats',
    'Rendering',
    'Developer experience',
    'Deployment',
    'Intelligence',
  ];

  const featuresByCategory: FeatureGroup[] = CATEGORY_ORDER.map((cat) => ({
    category: cat,
    features: features.filter((f) => f.category === cat),
  }));

  const comparisonColumns: ComparisonColumn[] = [
    'tileserver-rs',
    'martin',
    'tileserver-gl',
    'pg_tileserv',
    'titiler',
  ];

  const comparisonRows: ComparisonRow[] = [
    {
      feature: 'Vector tiles (MVT/PBF)',
      values: {
        'tileserver-rs': '✓',
        martin: '✓',
        'tileserver-gl': '✓',
        pg_tileserv: '✓',
        titiler: '✓',
      },
    },
    {
      feature: 'MLT transcoding',
      values: {
        'tileserver-rs': '✓',
        martin: '✗',
        'tileserver-gl': '✗',
        pg_tileserv: '✗',
        titiler: '✗',
      },
    },
    {
      feature: 'Raster tiles from COG',
      values: {
        'tileserver-rs': '✓',
        martin: '✗',
        'tileserver-gl': '✗',
        pg_tileserv: '✗',
        titiler: '✓',
      },
    },
    {
      feature: 'Server-side MapLibre rendering',
      values: {
        'tileserver-rs': '✓',
        martin: '✗',
        'tileserver-gl': '✓',
        pg_tileserv: '✗',
        titiler: '✗',
      },
    },
    {
      feature: 'PostGIS-backed tiles',
      values: {
        'tileserver-rs': '✓',
        martin: '✓',
        'tileserver-gl': '✗',
        pg_tileserv: '✓',
        titiler: '✗',
      },
    },
    {
      feature: 'OGC API Features (CRUD)',
      values: {
        'tileserver-rs': '✓',
        martin: '✗',
        'tileserver-gl': '✗',
        pg_tileserv: '◐',
        titiler: '✗',
      },
    },
    {
      feature: 'STAC catalog sources',
      values: {
        'tileserver-rs': '✓',
        martin: '✗',
        'tileserver-gl': '✗',
        pg_tileserv: '✗',
        titiler: '✓',
      },
    },
    {
      feature: 'Browser-local AI',
      values: {
        'tileserver-rs': '✓',
        martin: '✗',
        'tileserver-gl': '✗',
        pg_tileserv: '✗',
        titiler: '✗',
      },
    },
    {
      feature: 'MCP server (stdio + HTTP)',
      values: {
        'tileserver-rs': '✓',
        martin: '✗',
        'tileserver-gl': '✗',
        pg_tileserv: '✗',
        titiler: '✗',
      },
    },
    {
      feature: 'Single binary (no Python deps)',
      values: {
        'tileserver-rs': '✓',
        martin: '✓',
        'tileserver-gl': '✗',
        pg_tileserv: '✓',
        titiler: '✗',
      },
    },
    {
      feature: 'OpenAPI spec built-in',
      values: {
        'tileserver-rs': '✓',
        martin: '✗',
        'tileserver-gl': '✗',
        pg_tileserv: '✗',
        titiler: '✗',
      },
    },
    {
      feature: 'Zero-config startup',
      values: {
        'tileserver-rs': '✓',
        martin: '◐',
        'tileserver-gl': '◐',
        pg_tileserv: '◐',
        titiler: '✗',
      },
    },
    {
      feature: 'Drag-and-drop file serving',
      values: {
        'tileserver-rs': '✓',
        martin: '✗',
        'tileserver-gl': '✗',
        pg_tileserv: '✗',
        titiler: '✗',
      },
    },
  ];

  const performanceStats: PerformanceStat[] = [
    {
      icon: Timer,
      value: 3,
      label: 'ms to fill 4×4 viewport',
      detail: '4× faster than tileserver-gl',
    },
    {
      icon: Zap,
      value: 358,
      label: 'Grids/sec at z14',
      detail: 'PMTiles 4×4 viewport load',
    },
    {
      icon: Database,
      value: 12682,
      label: 'PostGIS req/sec at z14',
      detail: '7.45ms avg, 410 MB/s throughput',
    },
    {
      icon: Layers,
      value: 89,
      label: 'MB/s throughput',
      detail: 'Steady across the city zoom band',
      prefix: '~',
    },
  ];

  const aiBenefits: AiBenefit[] = [
    {
      icon: ShieldCheck,
      title: 'Zero Data Leakage',
      description:
        'Every query stays in your browser. Map data, questions, and results never touch a third-party server.',
    },
    {
      icon: Cpu,
      title: 'WebGPU Powered',
      description:
        'Runs on your GPU via WebLLM. No server to maintain, no API keys to rotate, no monthly AI bills.',
    },
    {
      icon: MessageSquare,
      title: 'Natural Language Control',
      description:
        'Fly to locations, filter layers, query features, and restyle your map — all by chatting in plain English.',
    },
    {
      icon: Puzzle,
      title: '10+ Map Tools',
      description:
        'fly_to, fit_bounds, set_layer_paint, query_rendered_features, spatial_query, and more — all callable by the LLM.',
    },
  ];

  const aiChatExample: AiChatMessage[] = [
    { role: 'user' as const, text: 'Show me all buildings in downtown Tokyo' },
    {
      role: 'assistant' as const,
      text: 'Flying to [139.7670, 35.6812] at zoom 15\n✅ Found 847 building features in viewport\nHighlighting buildings with height > 100m...',
    },
    { role: 'user' as const, text: 'Make parks greener and more visible' },
    {
      role: 'assistant' as const,
      text: '✅ Set park fill-color to #22c55e, opacity to 0.7',
    },
  ];

  const apiEndpoints: ApiEndpointGroup[] = [
    {
      title: 'Vector Tiles',
      endpoints: [
        { method: 'GET', path: '/data/{source}/{z}/{x}/{y}.pbf' },
        { method: 'GET', path: '/data/{source}/{z}/{x}/{y}.mlt' },
        { method: 'GET', path: '/data/{source}.json' },
      ],
    },
    {
      title: 'COG/Raster Tiles',
      endpoints: [
        { method: 'GET', path: '/data/{cog}/{z}/{x}/{y}.png' },
        { method: 'GET', path: '/data/{cog}/{z}/{x}/{y}.webp' },
      ],
    },
    {
      title: 'PostgreSQL Out-DB Rasters',
      endpoints: [
        {
          method: 'GET',
          path: '/data/{source}/{z}/{x}/{y}.png?satellite=...',
        },
      ],
    },
    {
      title: 'Style Rendering',
      endpoints: [
        { method: 'GET', path: '/styles/{style}/{z}/{x}/{y}.png' },
        { method: 'GET', path: '/styles/{style}/{z}/{x}/{y}@2x.png' },
      ],
    },
    {
      title: 'Static Images',
      endpoints: [
        {
          method: 'GET',
          path: '/styles/{style}/static/{lon},{lat},{zoom}/{w}x{h}.png',
        },
      ],
    },
    {
      title: 'Health & Admin',
      endpoints: [
        { method: 'GET', path: '/health' },
        { method: 'GET', path: '/ping' },
        { method: 'POST', path: '/__admin/reload' },
      ],
    },
  ];

  return {
    copied,
    installCommand,
    copyToClipboard,
    features,
    featuresByCategory,
    comparisonColumns,
    comparisonRows,
    aiBenefits,
    aiChatExample,
    performanceStats,
    apiEndpoints,
  };
}
