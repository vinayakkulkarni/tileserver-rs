//! MCP tool handlers — Tier A introspection + Tier B data query.
//!
//! All tools are namespaced with the `tileserver_` prefix per Anthropic's
//! [MCP naming convention](https://www.anthropic.com/engineering/writing-tools-for-agents),
//! and every backend failure is mapped to a tool-execution error via
//! [`crate::mcp::error::tile_error_to_call_result`] (never JSON-RPC).
//!
//! State propagation: the handler owns an `Arc<AppState>` and is `Clone` so
//! `StreamableHttpService` can mint a fresh handler per session while still
//! sharing the underlying source / style / renderer pools.

#![allow(clippy::needless_pass_by_value)] // rmcp tool macro requires owned Parameters<T>

use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use rmcp::ErrorData as McpError;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, Content, GetPromptRequestParams, GetPromptResult, ListPromptsResult,
    ListResourceTemplatesResult, PaginatedRequestParams, ProtocolVersion,
    ReadResourceRequestParams, ReadResourceResult, ServerCapabilities, ServerInfo,
};
use rmcp::service::{NotificationContext, RequestContext, RoleServer};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mcp::error::{tile_error_to_call_result, tool_error};
use crate::mcp::prompts::{get_prompt, list_prompts};
use crate::mcp::resources::{list_resource_templates, read_resource};
use crate::reload::AppState;
use crate::render::{ImageFormat, RenderOptions, StaticQueryParams, StaticType};

/// Soft upper bound on rendered image dimensions (per axis, in CSS pixels).
///
/// Anthropic's MCP image content limit is 5 MB base64; WebP at Q75 stays
/// comfortably under that at 2048x2048. The hard limit in [`crate::render`]
/// is 4096 — we stop earlier here to leave headroom for the base64 encoding
/// step (≈ 1.33× expansion).
const MAX_RENDER_DIMENSION: u32 = 2048;

/// Soft byte cap on rendered image payloads (1.5 MB), well below the 5 MB
/// hard limit on the Anthropic side.
const MAX_RENDER_BYTES: usize = 1_572_864;

/// Default tile size used by [`McpHandler::tileserver_query_features_at_point`]
/// when computing the bbox around the click coordinate.
const POINT_QUERY_DEFAULT_LIMIT: i64 = 10;

/// Default `limit` for paginated STAC item search.
#[cfg(feature = "stac")]
const STAC_SEARCH_DEFAULT_LIMIT: i64 = 10;

/// Default `limit` for CQL2 feature query.
#[cfg(feature = "postgres")]
const CQL2_QUERY_DEFAULT_LIMIT: i64 = 50;

/// Empty-input wrapper used by tools that take no parameters but still need
/// a `Parameters<T>` extractor (rmcp 1.7 requires it for argument schema
/// emission even when the schema is empty).
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
pub struct EmptyArgs {}

/// Input parameters for [`McpHandler::tileserver_get_source_tilejson`].
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetSourceTilejsonArgs {
    /// Source id as registered in `[[sources]]` configuration. Must match
    /// one of the ids returned by `tileserver_list_sources`.
    pub source_id: String,
}

/// Input parameters for [`McpHandler::tileserver_get_style`].
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetStyleArgs {
    /// Style id as registered in `[[styles]]` configuration. Must match
    /// one of the ids returned by `tileserver_list_styles`.
    pub style_id: String,
}

/// Input parameters for [`McpHandler::tileserver_get_tile_metadata`].
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetTileMetadataArgs {
    /// Source id whose `TileMetadata` (bounds, zoom range, format) to return.
    pub source_id: String,
}

