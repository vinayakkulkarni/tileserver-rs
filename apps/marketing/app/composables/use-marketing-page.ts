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
} from 'lucide-vue-next';
import type {
  Feature,
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
    },
    {
      icon: Globe,
      title: 'PMTiles & MBTiles',
      description:
        'Native support for modern PMTiles and classic MBTiles tile archives with MVT and MLT format support.',
    },
    {
      icon: RefreshCw,
      title: 'MLT Transcoding',
      description:
        'On-the-fly MLT↔MVT transcoding. Serve next-gen MapLibre Tiles from existing MVT sources — up to 6x smaller tiles.',
    },
    {
      icon: Database,
      title: 'PostgreSQL / PostGIS',
      description:
        'Serve vector tiles directly from PostGIS tables with optimized spatial queries.',
    },
    {
      icon: MapIcon,
      title: 'Cloud Optimized GeoTIFF',
      description:
        'Serve raster tiles from COG files with on-the-fly reprojection and colormap support.',
    },
    {
      icon: HardDrive,
      title: 'PostgreSQL Out-DB Rasters',
      description:
        'Dynamic VRT/COG tile serving via PostGIS functions with query-based filtering.',
    },
    {
      icon: Layers,
      title: 'Vector & Raster',
      description:
        'Serve vector tiles directly or render them to raster on-the-fly.',
    },
    {
      icon: Image,
      title: 'Static Images',
      description:
        'Generate static map images like Mapbox Static API with native MapLibre rendering.',
    },
    {
      icon: Server,
      title: 'Self-Hosted',
      description:
        'Run on your own infrastructure. No vendor lock-in, no API keys required.',
    },
    {
      icon: Sparkles,
      title: 'Zero-Config Startup',
      description:
        'Point at a directory or file and start serving. Auto-detects PMTiles, MBTiles, styles, and fonts.',
    },
    {
      icon: Terminal,
      title: 'Hot-Reload',
      description:
        'Reload configuration without downtime via SIGHUP or admin API. Zero-request-drop with ArcSwap.',
    },
    {
      icon: Upload,
      title: 'Drag & Drop',
      description:
        'Drop GeoJSON, KML, GPX, CSV, Shapefile, PMTiles, MBTiles, or COG files onto the map for instant visualization with auto-styling.',
    },
    {
      icon: Cloud,
      title: 'One-Click Deploy',
      description:
        'Deploy to Railway, Render, DigitalOcean, or Fly.io in minutes. Sample data auto-downloads on first start.',
    },
    {
      icon: BotMessageSquare,
      title: 'Browser-Local AI',
      description:
        'Talk to your maps with a built-in LLM. Runs entirely in your browser via WebGPU — no API keys, no cloud, no token costs.',
    },
    {
      icon: FileSpreadsheet,
      title: 'GeoParquet Source',
      description:
        'Serve vector tiles directly from GeoParquet files — no preprocessing. Point at Overture Maps data and get instant tiles.',
    },
    {
      icon: BarChart3,
      title: 'OpenAPI & Analytics',
      description:
        'Interactive OpenAPI spec with Scalar UI. Generate SDKs, import into Postman, or track usage with built-in telemetry.',
    },
  ];

  const performanceStats: PerformanceStat[] = [
    {
      icon: Zap,
      value: 1409,
      label: 'PMTiles req/sec',
      detail: '10% faster than tileserver-gl',
    },
    {
      icon: Database,
      value: 3596,
      label: 'PostGIS req/sec',
      detail: 'Matches martin (PostGIS-bound)',
    },
    {
      icon: BarChart3,
      value: 13144,
      label: 'PostGIS req/sec at z14',
      detail: '7ms avg latency',
    },
    {
      icon: Layers,
      value: 93,
      label: 'MB/s throughput',
      detail: 'Consistent across zoom levels',
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
    aiBenefits,
    aiChatExample,
    performanceStats,
    apiEndpoints,
  };
}
