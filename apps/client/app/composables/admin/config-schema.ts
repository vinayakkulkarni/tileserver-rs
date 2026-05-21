/**
 * Hand-authored catalog of every tileserver-rs configuration key.
 *
 * Source of truth (in priority order):
 *  1. `crates/tileserver-rs/src/config.rs` — the actual Rust structs.
 *  2. `apps/docs/content/1.getting-started/3.config.md` — user-facing docs.
 *
 * Both files MUST be kept in sync with any change here. When adding a field
 * to a Rust config struct, add the same entry to this file AND to the docs
 * page in the same PR.
 *
 * Type definitions for the schema live in `~/types/admin-config-schema`;
 * this file only holds the catalog data.
 */

import type { ConfigSectionSchema } from '~/types/admin-config-schema';

export const CONFIG_SCHEMA: readonly ConfigSectionSchema[] = [
  {
    header: '[server]',
    blurb: 'Public HTTP listener, CORS, uploads, and admin bind.',
    fields: [
      { key: 'host', type: 'string', default: '"0.0.0.0"', description: 'Bind address for the public tile server.' },
      { key: 'port', type: 'u16', default: '8080', description: 'Bind port for the public tile server.' },
      { key: 'cors_origins', type: 'string[]', default: '["*"]', description: 'CORS allow-list for tile + style endpoints. Lock down in production.' },
      { key: 'admin_bind', type: 'string', default: '"127.0.0.1:0"', description: 'Separate bind for admin endpoints (/__admin/*). ":0" disables. Use "127.0.0.1:8081" to enable.' },
      { key: 'public_url', type: 'string', default: null, description: 'Public URL embedded in TileJSON. Falls back to the bind address.', optional: true },
      { key: 'upload_dir', type: 'path', default: null, description: 'Directory for drag-and-drop uploads. Defaults to system tmp dir.', optional: true },
      { key: 'upload_max_size_mb', type: 'u32', default: '500', description: 'Maximum per-file upload size in megabytes.' },
    ],
  },
  {
    header: '[render]',
    blurb: 'Native MapLibre raster renderer pool (used for COG, static images, raster tiles).',
    fields: [
      { key: 'pool_size', type: 'usize', default: '4', description: 'Concurrent renderer worker threads.' },
      { key: 'render_timeout_secs', type: 'u64', default: '30', description: 'Per-request render timeout. Requests exceeding this are dropped.' },
    ],
  },
  {
    header: '[cache]',
    blurb: 'Global in-process tile cache (moka backend).',
    fields: [
      { key: 'enabled', type: 'bool', default: 'false', description: 'Enable the global tile cache.' },
      { key: 'max_size_mb', type: 'u64', default: '512', description: 'Maximum cache size in megabytes.' },
      { key: 'ttl_seconds', type: 'u64', default: '3600', description: 'Time-to-live for cache entries in seconds.' },
    ],
  },
  {
    header: '[raster]',
    blurb: 'Raster output defaults (resampler + tile size).',
    featureGate: 'raster',
    fields: [
      { key: 'default_resampling', type: 'enum', default: '"bilinear"', description: 'Default GDAL resampler. Per-source `resampling` overrides this.', enumValues: ['nearest', 'bilinear', 'cubic', 'cubicspline', 'lanczos', 'average', 'mode'] },
      { key: 'tile_size', type: 'u32', default: '256', description: 'Output tile size in pixels (set to 512 for retina-native tiles).' },
    ],
  },
  {
    header: '[telemetry]',
    blurb: 'OpenTelemetry tracing + Prometheus metrics export.',
    fields: [
      { key: 'enabled', type: 'bool', default: 'false', description: 'Enable OTLP trace export.' },
      { key: 'endpoint', type: 'string', default: '"http://localhost:4317"', description: 'OTLP gRPC endpoint.' },
      { key: 'service_name', type: 'string', default: '"tileserver-rs"', description: 'service.name resource attribute.' },
      { key: 'sample_rate', type: 'f64', default: '1.0', description: 'Sampling rate (0.0-1.0). 1.0 = export every span.' },
      { key: 'metrics_enabled', type: 'bool', default: 'true', description: 'Enable OTLP metrics export (requires enabled = true).' },
      { key: 'metrics_export_interval_secs', type: 'u64', default: '30', description: 'OTLP metrics push interval.' },
      { key: 'prometheus_bind', type: 'string', default: null, description: 'Bind for standalone Prometheus /metrics listener (independent of OTLP). E.g. "127.0.0.1:9100".', optional: true },
      { key: 'prometheus_path', type: 'string', default: '"/metrics"', description: 'HTTP path for the Prometheus exposition endpoint.' },
      { key: 'metrics_label_cardinality', type: 'enum', default: '"strict"', description: 'Strict = bucketed zoom, no tile coords. Standard = alias of strict. Verbose = full zoom 0..22.', enumValues: ['strict', 'standard', 'verbose'] },
    ],
  },
  {
    header: '[[sources]]',
    blurb: 'Tile source. Repeat for each source. Type-specific fields appear based on `type`.',
    isArray: true,
    fields: [
      { key: 'id', type: 'string', default: null, description: 'Unique identifier (becomes /data/<id>/* route). Required.' },
      { key: 'type', type: 'enum', default: null, description: 'Source backend. Required.', enumValues: ['pmtiles', 'mbtiles', 'postgres', 'cog', 'vrt', 'geoparquet', 'duckdb', 'stac'] },
      { key: 'path', type: 'string', default: null, description: 'File path, HTTP(S) URL, or cloud URL (s3://, gs://, az://). Required.' },
      { key: 'name', type: 'string', default: null, description: 'Display name shown in the viewer.', optional: true },
      { key: 'attribution', type: 'string', default: null, description: 'Attribution text included in TileJSON.', optional: true },
      { key: 'description', type: 'string', default: null, description: 'Free-form description.', optional: true },
      { key: 'minzoom', type: 'u8', default: null, description: 'Override the minzoom from source metadata.', optional: true },
      { key: 'maxzoom', type: 'u8', default: null, description: 'Override the maxzoom from source metadata.', optional: true },
      { key: 'serve_as', type: 'enum', default: null, description: 'Transcode on the fly. E.g. serve_as = "mlt" on a PBF source emits MLT.', optional: true, enumValues: ['pbf', 'mvt', 'mlt', 'png', 'jpeg', 'webp'] },
      { key: 'options', type: 'table', default: null, description: 'Key-value map forwarded to cloud backends (S3 credentials, GCS keys, etc.).', optional: true },
      { key: 'resampling', type: 'enum', default: null, description: 'Per-source resampler override (raster only). Defaults to [raster].default_resampling.', optional: true, enumValues: ['nearest', 'bilinear', 'cubic', 'cubicspline', 'lanczos', 'average', 'mode'] },
      { key: 'colormap', type: 'table', default: null, description: 'Inline colormap (raster only). See docs for full schema.', optional: true },
      { key: 'collection', type: 'string', default: null, description: 'STAC collection ID (STAC sources only).', optional: true },
      { key: 'asset_role', type: 'string', default: '"visual"', description: 'STAC asset role to render (STAC sources only).', optional: true },
      { key: 'dynamic', type: 'bool', default: 'false', description: 'Enable on-demand STAC search per tile (STAC sources only).', optional: true },
      { key: 'max_items', type: 'usize', default: '100', description: 'Max items per tile when dynamic = true (STAC sources only).', optional: true },
      { key: 'stac_bbox', type: 'f64[4]', default: null, description: 'Override the bbox passed to STAC search.', optional: true },
      { key: 'pixel_selection', type: 'enum', default: '"first"', description: 'STAC mosaic strategy.', optional: true, enumValues: ['first', 'highest', 'lowest', 'mean', 'median', 'stdev', 'count', 'lowestcloudcover'] },
    ],
  },
  {
    header: '[[styles]]',
    blurb: 'MapLibre style. Repeat for each style.',
    isArray: true,
    fields: [
      { key: 'id', type: 'string', default: null, description: 'Style identifier (becomes /styles/<id>/* route). Required.' },
      { key: 'path', type: 'string', default: null, description: 'Path to a MapLibre style JSON file. Required.' },
      { key: 'name', type: 'string', default: null, description: 'Display name shown in the viewer.', optional: true },
      { key: 'attribution', type: 'string', default: null, description: 'Attribution text.', optional: true },
    ],
  },
  {
    header: '[postgres]',
    blurb: 'PostgreSQL connection pool + per-source registries.',
    featureGate: 'postgres',
    fields: [
      { key: 'connection_string', type: 'string', default: null, description: 'Postgres connection URI. Required.' },
      { key: 'pool_size', type: 'usize', default: '10', description: 'Connections in the pool.' },
      { key: 'pool_wait_timeout_ms', type: 'u64', default: '5000', description: 'Max ms to wait for a free connection.' },
      { key: 'pool_create_timeout_ms', type: 'u64', default: '5000', description: 'Max ms to wait when opening a new connection.' },
      { key: 'pool_recycle_timeout_ms', type: 'u64', default: '5000', description: 'Max ms to wait recycling a stale connection.' },
      { key: 'pool_pre_warm', type: 'bool', default: 'false', description: 'Open all pool connections at startup.' },
      { key: 'ssl_cert', type: 'path', default: null, description: 'mTLS client certificate (PEM).', optional: true },
      { key: 'ssl_key', type: 'path', default: null, description: 'mTLS client key (PEM).', optional: true },
      { key: 'ssl_root_cert', type: 'path', default: null, description: 'Root CA for verify-full SSL.', optional: true },
    ],
  },
  {
    header: '[postgres.cache]',
    blurb: 'Per-Postgres MVT tile cache. Independent of the global [cache].',
    featureGate: 'postgres',
    fields: [
      { key: 'size_mb', type: 'u64', default: '256', description: 'Max cache size for Postgres MVT tiles.' },
      { key: 'ttl_seconds', type: 'u64', default: '300', description: 'TTL per entry.' },
    ],
  },
  {
    header: '[[postgres.functions]]',
    blurb: 'Postgres function source. Function must take (z, x, y) and return bytea.',
    featureGate: 'postgres',
    isArray: true,
    fields: [
      { key: 'id', type: 'string', default: null, description: 'Source ID. Required.' },
      { key: 'schema', type: 'string', default: null, description: 'Postgres schema. Required.' },
      { key: 'function', type: 'string', default: null, description: 'Function name. Required.' },
      { key: 'name', type: 'string', default: null, description: 'Display name.', optional: true },
      { key: 'attribution', type: 'string', default: null, description: 'Attribution text.', optional: true },
      { key: 'description', type: 'string', default: null, description: 'Free-form description.', optional: true },
      { key: 'minzoom', type: 'u8', default: '0', description: 'Minimum zoom.' },
      { key: 'maxzoom', type: 'u8', default: '22', description: 'Maximum zoom.' },
      { key: 'bounds', type: 'f64[4]', default: null, description: 'Geographic bounds [west, south, east, north].', optional: true },
    ],
  },
  {
    header: '[[postgres.tables]]',
    blurb: 'Postgres table source. tileserver-rs builds MVT tiles from the table on the fly.',
    featureGate: 'postgres',
    isArray: true,
    fields: [
      { key: 'id', type: 'string', default: null, description: 'Source ID. Required.' },
      { key: 'schema', type: 'string', default: null, description: 'Postgres schema. Required.' },
      { key: 'table', type: 'string', default: null, description: 'Table name. Required.' },
      { key: 'geometry_column', type: 'string', default: '"geom"', description: 'Geometry column.', optional: true },
      { key: 'id_column', type: 'string', default: null, description: 'Optional integer ID column carried into MVT features.', optional: true },
      { key: 'properties', type: 'string[]', default: null, description: 'Column allow-list for MVT properties. Defaults to all columns.', optional: true },
      { key: 'name', type: 'string', default: null, description: 'Display name.', optional: true },
      { key: 'attribution', type: 'string', default: null, description: 'Attribution text.', optional: true },
      { key: 'description', type: 'string', default: null, description: 'Free-form description.', optional: true },
      { key: 'minzoom', type: 'u8', default: '0', description: 'Minimum zoom.' },
      { key: 'maxzoom', type: 'u8', default: '22', description: 'Maximum zoom.' },
      { key: 'bounds', type: 'f64[4]', default: null, description: 'Geographic bounds [west, south, east, north].', optional: true },
      { key: 'extent', type: 'u32', default: '4096', description: 'MVT extent (4096 = standard).' },
      { key: 'buffer', type: 'u32', default: '64', description: 'Per-tile geometry buffer in pixels (auto-set to 0 for POINT/MULTIPOINT).' },
      { key: 'max_features', type: 'u32', default: null, description: 'Hard cap on emitted features per tile.', optional: true },
      { key: 'writable', type: 'bool', default: 'false', description: 'Enable OGC API Features POST/PATCH/DELETE on this table.' },
    ],
  },
  {
    header: '[mcp]',
    blurb: 'Model Context Protocol server (AI assistant integration).',
    featureGate: 'mcp',
    fields: [
      { key: 'enabled', type: 'bool', default: 'false', description: 'Mount the /mcp Streamable HTTP service.' },
      { key: 'auth_token', type: 'string', default: null, description: 'Static bearer token. Mutually exclusive with oauth.enabled = true.', optional: true },
      { key: 'cors_origins', type: 'string[]', default: '["*"]', description: 'CORS allow-list for /mcp. Lock down in production.' },
    ],
  },
  {
    header: '[mcp.oauth]',
    blurb: 'OAuth 2.0 + RFC 7591 DCR for the MCP HTTP transport (claude.ai Custom Connectors).',
    featureGate: 'mcp',
    fields: [
      { key: 'enabled', type: 'bool', default: 'false', description: 'Enable RFC 7591 DCR + JWT RS256 OAuth flow.' },
      { key: 'issuer_url', type: 'string', default: null, description: 'Required when enabled = true. Must match the public URL.', optional: true },
      { key: 'signing_key_path', type: 'path', default: null, description: 'Required when enabled = true. RSA PKCS#8 PEM private key.', optional: true },
      { key: 'token_ttl_secs', type: 'u64', default: '3600', description: 'Access-token TTL (clamped to 86400).' },
    ],
  },
];
