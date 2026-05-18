use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[cfg(feature = "raster")]
use gdal::raster::ResampleAlg;

/// Main configuration for the tileserver
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub telemetry: TelemetryConfig,
    #[serde(default)]
    pub sources: Vec<SourceConfig>,
    #[serde(default)]
    pub styles: Vec<StyleConfig>,
    /// Path to fonts directory containing PBF glyph files
    #[serde(default)]
    pub fonts: Option<PathBuf>,
    /// Path to static files directory for /files/{filename} endpoint
    #[serde(default)]
    pub files: Option<PathBuf>,
    /// PostgreSQL configuration (optional, requires `postgres` feature)
    #[serde(default)]
    #[cfg(feature = "postgres")]
    pub postgres: Option<PostgresConfig>,
    #[serde(default)]
    #[cfg(feature = "raster")]
    pub raster: RasterConfig,
    /// Native renderer pool configuration
    #[serde(default)]
    pub render: RenderPoolConfig,
    /// Global in-process tile cache
    #[serde(default)]
    pub cache: CacheConfig,
    /// Model Context Protocol (MCP) server configuration.
    ///
    /// Only present when the binary is compiled with `--features mcp`.
    /// When enabled, the HTTP server mounts a Streamable HTTP MCP service at
    /// `/mcp` for AI assistants (Cursor, Claude Desktop via mcp-remote, MCP
    /// Inspector). Stdio mode is reached via the `mcp-stdio` subcommand.
    #[serde(default)]
    #[cfg(feature = "mcp")]
    pub mcp: McpConfig,
}

/// MCP server configuration block.
///
/// ```toml
/// [mcp]
/// enabled = true
/// auth_token = "secret"          # optional bearer token; omit to disable auth
/// cors_origins = ["*"]           # wildcard by default; lock down in production
/// ```
#[cfg(feature = "mcp")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct McpConfig {
    /// Whether to mount the `/mcp` Streamable HTTP service on the main listener.
    pub enabled: bool,
    /// Optional bearer token. When `Some`, requests to `/mcp` must include
    /// `Authorization: Bearer <token>`. Stdio transport never reads this.
    pub auth_token: Option<String>,
    /// Origins allowed by the `/mcp` CORS layer.
    ///
    /// Defaults to `["*"]` (wildcard, preserving pre-1.0 behavior). Set
    /// explicit origins such as `["https://claude.ai", "https://app.cursor.com"]`
    /// to restrict access. An empty list falls back to wildcard with a warning
    /// log; individual invalid origin strings are skipped with a `warn!` log.
    #[serde(default = "default_mcp_cors_origins")]
    pub cors_origins: Vec<String>,
}

/// Default CORS allow-list for the `/mcp` endpoint — wildcard.
///
/// Wrapped in a function rather than a const so `#[serde(default = "…")]`
/// can reference it and the manual [`Default`] impl can reuse it.
#[cfg(feature = "mcp")]
fn default_mcp_cors_origins() -> Vec<String> {
    vec!["*".to_string()]
}

#[cfg(feature = "mcp")]
impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auth_token: None,
            cors_origins: default_mcp_cors_origins(),
        }
    }
}

/// Native renderer pool configuration for server-side raster tile generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPoolConfig {
    /// Number of concurrent renderer worker threads (default: 4)
    #[serde(default = "default_render_pool_size")]
    pub pool_size: usize,
    /// Render timeout in seconds — requests exceeding this are dropped (default: 30)
    #[serde(default = "default_render_timeout_secs")]
    pub render_timeout_secs: u64,
}

fn default_render_pool_size() -> usize {
    4
}

fn default_render_timeout_secs() -> u64 {
    30
}

impl Default for RenderPoolConfig {
    fn default() -> Self {
        Self {
            pool_size: default_render_pool_size(),
            render_timeout_secs: default_render_timeout_secs(),
        }
    }
}

/// Global in-process tile cache configuration.
///
/// ```toml
/// [cache]
/// enabled = true
/// max_size_mb = 512
/// ttl_seconds = 3600
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable the tile cache (default: `false`)
    #[serde(default)]
    pub enabled: bool,
    /// Maximum cache size in megabytes (default: 512)
    #[serde(default = "default_global_cache_max_size_mb")]
    pub max_size_mb: u64,
    /// Time-to-live for cache entries in seconds (default: 3600)
    #[serde(default = "default_global_cache_ttl_seconds")]
    pub ttl_seconds: u64,
}

fn default_global_cache_max_size_mb() -> u64 {
    512
}
fn default_global_cache_ttl_seconds() -> u64 {
    3600
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_size_mb: default_global_cache_max_size_mb(),
            ttl_seconds: default_global_cache_ttl_seconds(),
        }
    }
}

#[cfg(feature = "raster")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RasterConfig {
    #[serde(default)]
    pub default_resampling: ResamplingMethod,
    #[serde(default = "default_tile_size")]
    pub tile_size: u32,
}

#[cfg(feature = "raster")]
fn default_tile_size() -> u32 {
    256
}

#[cfg(feature = "raster")]
impl Default for RasterConfig {
    fn default() -> Self {
        Self {
            default_resampling: ResamplingMethod::default(),
            tile_size: default_tile_size(),
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub cors_origins: Vec<String>,
    /// Optional admin bind address for the reload endpoint.
    /// Use `"127.0.0.1:0"` (default) to disable.
    #[serde(default = "default_admin_bind")]
    pub admin_bind: String,
    /// Public URL for tile URLs in TileJSON responses.
    /// Use this when running behind a reverse proxy or Docker port mapping.
    /// Example: "http://localhost:4000" when Docker maps 4000:8080
    /// If not set, auto-generated from host:port
    #[serde(default)]
    pub public_url: Option<String>,
    /// Directory for uploaded files (temporary sources).
    /// Defaults to system temp dir + "tileserver-uploads" if not set.
    #[serde(default)]
    pub upload_dir: Option<PathBuf>,
    /// Maximum upload file size in megabytes (default: 500 MB)
    #[serde(default = "default_upload_max_size_mb")]
    pub upload_max_size_mb: u32,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_admin_bind() -> String {
    "127.0.0.1:0".to_string()
}

fn default_upload_max_size_mb() -> u32 {
    500
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            cors_origins: vec!["*".to_string()],
            admin_bind: default_admin_bind(),
            public_url: None,
            upload_dir: None,
            upload_max_size_mb: default_upload_max_size_mb(),
        }
    }
}

/// OpenTelemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Enable OpenTelemetry tracing
    #[serde(default)]
    pub enabled: bool,
    /// OTLP endpoint (e.g., "http://localhost:4317")
    #[serde(default = "default_otlp_endpoint")]
    pub endpoint: String,
    /// Service name for traces
    #[serde(default = "default_service_name")]
    pub service_name: String,
    /// Sampling rate (0.0 to 1.0, where 1.0 = 100% of traces)
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
    /// Enable OpenTelemetry metrics (requires `enabled = true`)
    #[serde(default = "default_metrics_enabled")]
    pub metrics_enabled: bool,
    /// Metrics export interval in seconds
    #[serde(default = "default_metrics_export_interval_secs")]
    pub metrics_export_interval_secs: u64,
    /// Bind address for the standalone Prometheus `/metrics` listener
    /// (e.g., `"127.0.0.1:9100"`). When `None` (the default), the
    /// listener task is never spawned and there is zero runtime cost.
    /// Independent of `enabled` — Prometheus pull works without OTLP
    /// push and vice versa.
    #[serde(default)]
    pub prometheus_bind: Option<String>,
    /// HTTP path for the Prometheus exposition endpoint (default
    /// `/metrics`). Only relevant when `prometheus_bind` is set.
    #[serde(default = "default_prometheus_path")]
    pub prometheus_path: String,
    /// Cardinality strategy for metric labels:
    /// - `Strict` (default): zoom collapsed to `low|mid|high` buckets,
    ///   tile coordinates dropped — bounded combinations safe for
    ///   long-term Prometheus retention.
    /// - `Standard`: same as strict for now (reserved for future
    ///   intermediate strategies).
    /// - `Verbose`: zoom passed through 0..=22, useful for debugging
    ///   short-window investigations but can blow up cardinality if
    ///   left enabled in production.
    #[serde(default)]
    pub metrics_label_cardinality: MetricsLabelCardinality,
}