/// Input parameters for [`McpHandler::tileserver_render_static_map`].
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct RenderStaticMapArgs {
    /// Style id used to render the map (must exist in `[[styles]]`).
    pub style_id: String,
    /// Longitude of the map center (WGS84 decimal degrees).
    pub lon: f64,
    /// Latitude of the map center (WGS84 decimal degrees).
    pub lat: f64,
    /// Zoom level (0-22). Higher values show more detail.
    pub zoom: f64,
    /// Image width in CSS pixels (1-2048).
    pub width: u32,
    /// Image height in CSS pixels (1-2048).
    pub height: u32,
    /// Optional camera bearing in degrees (default 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearing: Option<f64>,
    /// Optional camera pitch in degrees (default 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pitch: Option<f64>,
}

/// Input parameters for [`McpHandler::tileserver_get_tile`].
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetTileArgs {
    /// Source id whose tile to fetch.
    pub source_id: String,
    /// Zoom level (0-22).
    pub z: u8,
    /// Tile column index (XYZ scheme).
    pub x: u32,
    /// Tile row index (XYZ scheme, top-down).
    pub y: u32,
}

/// Input parameters for [`McpHandler::tileserver_query_features_at_point`].
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct QueryFeaturesAtPointArgs {
    /// Source id (must be a PostgreSQL table source in v1).
    pub source_id: String,
    /// Longitude in WGS84 decimal degrees.
    pub lon: f64,
    /// Latitude in WGS84 decimal degrees.
    pub lat: f64,
    /// Optional half-side of the bbox built around the click point, in
    /// decimal degrees. Defaults to ~0.0001° (~10m at the equator).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius_deg: Option<f64>,
    /// Maximum number of features to return (default 10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
}

/// Input parameters for [`McpHandler::tileserver_query_features_cql2`].
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct QueryFeaturesCql2Args {
    /// PostgreSQL table source id.
    pub source_id: String,
    /// CQL2-text expression (e.g. `name = 'Berlin' AND population > 1000000`).
    pub cql2: String,
    /// Optional `[west, south, east, north]` bbox to combine with the filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
    /// Maximum number of features to return (default 50).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
}

/// Input parameters for [`McpHandler::tileserver_search_stac_items`].
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SearchStacItemsArgs {
    /// STAC source id (must be a `type = "stac"` source).
    pub source_id: String,
    /// Optional `[west, south, east, north]` bbox to filter assets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
    /// Optional RFC 3339 datetime range or instant to filter by capture time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datetime: Option<String>,
    /// Maximum number of items to return (default 10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
}

/// Server info payload returned by `tileserver_get_server_info` — mirrors the
/// `/ping` admin endpoint so MCP clients can introspect runtime state without
/// hitting HTTP.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ServerInfoPayload {
    /// Crate version (matches `Cargo.toml` `package.version`).
    pub version: &'static str,
    /// Hash of the loaded TOML configuration.
    pub config_hash: String,
    /// Number of tile sources successfully loaded.
    pub loaded_sources: usize,
    /// Number of map styles successfully loaded.
    pub loaded_styles: usize,
    /// Whether the native MapLibre renderer pool is initialized.
    pub renderer_enabled: bool,
    /// Whether the in-process tile cache is active.
    pub cache_enabled: bool,
    /// Public base URL used for tile URL templates in TileJSON responses.
    pub base_url: String,
}

/// MCP server handler — holds a clone of the live [`AppState`] and dispatches
/// MCP tool calls to the underlying tile / style / render subsystems.
#[derive(Clone)]
pub struct McpHandler {
    state: Arc<AppState>,
}

impl McpHandler {
    /// Build a new handler bound to the given application state snapshot.
    ///
    /// The handler holds an `Arc<AppState>` rather than a `SharedState`
    /// reload handle: an MCP session sees a stable view for its entire
    /// lifetime, which is the conservative choice. To pick up reloads,
    /// re-construct the handler factory in [`crate::mcp::transport`].
    #[must_use]
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    fn tilejson_for(&self, source_id: &str) -> Option<crate::sources::TileJson> {
        self.state
            .sources
            .get(source_id)
            .map(|src| src.metadata().to_tilejson(&self.state.base_url))
    }
}

