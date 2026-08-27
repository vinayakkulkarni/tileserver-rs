//! Annotated schema for every `config.toml` key the server understands.
//!
//! Hand-authored adjacent to [`crate::config`] so a config field addition
//! lands in one Rust file plus this file in the same PR — and so the same
//! catalog drives the operator UI at `/admin/config` without a separate
//! TypeScript catalog drifting out of sync. Served by
//! [`crate::admin::config_schema_handler`] at `GET /__admin/config/schema`.
//!
//! # Drift detection
//!
//! [`tests::schema_covers_default_config`] (in this module's `#[cfg(test)]`
//! block) round-trips `Config::default()` through serde JSON, walks every
//! key, and asserts a matching `ConfigFieldSchema` exists in
//! [`CONFIG_SCHEMA`]. Adding a field to `config.rs` without adding it here
//! fails CI immediately. The inverse is also checked — a schema entry with
//! no corresponding config field flags a typo or removed field.
//!
//! # Field type strings
//!
//! `ConfigFieldSchema::field_type` is a free-form `&'static str` so we can
//! describe Rust types succinctly to the operator (`"u8"`, `"f64[4]"`,
//! `"string[]"`, `"table"`, `"path"`, `"enum"`). The frontend only uses
//! this for the inline hint comment, not for type-checking.

use serde::Serialize;

/// Schema for one configuration key.
#[derive(Debug, Clone, Serialize)]
pub struct ConfigFieldSchema {
    /// TOML key, exact spelling.
    pub key: &'static str,
    /// Display type label (e.g. `"u32"`, `"string[]"`, `"f64[4]"`, `"enum"`).
    #[serde(rename = "type")]
    pub field_type: &'static str,
    /// String rendering of the Rust default. `None` when the field has no
    /// default (operator must supply it for the feature to work).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<&'static str>,
    /// Operator-facing description rendered as an inline comment.
    pub description: &'static str,
    /// `true` when the field is not required even if its section is
    /// present.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// Allowed values for `field_type = "enum"` fields.
    #[serde(rename = "enumValues", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<&'static [&'static str]>,
}

/// Schema for one TOML section.
#[derive(Debug, Clone, Serialize)]
pub struct ConfigSectionSchema {
    /// Section header as it appears in the TOML (e.g. `"[server]"` or
    /// `"[[sources]]"`).
    pub header: &'static str,
    /// One-sentence summary rendered above the section in the UI.
    pub blurb: &'static str,
    /// Cargo feature that must be enabled for this section to be
    /// honoured (`None` for always-on sections like `[server]`).
    #[serde(rename = "featureGate", skip_serializing_if = "Option::is_none")]
    pub feature_gate: Option<&'static str>,
    /// `true` for `[[table]]` array-of-tables sections, `false` for
    /// `[table]` singletons.
    #[serde(
        rename = "isArray",
        default,
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub is_array: bool,
    /// Fields recognised in this section.
    pub fields: &'static [ConfigFieldSchema],
}

const RESAMPLERS: &[&str] = &[
    "nearest",
    "bilinear",
    "cubic",
    "cubicspline",
    "lanczos",
    "average",
    "mode",
];

const SOURCE_TYPES: &[&str] = &[
    "pmtiles",
    "mbtiles",
    "dir",
    "tar",
    "postgres",
    "cog",
    "vrt",
    "geoparquet",
    "duckdb",
    "stac",
    "dem",
];

const PIXEL_SELECTION: &[&str] = &[
    "first",
    "highest",
    "lowest",
    "mean",
    "median",
    "stdev",
    "count",
    "lowestcloudcover",
];

const SERVE_AS: &[&str] = &["pbf", "mvt", "mlt", "png", "jpeg", "webp"];

const DEM_ENCODINGS: &[&str] = &["terrarium", "mapbox_rgb"];

const METRICS_CARDINALITY: &[&str] = &["strict", "standard", "verbose"];