/// Cardinality strategy for metric labels emitted via the
/// Prometheus `/metrics` endpoint and the OTLP push pipeline.
///
/// See [`TelemetryConfig::metrics_label_cardinality`] for behavior
/// of each variant. Default is [`MetricsLabelCardinality::Strict`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetricsLabelCardinality {
    /// Bounded label set safe for production Prometheus scrape.
    #[default]
    Strict,
    /// Reserved for future intermediate cardinality (currently aliases `Strict`).
    Standard,
    /// Pass-through high-cardinality labels (debug only).
    Verbose,
}

fn default_otlp_endpoint() -> String {
    "http://localhost:4317".to_string()
}

fn default_service_name() -> String {
    "tileserver-rs".to_string()
}

fn default_sample_rate() -> f64 {
    1.0
}

fn default_metrics_enabled() -> bool {
    true
}

fn default_metrics_export_interval_secs() -> u64 {
    60
}

fn default_prometheus_path() -> String {
    "/metrics".to_string()
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: default_otlp_endpoint(),
            service_name: default_service_name(),
            sample_rate: default_sample_rate(),
            metrics_enabled: default_metrics_enabled(),
            metrics_export_interval_secs: default_metrics_export_interval_secs(),
            prometheus_bind: None,
            prometheus_path: default_prometheus_path(),
            metrics_label_cardinality: MetricsLabelCardinality::Strict,
        }
    }
}

/// Configuration for a tile source (PMTiles or MBTiles)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Unique identifier for this source
    pub id: String,
    /// Type of source: "pmtiles" or "mbtiles"
    #[serde(rename = "type")]
    pub source_type: SourceType,
    /// Path to the file (local path, HTTP URL, or S3 URL)
    pub path: String,
    /// Optional display name
    pub name: Option<String>,
    /// Optional attribution text
    pub attribution: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Resampling algorithm for rescaling raster tiles to the target
    /// 256×256 Web Mercator grid.  `None` falls back to the global
    /// [`RasterConfig::default_resampling`] (defaults to `Bilinear`).
    /// Matters when a COG's native resolution differs from the served
    /// tile size — e.g., Sentinel-2 10 m bands served as 256×256 Web
    /// Mercator tiles need downsampling, and `Bilinear` or `Cubic`
    /// give visibly better results than the `Nearest` default for
    /// continuous imagery.
    #[serde(default)]
    pub resampling: Option<ResamplingMethod>,
    #[serde(default)]
    pub layer_name: Option<String>,
    #[serde(default)]
    pub geometry_column: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub minzoom: Option<u8>,
    #[serde(default)]
    pub maxzoom: Option<u8>,
    /// Optional format transcoding: serve tiles as this format instead of native format.
    /// E.g., set `serve_as = "mlt"` on a PBF source to transcode MVT→MLT on the fly.
    #[serde(default)]
    pub serve_as: Option<crate::sources::TileFormat>,
    #[cfg(feature = "raster")]
    #[serde(default)]
    pub colormap: Option<ColorMapConfig>,
    /// Provider-specific options for cloud object storage (S3/Azure/GCS).
    #[serde(default)]
    pub options: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub collection: Option<String>,
    #[serde(default = "default_stac_asset_role")]
    pub asset_role: String,
    #[serde(default)]
    pub dynamic: bool,
    #[serde(default = "default_stac_max_items")]
    pub max_items: usize,
    /// Optional WGS-84 bounding box `[west, south, east, north]` used to scope
    /// Phase 1 static discovery AND to override the merged item bounds exposed
    /// to clients. Essential when a global STAC collection would otherwise
    /// anchor discovery on whichever items the API happens to rank first.
    #[serde(default)]
    pub stac_bbox: Option<[f64; 4]>,
    /// STAC mosaic pixel-selection method. Defaults to [`PixelSelectionMethod::First`]
    /// which preserves pre-4.0 behaviour (first asset wins where opaque).
    #[serde(default)]
    pub pixel_selection: PixelSelectionMethod,
}

/// Pixel-selection strategy for STAC mosaic compositing.
///
/// Matches rio-tiler's `rio_tiler.mosaic.methods` set so operators who
/// already know titiler feel at home.  When the default `first` is
/// used, cold-path latency is minimal: the method short-circuits on
/// the first fully-opaque pixel and stops fetching downstream assets.
///
/// Short-circuit capability is declared in [`PixelSelectionMethod::can_short_circuit`]
/// and is honoured by the mosaic pipeline.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PixelSelectionMethod {
    /// First valid (non-masked) pixel wins. Short-circuits once the
    /// canvas is fully opaque; this is the lowest-latency strategy.
    #[default]
    First,
    /// Per-pixel maximum across all input bands.  Useful for highlighting
    /// bright features (e.g., snow, water glare).
    Highest,
    /// Per-pixel minimum.  Useful for deep-shadow emphasis.
    Lowest,
    /// Per-pixel arithmetic mean across all valid inputs.
    Mean,
    /// Per-pixel median across all valid inputs.  More robust to outliers
    /// than [`PixelSelectionMethod::Mean`] at the cost of extra allocations.
    Median,
    /// Per-pixel standard deviation across all valid inputs.  Useful for
    /// change-detection visualisations.
    Stdev,
    /// Per-pixel count of valid contributions.  Primarily a QA/debug
    /// visualisation; encodes the count in the red channel.
    Count,
    /// Select the asset with the lowest `eo:cloud_cover` value from the
    /// STAC metadata, then use its pixels wherever they are valid.
    /// Falls back to [`PixelSelectionMethod::First`] ordering for assets
    /// where the field is missing.
    LowestCloudCover,
}

impl PixelSelectionMethod {
    /// Returns true when this method can stop feeding inputs once the
    /// canvas is fully opaque — enabling an early-exit optimisation in
    /// the mosaic loop.  Only [`Self::First`] and [`Self::LowestCloudCover`]
    /// are short-circuit-safe; the statistical methods need every input.
    #[must_use]
    pub const fn can_short_circuit(self) -> bool {
        matches!(self, Self::First | Self::LowestCloudCover)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    PMTiles,
    MBTiles,
    #[cfg(feature = "postgres")]
    Postgres,
    #[cfg(feature = "raster")]
    Cog,
    #[cfg(feature = "raster")]
    Vrt,
    #[cfg(feature = "geoparquet")]
    GeoParquet,
    #[cfg(feature = "duckdb")]
    DuckDB,
    #[cfg(feature = "stac")]
    Stac,
}

fn default_stac_asset_role() -> String {
    "visual".to_string()
}

fn default_stac_max_items() -> usize {
    100
}

/// GDAL-compatible resampling algorithm for rescaling a source raster
/// onto the output tile grid.
///
/// Each variant maps 1-to-1 onto [`gdal::raster::ResampleAlg`] and
/// matches rio-tiler / rasterio names when serialised, so titiler and
/// tileserver-rs configs are interchangeable.
///
/// # When to change the default
///
/// `Bilinear` is a good general-purpose default for continuous imagery
/// (satellite visual bands, DEMs).  Use `Nearest` for classified/
/// categorical rasters where intermediate values are meaningless
/// (land-cover codes, nominal classes).  `Cubic` / `Lanczos` give
/// sharper downsampling at the cost of CPU time.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ResamplingMethod {
    /// Pick the closest source pixel. Preserves discrete values;
    /// correct choice for classified rasters and mandatory for any
    /// colourmap-indexed band.
    Nearest,
    /// Linear interpolation across 2×2 source pixels.  Good default
    /// for continuous imagery; balances quality and speed.
    #[default]
    Bilinear,
    /// Cubic interpolation across 4×4 source pixels.  Sharper than
    /// `Bilinear` at moderately higher CPU cost.
    Cubic,
    /// Spline-smoothed cubic.  Softer edges than `Cubic`; useful for
    /// continuous elevation data.
    CubicSpline,
    /// Lanczos windowed sinc.  Best-quality general-purpose resampler
    /// at the highest CPU cost.
    Lanczos,
    /// Pixel-value averaging.  Appropriate for heavy downsampling
    /// where aliasing would otherwise appear.
    Average,
    /// Most-frequent source value.  Use for categorical data when
    /// `Nearest` sampling alone would look noisy at low zooms.
    Mode,
}