#[tool_router]
impl McpHandler {
    #[tool(
        name = "tileserver_list_sources",
        description = "List all registered tile sources with their TileJSON metadata. Use this to discover what data is available before calling tileserver_get_tile or tileserver_get_source_tilejson."
    )]
    async fn tileserver_list_sources(
        &self,
        _params: Parameters<EmptyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let ids = self.state.sources.ids();
        let mut tilejsons: Vec<crate::sources::TileJson> = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(tj) = self.tilejson_for(id) {
                tilejsons.push(tj);
            }
        }
        match Content::json(&tilejsons) {
            Ok(content) => Ok(CallToolResult::success(vec![content])),
            Err(err) => Ok(tile_error_to_call_result(
                crate::error::TileServerError::Internal(anyhow::anyhow!(
                    "failed to serialize source list: {err}"
                )),
            )),
        }
    }

    #[tool(
        name = "tileserver_get_source_tilejson",
        description = "Return the TileJSON 3.0 metadata for a single tile source (bounds, zoom range, tile URL template, vector layers)."
    )]
    async fn tileserver_get_source_tilejson(
        &self,
        params: Parameters<GetSourceTilejsonArgs>,
    ) -> Result<CallToolResult, McpError> {
        let Parameters(GetSourceTilejsonArgs { source_id }) = params;
        let Some(tj) = self.tilejson_for(&source_id) else {
            return Ok(tile_error_to_call_result(
                crate::error::TileServerError::SourceNotFound(source_id),
            ));
        };
        match Content::json(&tj) {
            Ok(c) => Ok(CallToolResult::success(vec![c])),
            Err(e) => Ok(tile_error_to_call_result(
                crate::error::TileServerError::Internal(anyhow::anyhow!(
                    "failed to serialize TileJSON: {e}"
                )),
            )),
        }
    }

    #[tool(
        name = "tileserver_list_styles",
        description = "List all registered map styles with their ids, names, and URLs."
    )]
    async fn tileserver_list_styles(
        &self,
        _params: Parameters<EmptyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let infos = self.state.styles.all_infos(&self.state.base_url);
        match Content::json(&infos) {
            Ok(c) => Ok(CallToolResult::success(vec![c])),
            Err(e) => Ok(tile_error_to_call_result(
                crate::error::TileServerError::Internal(anyhow::anyhow!(
                    "failed to serialize style list: {e}"
                )),
            )),
        }
    }

    #[tool(
        name = "tileserver_get_style",
        description = "Return the full MapLibre style JSON for a registered style. The style is returned verbatim — URLs may be relative."
    )]
    async fn tileserver_get_style(
        &self,
        params: Parameters<GetStyleArgs>,
    ) -> Result<CallToolResult, McpError> {
        let Parameters(GetStyleArgs { style_id }) = params;
        let Some(style) = self.state.styles.get(&style_id) else {
            return Ok(tile_error_to_call_result(
                crate::error::TileServerError::StyleNotFound(style_id),
            ));
        };
        match Content::json(&style.style_json) {
            Ok(c) => Ok(CallToolResult::success(vec![c])),
            Err(e) => Ok(tile_error_to_call_result(
                crate::error::TileServerError::Internal(anyhow::anyhow!(
                    "failed to serialize style JSON: {e}"
                )),
            )),
        }
    }

    #[tool(
        name = "tileserver_get_tile_metadata",
        description = "Return the raw TileMetadata (bounds, minzoom, maxzoom, format, vector_layers) for a source. Lower-level than tileserver_get_source_tilejson."
    )]
    async fn tileserver_get_tile_metadata(
        &self,
        params: Parameters<GetTileMetadataArgs>,
    ) -> Result<CallToolResult, McpError> {
        let Parameters(GetTileMetadataArgs { source_id }) = params;
        let Some(source) = self.state.sources.get(&source_id) else {
            return Ok(tile_error_to_call_result(
                crate::error::TileServerError::SourceNotFound(source_id),
            ));
        };
        match Content::json(source.metadata()) {
            Ok(c) => Ok(CallToolResult::success(vec![c])),
            Err(e) => Ok(tile_error_to_call_result(
                crate::error::TileServerError::Internal(anyhow::anyhow!(
                    "failed to serialize TileMetadata: {e}"
                )),
            )),
        }
    }

    #[tool(
        name = "tileserver_get_server_info",
        description = "Return version, config hash, loaded source/style counts, and runtime feature flags for the tileserver-rs process."
    )]
    async fn tileserver_get_server_info(
        &self,
        _params: Parameters<EmptyArgs>,
    ) -> Result<CallToolResult, McpError> {
        let payload = ServerInfoPayload {
            version: env!("CARGO_PKG_VERSION"),
            config_hash: "unavailable-from-mcp".to_string(),
            loaded_sources: self.state.sources.len(),
            loaded_styles: self.state.styles.len(),
            renderer_enabled: self.state.renderer.is_some(),
            cache_enabled: self.state.sources.cache().is_some(),
            base_url: self.state.base_url.clone(),
        };
        match Content::json(&payload) {
            Ok(c) => Ok(CallToolResult::success(vec![c])),
            Err(e) => Ok(tile_error_to_call_result(
                crate::error::TileServerError::Internal(anyhow::anyhow!(
                    "failed to serialize ServerInfo: {e}"
                )),
            )),
        }
    }

    #[tool(
        name = "tileserver_render_static_map",
        description = "Render a static map image (WebP, base64-encoded) at the given center, zoom, and dimensions. Requires the native renderer to be enabled. Max dimensions: 2048x2048."
    )]
    async fn tileserver_render_static_map(
        &self,
        params: Parameters<RenderStaticMapArgs>,
    ) -> Result<CallToolResult, McpError> {
        let Parameters(args) = params;

        let Some(renderer) = self.state.renderer.as_ref() else {
            return Ok(tool_error(
                "native renderer is disabled; rebuild with --features raster and set render.pool_size > 0",
            ));
        };

        if args.width == 0 || args.height == 0 {
            return Ok(tool_error("width and height must be greater than zero"));
        }
        if args.width > MAX_RENDER_DIMENSION || args.height > MAX_RENDER_DIMENSION {
            return Ok(tool_error(format!(
                "image dimensions exceed MCP cap (max {MAX_RENDER_DIMENSION}x{MAX_RENDER_DIMENSION}); got {}x{}",
                args.width, args.height
            )));
        }

        let Some(style) = self.state.styles.get(&args.style_id) else {
            return Ok(tile_error_to_call_result(
                crate::error::TileServerError::StyleNotFound(args.style_id),
            ));
        };

        let style_json = match crate::styles::rewrite_style_for_api(
            &style.style_json,
            &self.state.render_base_url,
            &crate::styles::UrlQueryParams::default(),
            &self.state.sources,
        ) {
            value if value.is_object() || value.is_array() => match serde_json::to_string(&value) {
                Ok(s) => s,
                Err(e) => {
                    return Ok(tile_error_to_call_result(
                        crate::error::TileServerError::Internal(anyhow::anyhow!(
                            "failed to serialize rewritten style: {e}"
                        )),
                    ));
                }
            },
            _ => {
                return Ok(tool_error(
                    "rewritten style is neither an object nor an array",
                ));
            }
        };

        let render_options = match RenderOptions::for_static(
            style.id.clone(),
            style_json,
            StaticType::Center {
                lon: args.lon,
                lat: args.lat,
                zoom: args.zoom,
                bearing: args.bearing,
                pitch: args.pitch,
            },
            args.width,
            args.height,
            1,
            ImageFormat::Webp,
            StaticQueryParams::default(),
        ) {
            Ok(o) => o,
            Err(e) => return Ok(tool_error(format!("invalid render options: {e}"))),
        };

        let bytes = match renderer.render_static(render_options).await {
            Ok(b) => b,
            Err(e) => return Ok(tile_error_to_call_result(e)),
        };

        if bytes.len() > MAX_RENDER_BYTES {
            return Ok(tool_error(format!(
                "rendered image is {} bytes; exceeds MCP soft cap of {MAX_RENDER_BYTES} bytes — request smaller dimensions",
                bytes.len()
            )));
        }

        let encoded = BASE64_STANDARD.encode(&bytes);
        Ok(CallToolResult::success(vec![Content::image(
            encoded,
            "image/webp",
        )]))
    }

    #[tool(
        name = "tileserver_get_tile",
        description = "Fetch the raw bytes of a vector or raster tile at z/x/y. Returns base64-encoded bytes with the source's content type."
    )]
    async fn tileserver_get_tile(
        &self,
        params: Parameters<GetTileArgs>,
    ) -> Result<CallToolResult, McpError> {
        let Parameters(GetTileArgs { source_id, z, x, y }) = params;

        let tile = match self.state.sources.get_tile(&source_id, z, x, y).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                return Ok(tile_error_to_call_result(
                    crate::error::TileServerError::TileNotFound { z, x, y },
                ));
            }
            Err(e) => return Ok(tile_error_to_call_result(e)),
        };

        let mime = tile.format.content_type();
        let encoded = BASE64_STANDARD.encode(&tile.data);

        let summary = serde_json::json!({
            "source_id": source_id,
            "z": z,
            "x": x,
            "y": y,
            "format": tile.format.extension(),
            "size_bytes": tile.data.len(),
            "mime_type": mime,
            "data_base64": encoded,
        });
        match Content::json(&summary) {
            Ok(c) => Ok(CallToolResult::success(vec![c])),
            Err(e) => Ok(tile_error_to_call_result(
                crate::error::TileServerError::Internal(anyhow::anyhow!(
                    "failed to serialize tile payload: {e}"
                )),
            )),
        }
    }

    #[tool(
        name = "tileserver_query_features_at_point",
        description = "Query features intersecting a click point on a PostgreSQL table source. Only PostgreSQL-backed sources are supported in v1; other source types return an error result."
    )]
    async fn tileserver_query_features_at_point(
        &self,
        params: Parameters<QueryFeaturesAtPointArgs>,
    ) -> Result<CallToolResult, McpError> {
        let Parameters(args) = params;
        let _ = args.lon;
        let _ = args.lat;
        let _ = args.radius_deg;
        let _ = args.limit.unwrap_or(POINT_QUERY_DEFAULT_LIMIT);

        #[cfg(not(feature = "postgres"))]
        {
            let _ = args.source_id;
            return Ok(tool_error(
                "feature query at point requires the `postgres` feature; this binary was built without it",
            ));
        }

        #[cfg(feature = "postgres")]
        {
            point_query_postgres(&self.state, &args).await
        }
    }

    #[tool(
        name = "tileserver_query_features_cql2",
        description = "Run a CQL2-text filter against a PostgreSQL feature source. Returns a GeoJSON FeatureCollection. Requires the `postgres` feature."
    )]
    async fn tileserver_query_features_cql2(
        &self,
        params: Parameters<QueryFeaturesCql2Args>,
    ) -> Result<CallToolResult, McpError> {
        let Parameters(args) = params;
        let _ = args;
        #[cfg(not(feature = "postgres"))]
        {
            return Ok(tool_error(
                "tileserver_query_features_cql2 requires the `postgres` feature; this binary was built without it",
            ));
        }
        #[cfg(feature = "postgres")]
        {
            cql2_query_postgres(&self.state, &args).await
        }
    }

    #[tool(
        name = "tileserver_search_stac_items",
        description = "Search items in a STAC-backed source. v1 returns the discovered_assets snapshot filtered client-side by optional bbox / datetime / limit. Requires the `stac` feature."
    )]
    async fn tileserver_search_stac_items(
        &self,
        params: Parameters<SearchStacItemsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let Parameters(args) = params;
        let _ = args;
        #[cfg(not(feature = "stac"))]
        {
            return Ok(tool_error(
                "tileserver_search_stac_items requires the `stac` feature; this binary was built without it",
            ));
        }
        #[cfg(feature = "stac")]
        {
            stac_search(&self.state, &args)
        }
    }
}