/// Sentinel header for the virtual "root" section.
///
/// TOML allows scalar keys at the root before any `[section]`. Two such
/// keys live on [`crate::config::Config`] (`fonts` and `files`) — they
/// would otherwise have no place in [`CONFIG_SCHEMA`]. The drift test
/// treats this section specially: its field keys are matched against the
/// top-level keys of the serialised `Config`, not against any TOML
/// `[section]` header.
pub const ROOT_SECTION_HEADER: &str = "(root)";

/// Complete catalog of every config key the server understands.
///
/// Served verbatim at `GET /__admin/config/schema` and consumed by the
/// `/admin/config` page. New fields go here AND in `config.rs` in the
/// same commit — the drift test in this module enforces it.
pub static CONFIG_SCHEMA: &[ConfigSectionSchema] = &[
    ConfigSectionSchema {
        header: ROOT_SECTION_HEADER,
        blurb: "Top-level keys outside any [section] (TOML root namespace).",
        feature_gate: None,
        is_array: false,
        fields: &[
            ConfigFieldSchema {
                key: "fonts",
                field_type: "path",
                default: None,
                description: "Directory of PBF glyph files served at /fonts/{stack}/{range}.pbf. Omit to disable the fonts endpoint.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "files",
                field_type: "path",
                default: None,
                description: "Directory served verbatim under /files/{name}. Omit to disable the static file endpoint.",
                optional: true,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[server]",
        blurb: "Public HTTP listener, CORS, uploads, and admin bind.",
        feature_gate: None,
        is_array: false,
        fields: &[
            ConfigFieldSchema {
                key: "host",
                field_type: "string",
                default: Some("\"0.0.0.0\""),
                description: "Bind address for the public tile server.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "port",
                field_type: "u16",
                default: Some("8080"),
                description: "Bind port for the public tile server.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "cors_origins",
                field_type: "string[]",
                default: Some("[\"*\"]"),
                description: "CORS allow-list for tile + style endpoints. Lock down in production.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "admin_bind",
                field_type: "string",
                default: Some("\"127.0.0.1:0\""),
                description: "Separate bind for admin endpoints (/__admin/*). \":0\" disables. Use \"127.0.0.1:8081\" to enable.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "public_url",
                field_type: "string",
                default: None,
                description: "Public URL embedded in TileJSON. Falls back to the bind address.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "upload_dir",
                field_type: "path",
                default: None,
                description: "Directory for drag-and-drop uploads. Defaults to system tmp dir.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "upload_max_size_mb",
                field_type: "u32",
                default: Some("500"),
                description: "Maximum per-file upload size in megabytes.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "extra_response_headers",
                field_type: "table<string,string>",
                default: None,
                description: "User-defined HTTP response headers applied to every response. \
                    Header names must conform to RFC 7230 token grammar; reserved headers \
                    (Content-Length, Transfer-Encoding, Date, Connection) are rejected at startup. \
                    Empty-string values delete a header from outgoing responses.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "disable_render",
                field_type: "bool",
                default: Some("false"),
                description: "Unregister raster render routes (raster tiles + static images) \
                    at startup. style.json, sprites and WMTS stay served.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "disable_ogc",
                field_type: "bool",
                default: Some("false"),
                description: "Unregister OGC API routes (`/ogc/*`) at startup. No effect \
                    unless the binary was built with the `postgres` (OGC) feature.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "subfolder_mode",
                field_type: "enum",
                default: Some("proxy-strip"),
                description: "How to serve the embedded GUI and API when public_url \
                    carries a path (subfolder deployment). `proxy-strip`: the reverse \
                    proxy strips the prefix; `nested`: the server strips it itself. \
                    Ignored for root deployments.",
                optional: false,
                enum_values: Some(&["proxy-strip", "nested"]),
            },
        ],
    },
    ConfigSectionSchema {
        header: "[render]",
        blurb: "Native MapLibre raster renderer pool (used for COG, static images, raster tiles).",
        feature_gate: None,
        is_array: false,
        fields: &[
            ConfigFieldSchema {
                key: "pool_size",
                field_type: "usize",
                default: Some("4"),
                description: "Concurrent renderer worker threads.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "render_timeout_secs",
                field_type: "u64",
                default: Some("30"),
                description: "Per-request render timeout. Requests exceeding this are dropped.",
                optional: false,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[cache]",
        blurb: "Global in-process tile cache (moka backend).",
        feature_gate: None,
        is_array: false,
        fields: &[
            ConfigFieldSchema {
                key: "enabled",
                field_type: "bool",
                default: Some("false"),
                description: "Enable the global tile cache.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "max_size_mb",
                field_type: "u64",
                default: Some("512"),
                description: "Maximum cache size in megabytes.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "ttl_seconds",
                field_type: "u64",
                default: Some("3600"),
                description: "Time-to-live for cache entries in seconds.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "dir",
                field_type: "path",
                default: None,
                description: "Scratch / state directory for on-disk subsystems (uploads). \
                    Storage location only, not a cache eviction policy. Overridden by \
                    --cache-dir CLI flag and TILESERVER_CACHE_DIR env; falls back to \
                    the system temp dir + /tileserver-rs when all three are unset.",
                optional: true,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[compression]",
        blurb: "Tile body compression negotiation (Accept-Encoding -> br/zstd/gzip).",
        feature_gate: None,
        is_array: false,
        fields: &[
            ConfigFieldSchema {
                key: "br_quality",
                field_type: "u8",
                default: Some("5"),
                description: "Brotli quality 0-11. Higher = smaller but slower to encode.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "zstd_level",
                field_type: "i32",
                default: Some("3"),
                description: "Zstandard level 1-22. Higher = smaller but slower to encode.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "minimal_recompression",
                field_type: "bool",
                default: Some("false"),
                description: "When true, never re-encode tiles; always serve the source's \
                    stored encoding regardless of Accept-Encoding.",
                optional: false,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[raster]",
        blurb: "Raster output defaults (resampler + tile size).",
        feature_gate: Some("raster"),
        is_array: false,
        fields: &[
            ConfigFieldSchema {
                key: "default_resampling",
                field_type: "enum",
                default: Some("\"bilinear\""),
                description: "Default GDAL resampler. Per-source `resampling` overrides this.",
                optional: false,
                enum_values: Some(RESAMPLERS),
            },
            ConfigFieldSchema {
                key: "tile_size",
                field_type: "u32",
                default: Some("256"),
                description: "Output tile size in pixels (set to 512 for retina-native tiles).",
                optional: false,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[telemetry]",
        blurb: "OpenTelemetry tracing + Prometheus metrics export.",
        feature_gate: None,
        is_array: false,
        fields: &[
            ConfigFieldSchema {
                key: "enabled",
                field_type: "bool",
                default: Some("false"),
                description: "Enable OTLP trace export.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "endpoint",
                field_type: "string",
                default: Some("\"http://localhost:4317\""),
                description: "OTLP gRPC endpoint.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "service_name",
                field_type: "string",
                default: Some("\"tileserver-rs\""),
                description: "service.name resource attribute.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "sample_rate",
                field_type: "f64",
                default: Some("1.0"),
                description: "Sampling rate (0.0-1.0). 1.0 = export every span.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "metrics_enabled",
                field_type: "bool",
                default: Some("true"),
                description: "Enable OTLP metrics export (requires enabled = true).",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "metrics_export_interval_secs",
                field_type: "u64",
                default: Some("30"),
                description: "OTLP metrics push interval.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "prometheus_bind",
                field_type: "string",
                default: None,
                description: "Bind for standalone Prometheus /metrics listener (independent of OTLP). E.g. \"127.0.0.1:9100\".",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "prometheus_path",
                field_type: "string",
                default: Some("\"/metrics\""),
                description: "HTTP path for the Prometheus exposition endpoint.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "metrics_label_cardinality",
                field_type: "enum",
                default: Some("\"strict\""),
                description: "Strict = bucketed zoom, no tile coords. Standard = alias of strict. Verbose = full zoom 0..22.",
                optional: false,
                enum_values: Some(METRICS_CARDINALITY),
            },
        ],
    },
    ConfigSectionSchema {
        header: "[[sources]]",
        blurb: "Tile source. Repeat for each source. Type-specific fields appear based on `type`.",
        feature_gate: None,
        is_array: true,
        fields: &[
            ConfigFieldSchema {
                key: "id",
                field_type: "string",
                default: None,
                description: "Unique identifier (becomes /data/<id>/* route). Required.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "type",
                field_type: "enum",
                default: None,
                description: "Source backend. Required.",
                optional: false,
                enum_values: Some(SOURCE_TYPES),
            },
            ConfigFieldSchema {
                key: "path",
                field_type: "string",
                default: None,
                description: "File path, HTTP(S) URL, or cloud URL (s3://, gs://, az://). Required.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "name",
                field_type: "string",
                default: None,
                description: "Display name shown in the viewer.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "attribution",
                field_type: "string",
                default: None,
                description: "Attribution text included in TileJSON.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "description",
                field_type: "string",
                default: None,
                description: "Free-form description.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "minzoom",
                field_type: "u8",
                default: None,
                description: "Override the minzoom from source metadata.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "maxzoom",
                field_type: "u8",
                default: None,
                description: "Override the maxzoom from source metadata.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "serve_as",
                field_type: "enum",
                default: None,
                description: "Transcode on the fly. E.g. serve_as = \"mlt\" on a PBF source emits MLT.",
                optional: true,
                enum_values: Some(SERVE_AS),
            },
            ConfigFieldSchema {
                key: "options",
                field_type: "table",
                default: None,
                description: "Key-value map forwarded to cloud backends (S3 credentials, GCS keys, etc.).",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "resampling",
                field_type: "enum",
                default: None,
                description: "Per-source resampler override (raster only). Defaults to [raster].default_resampling.",
                optional: true,
                enum_values: Some(RESAMPLERS),
            },
            ConfigFieldSchema {
                key: "colormap",
                field_type: "table",
                default: None,
                description: "Inline colormap (raster only). See docs for full schema.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "collection",
                field_type: "string",
                default: None,
                description: "STAC collection ID (STAC sources only).",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "asset_role",
                field_type: "string",
                default: Some("\"visual\""),
                description: "STAC asset role to render (STAC sources only).",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "dynamic",
                field_type: "bool",
                default: Some("false"),
                description: "Enable on-demand STAC search per tile (STAC sources only).",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "max_items",
                field_type: "usize",
                default: Some("100"),
                description: "Max items per tile when dynamic = true (STAC sources only).",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "stac_bbox",
                field_type: "f64[4]",
                default: None,
                description: "Override the bbox passed to STAC search.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "pixel_selection",
                field_type: "enum",
                default: Some("\"first\""),
                description: "STAC mosaic strategy.",
                optional: true,
                enum_values: Some(PIXEL_SELECTION),
            },
            ConfigFieldSchema {
                key: "tile_path_template",
                field_type: "string",
                default: None,
                description: "Tile layout for dir/tar sources. Defaults to {z}/{x}/{y}.{ext}.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "tms",
                field_type: "bool",
                default: Some("false"),
                description: "Use TMS (south-up) addressing for dir/tar sources. Default false = XYZ.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "input_source",
                field_type: "string",
                default: None,
                description: "Reference another [[sources]] entry by id whose underlying raster tiles will be re-encoded as DEM. When set, `path` is ignored on this source. Requires the target source to load first.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "dem_encoding",
                field_type: "enum",
                default: Some("\"terrarium\""),
                description: "DEM RGB encoding for the tile PNG. `terrarium` (default) yields 1/256 m precision, `mapbox_rgb` yields 0.1 m precision and is the legacy `terrain-rgb` Mapbox tile format.",
                optional: true,
                enum_values: Some(DEM_ENCODINGS),
            },
            ConfigFieldSchema {
                key: "dem_scale",
                field_type: "f64",
                default: None,
                description: "Multiplicative scale applied to source elevation before encoding. Useful for unit conversion (e.g. feet -> metres: 0.3048).",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "dem_offset",
                field_type: "f64",
                default: None,
                description: "Additive offset applied to source elevation before encoding (after scale).",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "dem_band",
                field_type: "u32",
                default: Some("1"),
                description: "GDAL raster band to encode (1-indexed). Default 1.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "dem_nodata_color",
                field_type: "u8[4]",
                default: None,
                description: "RGBA sentinel for nodata pixels as [r, g, b, a]. MapLibre GL JS IGNORES alpha so the RGB must encode \"no data\". Default for mapbox_rgb: [1, 134, 160]. Default for terrarium: [0, 0, 0].",
                optional: true,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[[styles]]",
        blurb: "MapLibre style. Repeat for each style.",
        feature_gate: None,
        is_array: true,
        fields: &[
            ConfigFieldSchema {
                key: "id",
                field_type: "string",
                default: None,
                description: "Style identifier (becomes /styles/<id>/* route). Required.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "path",
                field_type: "string",
                default: None,
                description: "Path to a MapLibre style JSON file. Required.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "name",
                field_type: "string",
                default: None,
                description: "Display name shown in the viewer.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "attribution",
                field_type: "string",
                default: None,
                description: "Attribution text.",
                optional: true,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[[composites]]",
        blurb: "Named multi-source composite. Merges member sources into one \
                vector tile endpoint. Repeat for each composite.",
        feature_gate: None,
        is_array: true,
        fields: &[
            ConfigFieldSchema {
                key: "id",
                field_type: "string",
                default: None,
                description: "Composite identifier (becomes /data/<id> route). Required.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "sources",
                field_type: "array",
                default: None,
                description: "Member source ids merged into this composite. Required.",
                optional: false,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[postgres]",
        blurb: "PostgreSQL connection pool + per-source registries.",
        feature_gate: Some("postgres"),
        is_array: false,
        fields: &[
            ConfigFieldSchema {
                key: "connection_string",
                field_type: "string",
                default: None,
                description: "Postgres connection URI. Required.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "pool_size",
                field_type: "usize",
                default: Some("10"),
                description: "Connections in the pool.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "pool_wait_timeout_ms",
                field_type: "u64",
                default: Some("5000"),
                description: "Max ms to wait for a free connection.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "pool_create_timeout_ms",
                field_type: "u64",
                default: Some("5000"),
                description: "Max ms to wait when opening a new connection.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "pool_recycle_timeout_ms",
                field_type: "u64",
                default: Some("5000"),
                description: "Max ms to wait recycling a stale connection.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "pool_pre_warm",
                field_type: "bool",
                default: Some("false"),
                description: "Open all pool connections at startup.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "ssl_cert",
                field_type: "path",
                default: None,
                description: "mTLS client certificate (PEM).",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "ssl_key",
                field_type: "path",
                default: None,
                description: "mTLS client key (PEM).",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "ssl_root_cert",
                field_type: "path",
                default: None,
                description: "Root CA for verify-full SSL.",
                optional: true,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[postgres.cache]",
        blurb: "Per-Postgres MVT tile cache. Independent of the global [cache].",
        feature_gate: Some("postgres"),
        is_array: false,
        fields: &[
            ConfigFieldSchema {
                key: "size_mb",
                field_type: "u64",
                default: Some("256"),
                description: "Max cache size for Postgres MVT tiles.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "ttl_seconds",
                field_type: "u64",
                default: Some("300"),
                description: "TTL per entry.",
                optional: false,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[[postgres.functions]]",
        blurb: "Postgres function source. Function must take (z, x, y) and return bytea.",
        feature_gate: Some("postgres"),
        is_array: true,
        fields: &[
            ConfigFieldSchema {
                key: "id",
                field_type: "string",
                default: None,
                description: "Source ID. Required.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "schema",
                field_type: "string",
                default: None,
                description: "Postgres schema. Required.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "function",
                field_type: "string",
                default: None,
                description: "Function name. Required.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "name",
                field_type: "string",
                default: None,
                description: "Display name.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "attribution",
                field_type: "string",
                default: None,
                description: "Attribution text.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "description",
                field_type: "string",
                default: None,
                description: "Free-form description.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "minzoom",
                field_type: "u8",
                default: Some("0"),
                description: "Minimum zoom.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "maxzoom",
                field_type: "u8",
                default: Some("22"),
                description: "Maximum zoom.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "bounds",
                field_type: "f64[4]",
                default: None,
                description: "Geographic bounds [west, south, east, north].",
                optional: true,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[[postgres.tables]]",
        blurb: "Postgres table source. tileserver-rs builds MVT tiles from the table on the fly.",
        feature_gate: Some("postgres"),
        is_array: true,
        fields: &[
            ConfigFieldSchema {
                key: "id",
                field_type: "string",
                default: None,
                description: "Source ID. Required.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "schema",
                field_type: "string",
                default: None,
                description: "Postgres schema. Required.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "table",
                field_type: "string",
                default: None,
                description: "Table name. Required.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "geometry_column",
                field_type: "string",
                default: Some("\"geom\""),
                description: "Geometry column.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "id_column",
                field_type: "string",
                default: None,
                description: "Optional integer ID column carried into MVT features.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "properties",
                field_type: "string[]",
                default: None,
                description: "Column allow-list for MVT properties. Defaults to all columns.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "name",
                field_type: "string",
                default: None,
                description: "Display name.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "attribution",
                field_type: "string",
                default: None,
                description: "Attribution text.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "description",
                field_type: "string",
                default: None,
                description: "Free-form description.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "minzoom",
                field_type: "u8",
                default: Some("0"),
                description: "Minimum zoom.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "maxzoom",
                field_type: "u8",
                default: Some("22"),
                description: "Maximum zoom.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "bounds",
                field_type: "f64[4]",
                default: None,
                description: "Geographic bounds [west, south, east, north].",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "extent",
                field_type: "u32",
                default: Some("4096"),
                description: "MVT extent (4096 = standard).",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "buffer",
                field_type: "u32",
                default: Some("64"),
                description: "Per-tile geometry buffer in pixels (auto-set to 0 for POINT/MULTIPOINT).",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "max_features",
                field_type: "u32",
                default: None,
                description: "Hard cap on emitted features per tile.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "writable",
                field_type: "bool",
                default: Some("false"),
                description: "Enable OGC API Features POST/PATCH/DELETE on this table.",
                optional: false,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[mcp]",
        blurb: "Model Context Protocol server (AI assistant integration).",
        feature_gate: Some("mcp"),
        is_array: false,
        fields: &[
            ConfigFieldSchema {
                key: "enabled",
                field_type: "bool",
                default: Some("false"),
                description: "Mount the /mcp Streamable HTTP service.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "auth_token",
                field_type: "string",
                default: None,
                description: "Static bearer token. Mutually exclusive with oauth.enabled = true.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "cors_origins",
                field_type: "string[]",
                default: Some("[\"*\"]"),
                description: "CORS allow-list for /mcp. Lock down in production.",
                optional: false,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[mcp.oauth]",
        blurb: "OAuth 2.0 + RFC 7591 DCR for the MCP HTTP transport (claude.ai Custom Connectors).",
        feature_gate: Some("mcp"),
        is_array: false,
        fields: &[
            ConfigFieldSchema {
                key: "enabled",
                field_type: "bool",
                default: Some("false"),
                description: "Enable RFC 7591 DCR + JWT RS256 OAuth flow.",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "issuer_url",
                field_type: "string",
                default: None,
                description: "Required when enabled = true. Must match the public URL.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "signing_key_path",
                field_type: "path",
                default: None,
                description: "Required when enabled = true. RSA PKCS#8 PEM private key.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "token_ttl_secs",
                field_type: "u64",
                default: Some("3600"),
                description: "Access-token TTL (clamped to 86400).",
                optional: false,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "store_path",
                field_type: "path",
                default: None,
                description: "SQLite file persisting OAuth clients/tokens across restarts. Requires the mcp-persistence feature.",
                optional: true,
                enum_values: None,
            },
        ],
    },
    ConfigSectionSchema {
        header: "[sftp]",
        blurb: "Global defaults for SFTP PMTiles sources. Per-source overrides live in [[sources]].options.",
        feature_gate: Some("sftp"),
        is_array: false,
        fields: &[
            ConfigFieldSchema {
                key: "known_hosts_path",
                field_type: "path",
                default: Some("~/.ssh/known_hosts"),
                description: "Default known_hosts file. Overridden per-source by options.ssh_known_hosts_path.",
                optional: true,
                enum_values: None,
            },
            ConfigFieldSchema {
                key: "strict_host_key",
                field_type: "bool",
                default: Some("true"),
                description: "When false, accept first-seen host keys (TOFU). Default fails closed.",
                optional: true,
                enum_values: None,
            },
        ],
    },
];

/// Look up the schema for a top-level section by exact header match.
///
/// Used by [`crate::admin`] when annotating the loaded TOML.
#[must_use]
pub fn section(header: &str) -> Option<&'static ConfigSectionSchema> {
    CONFIG_SCHEMA.iter().find(|s| s.header == header)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_section_header_is_unique() {
        let mut seen = std::collections::HashSet::new();
        for s in CONFIG_SCHEMA {
            assert!(
                seen.insert(s.header),
                "duplicate section header in catalog: {}",
                s.header,
            );
        }
    }

    #[test]
    fn every_field_key_is_unique_within_its_section() {
        for section in CONFIG_SCHEMA {
            let mut seen = std::collections::HashSet::new();
            for f in section.fields {
                assert!(
                    seen.insert(f.key),
                    "duplicate field key '{}' in section '{}'",
                    f.key,
                    section.header,
                );
            }
        }
    }

    #[test]
    fn enum_fields_carry_their_values_and_others_do_not() {
        for section in CONFIG_SCHEMA {
            for f in section.fields {
                if f.field_type == "enum" {
                    assert!(
                        f.enum_values.is_some(),
                        "field '{}' in section '{}' has type=enum but no enum_values",
                        f.key,
                        section.header,
                    );
                } else {
                    assert!(
                        f.enum_values.is_none(),
                        "field '{}' in section '{}' has enum_values but type={}",
                        f.key,
                        section.header,
                        f.field_type,
                    );
                }
            }
        }
    }

    #[test]
    fn schema_covers_default_config() {
        let default_config = crate::config::Config::default();
        let json = serde_json::to_value(&default_config).expect("serialise Config default");
        let map = json
            .as_object()
            .expect("config serialises as a JSON object");

        let mut known_keys: std::collections::HashSet<&'static str> = CONFIG_SCHEMA
            .iter()
            .filter(|s| s.header != ROOT_SECTION_HEADER)
            .map(|s| {
                let h = s.header.trim_start_matches("[[").trim_start_matches('[');
                let h = h.trim_end_matches("]]").trim_end_matches(']');
                h.split('.').next().unwrap_or(h)
            })
            .collect();

        if let Some(root) = section(ROOT_SECTION_HEADER) {
            for f in root.fields {
                known_keys.insert(f.key);
            }
        }

        for key in map.keys() {
            assert!(
                known_keys.contains(key.as_str()),
                "config field '{key}' has no entry in CONFIG_SCHEMA — \
                 add it to an existing section or to the (root) section \
                 in src/config_schema.rs",
            );
        }
    }

    #[test]
    fn server_section_lists_every_serialised_server_field() {
        let default_config = crate::config::Config::default();
        let json = serde_json::to_value(&default_config).expect("serialise Config default");
        let server = json["server"].as_object().expect("server is a JSON object");

        let server_schema = section("[server]").expect("[server] section present in CONFIG_SCHEMA");
        let known: std::collections::HashSet<&'static str> =
            server_schema.fields.iter().map(|f| f.key).collect();

        for key in server.keys() {
            assert!(
                known.contains(key.as_str()),
                "server field '{key}' missing from CONFIG_SCHEMA [server] entry",
            );
        }
    }
}