impl std::fmt::Display for ResamplingMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResamplingMethod::Nearest => write!(f, "nearest"),
            ResamplingMethod::Bilinear => write!(f, "bilinear"),
            ResamplingMethod::Cubic => write!(f, "cubic"),
            ResamplingMethod::CubicSpline => write!(f, "cubicspline"),
            ResamplingMethod::Lanczos => write!(f, "lanczos"),
            ResamplingMethod::Average => write!(f, "average"),
            ResamplingMethod::Mode => write!(f, "mode"),
        }
    }
}

impl std::str::FromStr for ResamplingMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "nearest" => Ok(ResamplingMethod::Nearest),
            "bilinear" => Ok(ResamplingMethod::Bilinear),
            "cubic" => Ok(ResamplingMethod::Cubic),
            "cubicspline" => Ok(ResamplingMethod::CubicSpline),
            "lanczos" => Ok(ResamplingMethod::Lanczos),
            "average" => Ok(ResamplingMethod::Average),
            "mode" => Ok(ResamplingMethod::Mode),
            _ => Err(format!("unknown resampling method: {s}")),
        }
    }
}

#[cfg(feature = "raster")]
impl From<ResamplingMethod> for ResampleAlg {
    fn from(method: ResamplingMethod) -> Self {
        match method {
            ResamplingMethod::Nearest => ResampleAlg::NearestNeighbour,
            ResamplingMethod::Bilinear => ResampleAlg::Bilinear,
            ResamplingMethod::Cubic => ResampleAlg::Cubic,
            ResamplingMethod::CubicSpline => ResampleAlg::CubicSpline,
            ResamplingMethod::Lanczos => ResampleAlg::Lanczos,
            ResamplingMethod::Average => ResampleAlg::Average,
            ResamplingMethod::Mode => ResampleAlg::Mode,
        }
    }
}

#[cfg(feature = "raster")]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ColorMapType {
    #[default]
    Discrete,
    Continuous,
}

#[cfg(feature = "raster")]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RescaleMode {
    #[default]
    Static,
    Dynamic,
    /// No rescaling - use raw pixel values directly for colormap lookup.
    /// Ideal for categorical/classified rasters (land cover, crop types, etc.)
    /// where pixel values represent discrete classes.
    None,
}

#[cfg(feature = "raster")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorMapEntry {
    pub value: f64,
    pub color: String,
}

#[cfg(feature = "raster")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorMapConfig {
    #[serde(default)]
    pub map_type: ColorMapType,
    #[serde(default)]
    pub rescale_mode: RescaleMode,
    pub entries: Vec<ColorMapEntry>,
    #[serde(default)]
    pub nodata_color: Option<String>,
}

#[cfg(feature = "raster")]
impl ColorMapConfig {
    #[must_use]
    pub fn parse_color(hex: &str) -> Option<[u8; 4]> {
        let hex = hex.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some([r, g, b, 255])
        } else if hex.len() == 8 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some([r, g, b, a])
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_color(&self, value: f64) -> [u8; 4] {
        if self.entries.is_empty() {
            return [0, 0, 0, 0];
        }

        match self.map_type {
            ColorMapType::Discrete => {
                for entry in &self.entries {
                    if (entry.value - value).abs() < 0.5 {
                        return Self::parse_color(&entry.color).unwrap_or([0, 0, 0, 0]);
                    }
                }
                self.nodata_color
                    .as_ref()
                    .and_then(|c| Self::parse_color(c))
                    .unwrap_or([0, 0, 0, 0])
            }
            ColorMapType::Continuous => {
                let mut sorted: Vec<_> = self.entries.iter().collect();
                sorted.sort_by(|a, b| {
                    a.value
                        .partial_cmp(&b.value)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                if value <= sorted[0].value {
                    return Self::parse_color(&sorted[0].color).unwrap_or([0, 0, 0, 0]);
                }
                if value >= sorted[sorted.len() - 1].value {
                    return Self::parse_color(&sorted[sorted.len() - 1].color)
                        .unwrap_or([0, 0, 0, 0]);
                }

                for pair in sorted.windows(2) {
                    let low = &pair[0];
                    let high = &pair[1];
                    if value >= low.value && value <= high.value {
                        let t = (value - low.value) / (high.value - low.value);
                        let c1 = Self::parse_color(&low.color).unwrap_or([0, 0, 0, 0]);
                        let c2 = Self::parse_color(&high.color).unwrap_or([0, 0, 0, 0]);
                        return [
                            (c1[0] as f64 + (c2[0] as f64 - c1[0] as f64) * t) as u8,
                            (c1[1] as f64 + (c2[1] as f64 - c1[1] as f64) * t) as u8,
                            (c1[2] as f64 + (c2[2] as f64 - c1[2] as f64) * t) as u8,
                            (c1[3] as f64 + (c2[3] as f64 - c1[3] as f64) * t) as u8,
                        ];
                    }
                }
                [0, 0, 0, 0]
            }
        }
    }
}

/// PostgreSQL connection configuration
#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Database connection string (e.g., "postgresql://user:pass@host:5432/db")
    pub connection_string: String,
    /// Maximum number of connections in the pool (default: 20)
    #[serde(default = "default_pool_size")]
    pub pool_size: usize,
    /// Timeout waiting for a connection from the pool in milliseconds (default: 30000)
    #[serde(default = "default_pool_wait_timeout_ms")]
    pub pool_wait_timeout_ms: u64,
    /// Timeout for creating a new connection in milliseconds (default: 10000)
    #[serde(default = "default_pool_create_timeout_ms")]
    pub pool_create_timeout_ms: u64,
    /// Timeout for recycling a connection in milliseconds (default: 5000)
    #[serde(default = "default_pool_recycle_timeout_ms")]
    pub pool_recycle_timeout_ms: u64,
    /// Pre-warm all connections at startup (default: true)
    #[serde(default = "default_pool_pre_warm")]
    pub pool_pre_warm: bool,
    /// SSL certificate file path (optional, same as PGSSLCERT)
    pub ssl_cert: Option<PathBuf>,
    /// SSL key file path (optional, same as PGSSLKEY)
    pub ssl_key: Option<PathBuf>,
    /// SSL root certificate file path (optional, same as PGSSLROOTCERT)
    pub ssl_root_cert: Option<PathBuf>,
    /// Function sources to publish
    #[serde(default)]
    pub functions: Vec<PostgresFunctionConfig>,
    /// Table sources to publish (generates optimized SQL with spatial filtering)
    #[serde(default)]
    pub tables: Vec<PostgresTableConfig>,
    /// Tile cache configuration (optional, disabled by default)
    #[serde(default)]
    pub cache: Option<PostgresCacheConfig>,
    /// Out-of-database raster sources (VRT/COG files referenced from PostgreSQL)
    #[cfg(feature = "raster")]
    #[serde(default)]
    pub outdb_rasters: Vec<PostgresOutDbRasterConfig>,
}

/// Tile cache configuration for PostgreSQL sources
#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresCacheConfig {
    /// Maximum cache size in megabytes (default: 256)
    #[serde(default = "default_cache_size_mb")]
    pub size_mb: u64,
    /// Time-to-live for cache entries in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_cache_ttl_seconds")]
    pub ttl_seconds: u64,
}