#[tool_handler]
impl ServerHandler for McpHandler {
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_prompts()
            .build();
        ServerInfo::new(capabilities)
            .with_protocol_version(ProtocolVersion::default())
            .with_server_info(
                rmcp::model::Implementation::new("tileserver-rs", env!("CARGO_PKG_VERSION"))
                    .with_title("tileserver-rs MCP"),
            )
            .with_instructions(
                "tileserver-rs exposes vector tile sources, map styles, and a native renderer. \
                 Use tileserver_list_sources and tileserver_list_styles to discover content. \
                 Use tileserver_render_static_map to produce preview images. \
                 Resources at tileserver://styles/{id} and tileserver://data/{id}.json mirror \
                 the introspection tools as read-only handles. \
                 Prompts (describe_style, suggest_cql2_filter, render_location_preview, \
                 explain_tile_metadata) provide reusable scaffolds for common workflows.",
            )
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(list_resource_templates())
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        read_resource(&request.uri, &self.state)
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(list_prompts())
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        get_prompt(&request)
    }

    async fn on_cancelled(
        &self,
        _notification: rmcp::model::CancelledNotificationParam,
        _context: NotificationContext<RoleServer>,
    ) {
    }
}

#[cfg(feature = "postgres")]
async fn point_query_postgres(
    state: &Arc<AppState>,
    args: &QueryFeaturesAtPointArgs,
) -> Result<CallToolResult, McpError> {
    use crate::sources::postgres::PostgresTableSource;

    let Some(source) = state.sources.get(&args.source_id) else {
        return Ok(tile_error_to_call_result(
            crate::error::TileServerError::SourceNotFound(args.source_id.clone()),
        ));
    };

    let Some(table) = source.as_any().downcast_ref::<PostgresTableSource>() else {
        return Ok(tool_error(format!(
            "source '{}' is not a PostgreSQL table source; feature query at point is only supported for PostgreSQL table sources currently",
            args.source_id
        )));
    };

    let radius = args.radius_deg.unwrap_or(0.0001).abs();
    let bbox = [
        args.lon - radius,
        args.lat - radius,
        args.lon + radius,
        args.lat + radius,
    ];
    let limit = args
        .limit
        .unwrap_or(POINT_QUERY_DEFAULT_LIMIT)
        .clamp(1, 1000);

    let (features, _matched) = match table
        .query_features_geojson(Some(bbox), 4326, 4326, None, None, limit, 0)
        .await
    {
        Ok(v) => v,
        Err(e) => return Ok(tile_error_to_call_result(e)),
    };

    let fc = serde_json::json!({
        "type": "FeatureCollection",
        "features": features,
        "numberReturned": features.len(),
    });
    match Content::json(&fc) {
        Ok(c) => Ok(CallToolResult::success(vec![c])),
        Err(e) => Ok(tile_error_to_call_result(
            crate::error::TileServerError::Internal(anyhow::anyhow!(
                "failed to serialize FeatureCollection: {e}"
            )),
        )),
    }
}