#[cfg(feature = "postgres")]
fn default_cache_size_mb() -> u64 {
    256
}

#[cfg(feature = "postgres")]
fn default_cache_ttl_seconds() -> u64 {
    3600
}

#[cfg(feature = "postgres")]
fn default_pool_size() -> usize {
    20
}

#[cfg(feature = "postgres")]
fn default_pool_wait_timeout_ms() -> u64 {
    30000
}

#[cfg(feature = "postgres")]
fn default_pool_create_timeout_ms() -> u64 {
    10000
}

#[cfg(feature = "postgres")]
fn default_pool_recycle_timeout_ms() -> u64 {
    5000
}

#[cfg(feature = "postgres")]
fn default_pool_pre_warm() -> bool {
    true
}

/// PostgreSQL function source configuration
#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresFunctionConfig {
    /// Unique identifier for this source
    pub id: String,
    /// Schema name (default: public)
    #[serde(default = "default_schema")]
    pub schema: String,
    /// Function name (required)
    pub function: String,
    /// Optional display name
    pub name: Option<String>,
    /// Optional attribution text
    pub attribution: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Minimum zoom level (default: 0)
    #[serde(default)]
    pub minzoom: u8,
    /// Maximum zoom level (default: 22)
    #[serde(default = "default_maxzoom")]
    pub maxzoom: u8,
    /// Bounds [west, south, east, north] in WGS84
    pub bounds: Option<[f64; 4]>,
}

#[cfg(feature = "postgres")]
fn default_schema() -> String {
    "public".to_string()
}

#[cfg(feature = "postgres")]
fn default_maxzoom() -> u8 {
    22
}

#[cfg(feature = "postgres")]
fn default_extent() -> u32 {
    4096
}

#[cfg(feature = "postgres")]
fn default_buffer() -> u32 {
    64
}

/// PostgreSQL table source configuration
#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresTableConfig {
    /// Unique identifier for this source
    pub id: String,
    /// Schema name (default: public)
    #[serde(default = "default_schema")]
    pub schema: String,
    /// Table name (required)
    pub table: String,
    /// Geometry column name (default: auto-detect)
    pub geometry_column: Option<String>,
    /// ID column name for feature IDs (optional)
    pub id_column: Option<String>,
    /// Columns to include in tile properties (default: all non-geometry columns)
    pub properties: Option<Vec<String>>,
    /// Optional display name
    pub name: Option<String>,
    /// Optional attribution text
    pub attribution: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Minimum zoom level (default: 0)
    #[serde(default)]
    pub minzoom: u8,
    /// Maximum zoom level (default: 22)
    #[serde(default = "default_maxzoom")]
    pub maxzoom: u8,
    /// Bounds [west, south, east, north] in WGS84 (default: auto-detect from data)
    pub bounds: Option<[f64; 4]>,
    /// MVT extent (default: 4096)
    #[serde(default = "default_extent")]
    pub extent: u32,
    /// Buffer around tiles in pixels (default: 64)
    #[serde(default = "default_buffer")]
    pub buffer: u32,
    /// Maximum features per tile (default: unlimited)
    pub max_features: Option<u32>,
    /// Enable OGC API Features Part 4 transactions (POST/PUT/PATCH/DELETE).
    ///
    /// Defaults to `false` — mutation endpoints return `405 Method Not Allowed`
    /// unless this is explicitly set. Enable per table on trusted deployments
    /// only; there is no built-in auth.
    #[serde(default)]
    pub writable: bool,
}

#[cfg(all(feature = "postgres", feature = "raster"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresOutDbRasterConfig {
    pub id: String,
    #[serde(default = "default_schema")]
    pub schema: String,
    pub function: Option<String>,
    pub name: Option<String>,
    pub attribution: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub minzoom: u8,
    #[serde(default = "default_maxzoom")]
    pub maxzoom: u8,
    pub bounds: Option<[f64; 4]>,
    #[serde(default)]
    pub resampling: Option<ResamplingMethod>,
    #[serde(default)]
    pub colormap: Option<ColorMapConfig>,
}

/// Configuration for a map style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleConfig {
    /// Unique identifier for this style
    pub id: String,
    /// Path to the style.json file
    pub path: PathBuf,
    /// Optional display name
    pub name: Option<String>,
}

/// Configuration with source metadata and content hash.
pub struct ConfigLoadMetadata {
    pub config: Config,
    pub content_hash: String,
}

impl Config {
    fn hash_content(content: &str) -> String {
        use std::fmt::Write;
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let digest = hasher.finalize();
        let mut hex = String::with_capacity(64);
        for b in digest {
            write!(hex, "{b:02x}").expect("write to String never fails");
        }
        hex
    }

    fn substitute_env_vars(content: &str) -> String {
        shellexpand::env_with_context_no_errors(content, |var| std::env::var(var).ok()).to_string()
    }

    fn from_file_with_metadata(path: &PathBuf) -> anyhow::Result<ConfigLoadMetadata> {
        let content = std::fs::read_to_string(path)?;
        let content = Self::substitute_env_vars(&content);
        let config: Config = toml::from_str(&content)?;
        Ok(ConfigLoadMetadata {
            config,
            content_hash: Self::hash_content(&content),
        })
    }

    /// Load configuration and return metadata including the content hash.
    pub fn load_with_metadata(config_path: Option<PathBuf>) -> anyhow::Result<ConfigLoadMetadata> {
        if let Some(path) = config_path
            && path.exists()
        {
            return Self::from_file_with_metadata(&path);
        }

        let default_paths = vec![
            PathBuf::from("config.toml"),
            PathBuf::from("/etc/tileserver-rs/config.toml"),
        ];

        for path in default_paths {
            if path.exists() {
                return Self::from_file_with_metadata(&path);
            }
        }

        let config = Config::default();
        let content = toml::to_string(&config).unwrap_or_default();
        Ok(ConfigLoadMetadata {
            config,
            content_hash: Self::hash_content(&content),
        })
    }