#[cfg(feature = "postgres")]
async fn cql2_query_postgres(
    state: &Arc<AppState>,
    args: &QueryFeaturesCql2Args,
) -> Result<CallToolResult, McpError> {
    use crate::sources::postgres::PostgresTableSource;
    use cql2::{Expr, ToSqlAst};

    let Some(source) = state.sources.get(&args.source_id) else {
        return Ok(tile_error_to_call_result(
            crate::error::TileServerError::SourceNotFound(args.source_id.clone()),
        ));
    };

    let Some(table) = source.as_any().downcast_ref::<PostgresTableSource>() else {
        return Ok(tool_error(format!(
            "source '{}' is not a PostgreSQL table source",
            args.source_id
        )));
    };

    let expr: Expr = match cql2::parse_text(args.cql2.trim()) {
        Ok(e) => e,
        Err(e) => return Ok(tool_error(format!("CQL2-text parse failed: {e}"))),
    };

    let sql = match expr.to_sql() {
        Ok(s) => s,
        Err(e) => return Ok(tool_error(format!("CQL2 -> SQL translation failed: {e}"))),
    };

    let limit = args
        .limit
        .unwrap_or(CQL2_QUERY_DEFAULT_LIMIT)
        .clamp(1, 1000);

    let (features, matched) = match table
        .query_features_geojson(
            args.bbox,
            4326,
            4326,
            Some(sql.as_str()),
            Some(4326),
            limit,
            0,
        )
        .await
    {
        Ok(v) => v,
        Err(e) => return Ok(tile_error_to_call_result(e)),
    };

    let fc = serde_json::json!({
        "type": "FeatureCollection",
        "features": features,
        "numberMatched": matched,
        "numberReturned": features.len(),
    });
    match Content::json(&fc) {
        Ok(c) => Ok(CallToolResult::success(vec![c])),
        Err(e) => Ok(tile_error_to_call_result(
            crate::error::TileServerError::Internal(anyhow::anyhow!(
                "failed to serialize FeatureCollection: {e}"
            )),
        )),
    }
}

#[cfg(feature = "stac")]
fn stac_search(
    state: &Arc<AppState>,
    args: &SearchStacItemsArgs,
) -> Result<CallToolResult, McpError> {
    use crate::sources::stac::StacSource;

    let Some(source) = state.sources.get(&args.source_id) else {
        return Ok(tile_error_to_call_result(
            crate::error::TileServerError::SourceNotFound(args.source_id.clone()),
        ));
    };

    let Some(stac_source) = source.as_any().downcast_ref::<StacSource>() else {
        return Ok(tool_error(format!(
            "source '{}' is not a STAC source",
            args.source_id
        )));
    };

    let limit_usize = args
        .limit
        .unwrap_or(STAC_SEARCH_DEFAULT_LIMIT)
        .clamp(1, 1000) as usize;

    let bbox_filter = args.bbox;
    let datetime_filter = args.datetime.as_deref();

    let filtered: Vec<_> = stac_source
        .discovered_assets()
        .iter()
        .filter(|asset| match bbox_filter {
            None => true,
            Some([min_lon, min_lat, max_lon, max_lat]) => {
                let [a_min_lon, a_min_lat, a_max_lon, a_max_lat] = asset.bbox;
                !(a_max_lon < min_lon
                    || a_min_lon > max_lon
                    || a_max_lat < min_lat
                    || a_min_lat > max_lat)
            }
        })
        .filter(|asset| match (datetime_filter, asset.datetime.as_deref()) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(filter), Some(asset_dt)) => asset_dt.starts_with(filter) || asset_dt == filter,
        })
        .take(limit_usize)
        .collect();

    let fc = serde_json::json!({
        "type": "FeatureCollection",
        "features": filtered.iter().map(|asset| serde_json::json!({
            "type": "Feature",
            "id": asset.id,
            "bbox": asset.bbox,
            "properties": {
                "title": asset.title,
                "datetime": asset.datetime,
                "cloud_cover": asset.cloud_cover,
                "href": asset.href,
            },
            "geometry": null,
        })).collect::<Vec<_>>(),
        "numberReturned": filtered.len(),
    });
    match Content::json(&fc) {
        Ok(c) => Ok(CallToolResult::success(vec![c])),
        Err(e) => Ok(tile_error_to_call_result(
            crate::error::TileServerError::Internal(anyhow::anyhow!(
                "failed to serialize STAC FeatureCollection: {e}"
            )),
        )),
    }
}