    /// Load configuration from environment or file.
    pub fn load(config_path: Option<PathBuf>) -> anyhow::Result<Self> {
        Ok(Self::load_with_metadata(config_path)?.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
            [server]
            host = "127.0.0.1"
            port = 3000

            [[sources]]
            id = "osm"
            type = "pmtiles"
            path = "/data/osm.pmtiles"
            name = "OpenStreetMap"

            [[styles]]
            id = "bright"
            path = "/data/styles/bright/style.json"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].id, "osm");
        assert_eq!(config.sources[0].source_type, SourceType::PMTiles);
    }

    #[test]
    fn test_source_type_serialization() {
        assert_eq!(
            serde_json::to_string(&SourceType::PMTiles).unwrap(),
            "\"pmtiles\""
        );
        assert_eq!(
            serde_json::to_string(&SourceType::MBTiles).unwrap(),
            "\"mbtiles\""
        );
    }

    #[test]
    fn test_env_var_substitution_basic() {
        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::set_var("TEST_VAR_1", "hello") };
        let result = Config::substitute_env_vars("value is ${TEST_VAR_1}");
        assert_eq!(result, "value is hello");
        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::remove_var("TEST_VAR_1") };
    }

    #[test]
    fn test_env_var_substitution_with_default() {
        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::remove_var("NONEXISTENT_VAR") };
        let result = Config::substitute_env_vars("value is ${NONEXISTENT_VAR:-fallback}");
        assert_eq!(result, "value is fallback");
    }

    #[test]
    fn test_env_var_substitution_set_var_ignores_default() {
        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::set_var("TEST_VAR_2", "actual") };
        let result = Config::substitute_env_vars("value is ${TEST_VAR_2:-default}");
        assert_eq!(result, "value is actual");
        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::remove_var("TEST_VAR_2") };
    }

    #[test]
    fn test_env_var_substitution_empty_string_keeps_empty() {
        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::set_var("TEST_VAR_3", "") };
        let result = Config::substitute_env_vars("value is ${TEST_VAR_3:-default}");
        assert_eq!(result, "value is ");
        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::remove_var("TEST_VAR_3") };
    }

    #[test]
    fn test_env_var_substitution_multiple() {
        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::set_var("TEST_HOST", "localhost") };
        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::set_var("TEST_PORT", "5432") };
        let result = Config::substitute_env_vars("postgresql://${TEST_HOST}:${TEST_PORT}/db");
        assert_eq!(result, "postgresql://localhost:5432/db");
        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::remove_var("TEST_HOST") };
        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::remove_var("TEST_PORT") };
    }

    #[test]
    fn test_env_var_substitution_postgres_config() {
        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::set_var("DATABASE_URL", "postgresql://user:pass@db:5432/mydb") };

        let toml = r#"
            [server]
            host = "0.0.0.0"
            port = 3000
        "#;

        let substituted = Config::substitute_env_vars(toml);
        assert!(!substituted.contains("${DATABASE_URL}"));

        let toml_with_env = r#"connection_string = "${DATABASE_URL}""#;
        let substituted = Config::substitute_env_vars(toml_with_env);
        assert_eq!(
            substituted,
            r#"connection_string = "postgresql://user:pass@db:5432/mydb""#
        );

        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::remove_var("DATABASE_URL") };
    }

    #[test]
    fn test_metrics_label_cardinality_default_is_strict() {
        assert_eq!(
            MetricsLabelCardinality::default(),
            MetricsLabelCardinality::Strict
        );
    }

    #[test]
    fn test_metrics_label_cardinality_serde_round_trip_all_variants() {
        for (s, variant) in [
            ("strict", MetricsLabelCardinality::Strict),
            ("standard", MetricsLabelCardinality::Standard),
            ("verbose", MetricsLabelCardinality::Verbose),
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, format!("\"{}\"", s));
            let parsed: MetricsLabelCardinality = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn test_telemetry_config_prometheus_fields_parse() {
        let toml = r#"
            [telemetry]
            prometheus_bind = "127.0.0.1:9100"
            prometheus_path = "/metrics"
            metrics_label_cardinality = "verbose"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(
            config.telemetry.prometheus_bind,
            Some("127.0.0.1:9100".to_string())
        );
        assert_eq!(config.telemetry.prometheus_path, "/metrics");
        assert_eq!(
            config.telemetry.metrics_label_cardinality,
            MetricsLabelCardinality::Verbose
        );
    }

    #[test]
    fn test_telemetry_config_prometheus_bind_default_is_none() {
        let config = Config::default();
        assert!(config.telemetry.prometheus_bind.is_none());
        assert_eq!(config.telemetry.prometheus_path, "/metrics");
        assert_eq!(
            config.telemetry.metrics_label_cardinality,
            MetricsLabelCardinality::Strict
        );
    }

    // === Edge-branch tests: defaults, round-trips, load_with_metadata ===

    #[test]
    fn test_config_default_has_empty_sources_and_styles() {
        let config = Config::default();
        assert!(config.sources.is_empty());
        assert!(config.styles.is_empty());
        assert!(config.fonts.is_none());
        assert!(config.files.is_none());
    }

    #[test]
    fn test_server_config_default_values() {
        let server = ServerConfig::default();
        assert_eq!(server.host, "0.0.0.0");
        assert_eq!(server.port, 8080);
        assert_eq!(server.cors_origins, vec!["*".to_string()]);
        assert_eq!(server.admin_bind, "127.0.0.1:0");
        assert!(server.public_url.is_none());
        assert!(server.upload_dir.is_none());
        assert_eq!(server.upload_max_size_mb, 500);
    }

    #[test]
    fn test_cache_config_default_is_disabled() {
        let cache = CacheConfig::default();
        assert!(!cache.enabled);
        assert_eq!(cache.max_size_mb, 512);
        assert_eq!(cache.ttl_seconds, 3600);
    }

    #[test]
    fn test_cache_config_default_via_config() {
        let config = Config::default();
        assert!(!config.cache.enabled);
        assert_eq!(config.cache.max_size_mb, 512);
    }

    #[test]
    fn test_render_pool_config_default_values() {
        let render = RenderPoolConfig::default();
        assert_eq!(render.pool_size, 4);
        assert_eq!(render.render_timeout_secs, 30);
    }

    #[test]
    fn test_config_toml_round_trip_empty() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).expect("serialize default config to TOML");
        let reparsed: Config = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(reparsed.sources.len(), config.sources.len());
        assert_eq!(reparsed.styles.len(), config.styles.len());
        assert_eq!(reparsed.server.host, config.server.host);
        assert_eq!(reparsed.server.port, config.server.port);
        assert_eq!(reparsed.cache.enabled, config.cache.enabled);
    }

    #[test]
    fn test_source_config_pmtiles_round_trip() {
        let toml_str = r#"
            [[sources]]
            id = "test-pmtiles"
            type = "pmtiles"
            path = "/data/test.pmtiles"
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse pmtiles source config");
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].id, "test-pmtiles");
        assert_eq!(config.sources[0].source_type, SourceType::PMTiles);
        assert_eq!(config.sources[0].path, "/data/test.pmtiles");
        assert!(config.sources[0].name.is_none());
    }

    #[test]
    fn test_source_config_mbtiles_round_trip() {
        let toml_str = r#"
            [[sources]]
            id = "test-mbtiles"
            type = "mbtiles"
            path = "/data/test.mbtiles"
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse mbtiles source config");
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].source_type, SourceType::MBTiles);
    }

    #[test]
    fn test_source_config_with_optional_fields() {
        let toml_str = r#"
            [[sources]]
            id = "full"
            type = "pmtiles"
            path = "/data/full.pmtiles"
            name = "Full Source"
            attribution = "© Provider"
            description = "Test description"
            minzoom = 0
            maxzoom = 14
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse full source config");
        let src = &config.sources[0];
        assert_eq!(src.name.as_deref(), Some("Full Source"));
        assert_eq!(src.attribution.as_deref(), Some("© Provider"));
        assert_eq!(src.description.as_deref(), Some("Test description"));
        assert_eq!(src.minzoom, Some(0));
        assert_eq!(src.maxzoom, Some(14));
    }

    #[test]
    fn test_style_config_with_name_round_trip() {
        let toml_str = r#"
            [[styles]]
            id = "my-style"
            path = "/styles/my-style/style.json"
            name = "My Style Name"
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse style config with name");
        assert_eq!(config.styles.len(), 1);
        assert_eq!(config.styles[0].id, "my-style");
        assert_eq!(config.styles[0].name.as_deref(), Some("My Style Name"));
    }

    #[test]
    fn test_style_config_without_name_round_trip() {
        let toml_str = r#"
            [[styles]]
            id = "minimal-style"
            path = "/styles/minimal/style.json"
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse style config without name");
        assert_eq!(config.styles.len(), 1);
        assert_eq!(config.styles[0].id, "minimal-style");
        assert!(config.styles[0].name.is_none());
    }

    #[test]
    fn test_cache_config_enabled_round_trip() {
        let toml_str = r#"
            [cache]
            enabled = true
            max_size_mb = 1024
            ttl_seconds = 7200
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse cache config");
        assert!(config.cache.enabled);
        assert_eq!(config.cache.max_size_mb, 1024);
        assert_eq!(config.cache.ttl_seconds, 7200);
    }

    #[test]
    fn test_cache_config_partial_uses_defaults() {
        let toml_str = r#"
            [cache]
            enabled = true
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse partial cache config");
        assert!(config.cache.enabled);
        assert_eq!(config.cache.max_size_mb, 512);
        assert_eq!(config.cache.ttl_seconds, 3600);
    }

    #[test]
    fn test_render_pool_config_round_trip() {
        let toml_str = r#"
            [render]
            pool_size = 8
            render_timeout_secs = 60
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse render pool config");
        assert_eq!(config.render.pool_size, 8);
        assert_eq!(config.render.render_timeout_secs, 60);
    }

    #[test]
    fn test_render_pool_partial_uses_defaults() {
        let toml_str = r#"
            [render]
            pool_size = 2
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse partial render config");
        assert_eq!(config.render.pool_size, 2);
        assert_eq!(config.render.render_timeout_secs, 30);
    }

    #[test]
    fn test_server_config_custom_port_and_host() {
        let toml_str = r#"
            [server]
            host = "127.0.0.1"
            port = 9090
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse server config with port");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.server.host, "127.0.0.1");
    }

    #[test]
    fn test_server_config_public_url_override() {
        let toml_str = r#"
            [server]
            public_url = "https://tiles.example.com"
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse server public_url");
        assert_eq!(
            config.server.public_url.as_deref(),
            Some("https://tiles.example.com")
        );
    }

    #[test]
    fn test_server_config_cors_origins_round_trip() {
        let toml_str = r#"
            [server]
            cors_origins = ["https://a.com", "https://b.com"]
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse cors_origins");
        assert_eq!(config.server.cors_origins.len(), 2);
        assert_eq!(config.server.cors_origins[0], "https://a.com");
    }

    #[test]
    fn test_server_config_upload_settings() {
        let toml_str = r#"
            [server]
            upload_max_size_mb = 100
            upload_dir = "/var/uploads"
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse upload settings");
        assert_eq!(config.server.upload_max_size_mb, 100);
        assert_eq!(
            config
                .server
                .upload_dir
                .as_ref()
                .map(|p| p.to_str().unwrap()),
            Some("/var/uploads")
        );
    }

    #[test]
    fn test_config_fonts_and_files_paths() {
        let toml_str = r#"
            fonts = "/data/fonts"
            files = "/data/static"
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse fonts/files paths");
        assert_eq!(
            config.fonts.as_ref().and_then(|p| p.to_str()),
            Some("/data/fonts")
        );
        assert_eq!(
            config.files.as_ref().and_then(|p| p.to_str()),
            Some("/data/static")
        );
    }

    #[test]
    fn test_config_content_hash_is_deterministic() {
        let content = "[server]\nport = 8080\n";
        let hash1 = Config::hash_content(content);
        let hash2 = Config::hash_content(content);
        assert_eq!(hash1, hash2);
        // SHA-256 produces 32 bytes = 64 hex characters
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_config_content_hash_differs_for_different_content() {
        let hash1 = Config::hash_content("[server]\nport = 8080\n");
        let hash2 = Config::hash_content("[server]\nport = 9090\n");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_config_load_with_metadata_from_tempfile() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("create tempfile");
        write!(
            tmp,
            r#"
[server]
host = "127.0.0.1"
port = 8181

[[sources]]
id = "local"
type = "pmtiles"
path = "/tmp/test.pmtiles"
"#
        )
        .expect("write tempfile");
        tmp.flush().expect("flush tempfile");

        let loaded = Config::load_with_metadata(Some(tmp.path().to_path_buf()))
            .expect("load_with_metadata succeeds");
        assert_eq!(loaded.config.server.port, 8181);
        assert_eq!(loaded.config.server.host, "127.0.0.1");
        assert_eq!(loaded.config.sources.len(), 1);
        assert_eq!(loaded.config.sources[0].id, "local");
        assert_eq!(loaded.config.sources[0].source_type, SourceType::PMTiles);
        assert_eq!(loaded.content_hash.len(), 64);
    }

    #[test]
    fn test_config_load_with_metadata_missing_path_falls_back_to_default() {
        let nonexistent = PathBuf::from("/tmp/definitely-not-a-real-config-9999.toml");
        let result = Config::load_with_metadata(Some(nonexistent));
        assert!(result.is_ok());
        let loaded = result.expect("load returns ok");
        assert_eq!(loaded.content_hash.len(), 64);
    }

    #[test]
    fn test_config_load_wrapper_returns_config_only() {
        let nonexistent = PathBuf::from("/tmp/definitely-not-a-real-config-9998.toml");
        let result = Config::load(Some(nonexistent));
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_load_with_metadata_invalid_toml_errors() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("create tempfile");
        write!(tmp, "this is = not = valid = toml\n[[[broken").expect("write invalid TOML");
        tmp.flush().expect("flush tempfile");

        let result = Config::load_with_metadata(Some(tmp.path().to_path_buf()));
        assert!(result.is_err(), "invalid TOML should error");
    }

    #[test]
    fn test_config_load_with_metadata_applies_env_substitution() {
        use std::io::Write;
        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::set_var("TEST_ENV_SUB_PORT", "7777") };
        let mut tmp = tempfile::NamedTempFile::new().expect("create tempfile");
        write!(
            tmp,
            r#"
[server]
port = ${{TEST_ENV_SUB_PORT}}
"#
        )
        .expect("write tempfile with env var");
        tmp.flush().expect("flush tempfile");

        let loaded = Config::load_with_metadata(Some(tmp.path().to_path_buf()))
            .expect("load_with_metadata succeeds");
        assert_eq!(loaded.config.server.port, 7777);

        // SAFETY: test-only; no concurrent threads access env vars in this test
        unsafe { std::env::remove_var("TEST_ENV_SUB_PORT") };
    }

    #[test]
    fn test_source_type_non_exhaustive_marker_does_not_break_serde() {
        let s = serde_json::to_string(&SourceType::PMTiles).expect("serialize");
        let back: SourceType = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back, SourceType::PMTiles);
    }

    #[test]
    fn test_telemetry_default_metrics_path() {
        let config = Config::default();
        assert_eq!(config.telemetry.prometheus_path, "/metrics");
    }

    #[test]
    fn test_metrics_label_cardinality_serde_standard_variant() {
        let s = serde_json::to_string(&MetricsLabelCardinality::Standard).expect("serialize");
        assert_eq!(s, "\"standard\"");
        let back: MetricsLabelCardinality =
            serde_json::from_str("\"standard\"").expect("deserialize");
        assert_eq!(back, MetricsLabelCardinality::Standard);
    }

    #[cfg(feature = "postgres")]
    mod postgres_tests {
        use super::*;

        #[test]
        fn test_parse_postgres_config() {
            let toml = r#"
                [server]
                host = "127.0.0.1"
                port = 3000

                [postgres]
                connection_string = "postgresql://user:pass@localhost:5432/mydb"
                pool_size = 10

                [[postgres.functions]]
                id = "my_tiles"
                schema = "public"
                function = "tile_function"
                minzoom = 0
                maxzoom = 14
                bounds = [-180.0, -85.0, 180.0, 85.0]

                [[postgres.functions]]
                id = "other_tiles"
                function = "other_function"
                name = "Other Tiles"
                attribution = "© My Company"
            "#;

            let config: Config = toml::from_str(toml).unwrap();

            let pg = config.postgres.expect("postgres config should be present");
            assert_eq!(
                pg.connection_string,
                "postgresql://user:pass@localhost:5432/mydb"
            );
            assert_eq!(pg.pool_size, 10);
            assert_eq!(pg.functions.len(), 2);

            // First function
            let func1 = &pg.functions[0];
            assert_eq!(func1.id, "my_tiles");
            assert_eq!(func1.schema, "public");
            assert_eq!(func1.function, "tile_function");
            assert_eq!(func1.minzoom, 0);
            assert_eq!(func1.maxzoom, 14);
            assert!(func1.bounds.is_some());
            assert_eq!(func1.bounds.unwrap(), [-180.0, -85.0, 180.0, 85.0]);

            // Second function with defaults
            let func2 = &pg.functions[1];
            assert_eq!(func2.id, "other_tiles");
            assert_eq!(func2.schema, "public"); // default
            assert_eq!(func2.function, "other_function");
            assert_eq!(func2.name, Some("Other Tiles".to_string()));
            assert_eq!(func2.attribution, Some("© My Company".to_string()));
            assert_eq!(func2.minzoom, 0); // default
            assert_eq!(func2.maxzoom, 22); // default
            assert!(func2.bounds.is_none());
        }

        #[test]
        fn test_postgres_config_defaults() {
            let toml = r#"
                [postgres]
                connection_string = "postgresql://localhost/db"

                [[postgres.functions]]
                id = "tiles"
                function = "get_tiles"
            "#;

            let config: Config = toml::from_str(toml).unwrap();
            let pg = config.postgres.unwrap();

            assert_eq!(pg.pool_size, 20); // default
            assert!(pg.ssl_cert.is_none());
            assert!(pg.ssl_key.is_none());
            assert!(pg.ssl_root_cert.is_none());

            let func = &pg.functions[0];
            assert_eq!(func.schema, "public"); // default
            assert_eq!(func.minzoom, 0); // default
            assert_eq!(func.maxzoom, 22); // default
        }

        #[test]
        fn test_postgres_function_config_serialization() {
            let func = PostgresFunctionConfig {
                id: "test".to_string(),
                schema: "myschema".to_string(),
                function: "myfunc".to_string(),
                name: Some("Test Function".to_string()),
                attribution: None,
                description: Some("A test function".to_string()),
                minzoom: 0,
                maxzoom: 16,
                bounds: Some([-10.0, -10.0, 10.0, 10.0]),
            };

            let json = serde_json::to_string(&func).unwrap();
            let parsed: PostgresFunctionConfig = serde_json::from_str(&json).unwrap();

            assert_eq!(parsed.id, "test");
            assert_eq!(parsed.schema, "myschema");
            assert_eq!(parsed.function, "myfunc");
            assert_eq!(parsed.name, Some("Test Function".to_string()));
            assert_eq!(parsed.maxzoom, 16);
        }

        #[test]
        fn test_source_type_postgres() {
            assert_eq!(
                serde_json::to_string(&SourceType::Postgres).unwrap(),
                "\"postgres\""
            );

            let parsed: SourceType = serde_json::from_str("\"postgres\"").unwrap();
            assert_eq!(parsed, SourceType::Postgres);
        }

        #[test]
        fn test_parse_postgres_table_config() {
            let toml = r#"
                [postgres]
                connection_string = "postgresql://user:pass@localhost:5432/mydb"

                [[postgres.tables]]
                id = "points"
                table = "my_points"
                geometry_column = "geom"
                id_column = "id"
                properties = ["name", "category"]
                minzoom = 0
                maxzoom = 14
                extent = 4096
                buffer = 64
                max_features = 10000

                [[postgres.tables]]
                id = "polygons"
                schema = "public"
                table = "my_polygons"
            "#;

            let config: Config = toml::from_str(toml).unwrap();
            let pg = config.postgres.expect("postgres config should be present");
            assert_eq!(pg.tables.len(), 2);

            let table1 = &pg.tables[0];
            assert_eq!(table1.id, "points");
            assert_eq!(table1.table, "my_points");
            assert_eq!(table1.geometry_column, Some("geom".to_string()));
            assert_eq!(table1.id_column, Some("id".to_string()));
            assert_eq!(
                table1.properties,
                Some(vec!["name".to_string(), "category".to_string()])
            );
            assert_eq!(table1.extent, 4096);
            assert_eq!(table1.buffer, 64);
            assert_eq!(table1.max_features, Some(10000));

            let table2 = &pg.tables[1];
            assert_eq!(table2.id, "polygons");
            assert_eq!(table2.schema, "public");
            assert_eq!(table2.table, "my_polygons");
            assert_eq!(table2.extent, 4096);
            assert_eq!(table2.buffer, 64);
            assert!(table2.geometry_column.is_none());
            assert!(table2.max_features.is_none());
        }

        #[cfg(feature = "raster")]
        #[test]
        fn test_parse_postgres_outdb_raster_config() {
            let toml = r#"
                [postgres]
                connection_string = "postgresql://user:pass@localhost:5432/gis"

                [[postgres.outdb_rasters]]
                id = "imagery"
                schema = "public"
                function = "get_raster_paths"
                name = "Satellite Imagery"
                minzoom = 0
                maxzoom = 18
                bounds = [-180.0, -85.0, 180.0, 85.0]

                [[postgres.outdb_rasters]]
                id = "dem"
                function = "get_dem_paths"
            "#;

            let config: Config = toml::from_str(toml).unwrap();
            let pg = config.postgres.expect("postgres config should be present");
            assert_eq!(pg.outdb_rasters.len(), 2);

            let outdb1 = &pg.outdb_rasters[0];
            assert_eq!(outdb1.id, "imagery");
            assert_eq!(outdb1.schema, "public");
            assert_eq!(outdb1.function, Some("get_raster_paths".to_string()));
            assert_eq!(outdb1.name, Some("Satellite Imagery".to_string()));
            assert_eq!(outdb1.minzoom, 0);
            assert_eq!(outdb1.maxzoom, 18);
            assert!(outdb1.bounds.is_some());
            assert_eq!(outdb1.bounds.unwrap(), [-180.0, -85.0, 180.0, 85.0]);

            let outdb2 = &pg.outdb_rasters[1];
            assert_eq!(outdb2.id, "dem");
            assert_eq!(outdb2.schema, "public");
            assert_eq!(outdb2.function, Some("get_dem_paths".to_string()));
            assert!(outdb2.name.is_none());
            assert_eq!(outdb2.minzoom, 0);
            assert_eq!(outdb2.maxzoom, 22);
        }

        #[cfg(feature = "raster")]
        #[test]
        fn test_outdb_raster_with_resampling() {
            let toml = r#"
                [postgres]
                connection_string = "postgresql://localhost/db"

                [[postgres.outdb_rasters]]
                id = "elevation"
                function = "get_dem_paths"
                resampling = "bilinear"
            "#;

            let config: Config = toml::from_str(toml).unwrap();
            let pg = config.postgres.unwrap();
            assert_eq!(pg.outdb_rasters.len(), 1);

            let outdb = &pg.outdb_rasters[0];
            assert_eq!(outdb.id, "elevation");
            assert_eq!(
                outdb.resampling,
                Some(crate::config::ResamplingMethod::Bilinear)
            );
        }

        #[cfg(feature = "raster")]
        #[test]
        fn test_outdb_raster_function_defaults_to_id() {
            let toml = r#"
                [postgres]
                connection_string = "postgresql://localhost/db"

                [[postgres.outdb_rasters]]
                id = "imagery"
            "#;

            let config: Config = toml::from_str(toml).unwrap();
            let pg = config.postgres.unwrap();
            let outdb = &pg.outdb_rasters[0];
            assert_eq!(outdb.id, "imagery");
            assert!(outdb.function.is_none());
            assert_eq!(outdb.function.as_ref().unwrap_or(&outdb.id), "imagery");
        }

        #[cfg(feature = "raster")]
        #[test]
        fn test_rescale_mode_none_parsing() {
            let toml = r##"
[postgres]
connection_string = "postgresql://localhost/db"

[[postgres.outdb_rasters]]
id = "landcover"
function = "get_landcover_paths"

[postgres.outdb_rasters.colormap]
map_type = "discrete"
rescale_mode = "none"
nodata_color = "#00000000"
entries = [
    { value = 0.0, color = "#00000000" },
    { value = 1.0, color = "#FD080C" },
    { value = 2.0, color = "#1D90FF" },
    { value = 3.0, color = "#22FDD5" },
]
"##;

            let config: Config = toml::from_str(toml).unwrap();
            let pg = config.postgres.unwrap();
            let outdb = &pg.outdb_rasters[0];
            let colormap = outdb.colormap.as_ref().unwrap();

            assert_eq!(colormap.rescale_mode, RescaleMode::None);
            assert_eq!(colormap.map_type, ColorMapType::Discrete);
            assert_eq!(colormap.entries.len(), 4);
        }

        #[cfg(feature = "raster")]
        #[test]
        fn test_rescale_mode_serialization() {
            assert_eq!(
                serde_json::to_string(&RescaleMode::Static).unwrap(),
                "\"static\""
            );
            assert_eq!(
                serde_json::to_string(&RescaleMode::Dynamic).unwrap(),
                "\"dynamic\""
            );
            assert_eq!(
                serde_json::to_string(&RescaleMode::None).unwrap(),
                "\"none\""
            );

            let parsed: RescaleMode = serde_json::from_str("\"none\"").unwrap();
            assert_eq!(parsed, RescaleMode::None);
        }

        #[cfg(feature = "raster")]
        #[test]
        fn test_discrete_colormap_with_raw_values() {
            let colormap = ColorMapConfig {
                map_type: ColorMapType::Discrete,
                rescale_mode: RescaleMode::None,
                entries: vec![
                    ColorMapEntry {
                        value: 0.0,
                        color: "#00000000".to_string(),
                    },
                    ColorMapEntry {
                        value: 1.0,
                        color: "#FF0000FF".to_string(),
                    },
                    ColorMapEntry {
                        value: 2.0,
                        color: "#00FF00FF".to_string(),
                    },
                    ColorMapEntry {
                        value: 3.0,
                        color: "#0000FFFF".to_string(),
                    },
                ],
                nodata_color: Some("#00000000".to_string()),
            };

            assert_eq!(colormap.get_color(1.0), [255, 0, 0, 255]);
            assert_eq!(colormap.get_color(2.0), [0, 255, 0, 255]);
            assert_eq!(colormap.get_color(3.0), [0, 0, 255, 255]);
            assert_eq!(colormap.get_color(0.0), [0, 0, 0, 0]);

            assert_eq!(colormap.get_color(1.2), [255, 0, 0, 255]);
            assert_eq!(colormap.get_color(0.8), [255, 0, 0, 255]);

            assert_eq!(colormap.get_color(99.0), [0, 0, 0, 0]);
        }

        #[cfg(feature = "raster")]
        #[test]
        fn test_rescale_mode_default_is_static() {
            let mode = RescaleMode::default();
            assert_eq!(mode, RescaleMode::Static);
        }

        #[test]
        fn test_resampling_method_display_round_trip() {
            for method in [
                ResamplingMethod::Nearest,
                ResamplingMethod::Bilinear,
                ResamplingMethod::Cubic,
                ResamplingMethod::CubicSpline,
                ResamplingMethod::Lanczos,
                ResamplingMethod::Average,
                ResamplingMethod::Mode,
            ] {
                let s = method.to_string();
                let parsed: ResamplingMethod = s.parse().unwrap();
                assert_eq!(parsed, method);
            }
        }

        #[test]
        fn test_resampling_method_from_str_case_insensitive() {
            assert_eq!(
                "NEAREST".parse::<ResamplingMethod>().unwrap(),
                ResamplingMethod::Nearest
            );
            assert_eq!(
                "Bilinear".parse::<ResamplingMethod>().unwrap(),
                ResamplingMethod::Bilinear
            );
            assert_eq!(
                "CUBICSPLINE".parse::<ResamplingMethod>().unwrap(),
                ResamplingMethod::CubicSpline
            );
        }

        #[test]
        fn test_resampling_method_from_str_unknown_errors() {
            let err = "garbage".parse::<ResamplingMethod>().unwrap_err();
            assert!(err.contains("unknown"));
            assert!(err.contains("garbage"));
        }

        #[test]
        fn test_resampling_method_default_is_bilinear() {
            assert_eq!(ResamplingMethod::default(), ResamplingMethod::Bilinear);
        }

        #[test]
        fn test_pixel_selection_method_short_circuit_truth_table() {
            assert!(PixelSelectionMethod::First.can_short_circuit());
            assert!(PixelSelectionMethod::LowestCloudCover.can_short_circuit());

            assert!(!PixelSelectionMethod::Highest.can_short_circuit());
            assert!(!PixelSelectionMethod::Lowest.can_short_circuit());
            assert!(!PixelSelectionMethod::Mean.can_short_circuit());
            assert!(!PixelSelectionMethod::Median.can_short_circuit());
            assert!(!PixelSelectionMethod::Stdev.can_short_circuit());
            assert!(!PixelSelectionMethod::Count.can_short_circuit());
        }

        #[test]
        fn test_pixel_selection_method_default_is_first() {
            assert_eq!(PixelSelectionMethod::default(), PixelSelectionMethod::First);
        }

        #[test]
        fn test_pixel_selection_method_serde_round_trip_all_variants() {
            for method in [
                PixelSelectionMethod::First,
                PixelSelectionMethod::Highest,
                PixelSelectionMethod::Lowest,
                PixelSelectionMethod::Mean,
                PixelSelectionMethod::Median,
                PixelSelectionMethod::Stdev,
                PixelSelectionMethod::Count,
                PixelSelectionMethod::LowestCloudCover,
            ] {
                let json = serde_json::to_string(&method).unwrap();
                let parsed: PixelSelectionMethod = serde_json::from_str(&json).unwrap();
                assert_eq!(parsed, method);
            }
        }

        #[test]
        fn test_source_type_pmtiles_mbtiles_serde() {
            assert_eq!(
                serde_json::to_string(&SourceType::PMTiles).unwrap(),
                "\"pmtiles\""
            );
            assert_eq!(
                serde_json::to_string(&SourceType::MBTiles).unwrap(),
                "\"mbtiles\""
            );

            let pmt: SourceType = serde_json::from_str("\"pmtiles\"").unwrap();
            assert_eq!(pmt, SourceType::PMTiles);
            let mbt: SourceType = serde_json::from_str("\"mbtiles\"").unwrap();
            assert_eq!(mbt, SourceType::MBTiles);
        }

        #[cfg(feature = "raster")]
        #[test]
        fn test_source_type_cog_vrt_serde() {
            assert_eq!(serde_json::to_string(&SourceType::Cog).unwrap(), "\"cog\"");
            assert_eq!(serde_json::to_string(&SourceType::Vrt).unwrap(), "\"vrt\"");
        }

        #[cfg(feature = "geoparquet")]
        #[test]
        fn test_source_type_geoparquet_serde() {
            assert_eq!(
                serde_json::to_string(&SourceType::GeoParquet).unwrap(),
                "\"geoparquet\""
            );
            let parsed: SourceType = serde_json::from_str("\"geoparquet\"").unwrap();
            assert_eq!(parsed, SourceType::GeoParquet);
        }

        #[cfg(feature = "duckdb")]
        #[test]
        fn test_source_type_duckdb_serde() {
            assert_eq!(
                serde_json::to_string(&SourceType::DuckDB).unwrap(),
                "\"duckdb\""
            );
            let parsed: SourceType = serde_json::from_str("\"duckdb\"").unwrap();
            assert_eq!(parsed, SourceType::DuckDB);
        }

        #[cfg(feature = "stac")]
        #[test]
        fn test_source_type_stac_serde() {
            assert_eq!(
                serde_json::to_string(&SourceType::Stac).unwrap(),
                "\"stac\""
            );
            let parsed: SourceType = serde_json::from_str("\"stac\"").unwrap();
            assert_eq!(parsed, SourceType::Stac);
        }

        #[test]
        fn test_source_type_unknown_string_errors() {
            let result: Result<SourceType, _> = serde_json::from_str("\"banana\"");
            assert!(result.is_err());
        }
    }
}
