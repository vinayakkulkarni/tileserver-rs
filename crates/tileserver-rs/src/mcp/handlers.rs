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
    CallToolResult, ContentBlock, GetPromptRequestParams, GetPromptResponse, ListPromptsResult,
    ListResourceTemplatesResult, PaginatedRequestParams, ProtocolVersion,
    ReadResourceRequestParams, ReadResourceResponse, ServerCapabilities, ServerConfig,
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

/// Input parameters for `McpHandler::tileserver_get_source_tilejson`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetSourceTilejsonArgs {
    /// Source id as registered in `[[sources]]` configuration. Must match
    /// one of the ids returned by `tileserver_list_sources`.
    pub source_id: String,
}

/// Input parameters for `McpHandler::tileserver_get_style`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetStyleArgs {
    /// Style id as registered in `[[styles]]` configuration. Must match
    /// one of the ids returned by `tileserver_list_styles`.
    pub style_id: String,
}

/// Input parameters for `McpHandler::tileserver_get_tile_metadata`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetTileMetadataArgs {
    /// Source id whose `TileMetadata` (bounds, zoom range, format) to return.
    pub source_id: String,
}

/// Input parameters for `McpHandler::tileserver_render_static_map`.
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

/// Input parameters for `McpHandler::tileserver_get_tile`.
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

/// Input parameters for `McpHandler::tileserver_query_features_at_point`.
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

/// Input parameters for `McpHandler::tileserver_query_features_cql2`.
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

/// Input parameters for `McpHandler::tileserver_search_stac_items`.
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

    /// `tileserver_render_static_map`. `Err` carries the tool-error result.
    async fn render_static_map(
        &self,
        args: RenderStaticMapArgs,
    ) -> Result<CallToolResult, ToolFailure> {
        let renderer = self.state.renderer.as_ref().ok_or_else(|| {
            fail(tool_error(
                "native renderer is disabled; rebuild with --features raster and set render.pool_size > 0",
            ))
        })?;
        check_render_dimensions(args.width, args.height)?;
        let style = self.state.styles.get(&args.style_id).ok_or_else(|| {
            fail(tile_error_to_call_result(
                crate::error::TileServerError::StyleNotFound(args.style_id.clone()),
            ))
        })?;
        let style_json = self.render_style_json(&style.style_json)?;
        let render_options = RenderOptions::for_static(
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
        )
        .map_err(|e| fail(tool_error(format!("invalid render options: {e}"))))?;
        let bytes = renderer
            .render_static(render_options)
            .await
            .map_err(|e| fail(tile_error_to_call_result(e)))?;
        webp_image_result(&bytes)
    }

    /// The style with tile URLs pointing at this server, as JSON text.
    fn render_style_json(&self, style: &serde_json::Value) -> Result<String, ToolFailure> {
        let value = crate::styles::rewrite_style_for_api(
            style,
            &self.state.render_base_url,
            &crate::styles::UrlQueryParams::default(),
            &self.state.sources,
        );
        if !(value.is_object() || value.is_array()) {
            return Err(fail(tool_error(
                "rewritten style is neither an object nor an array",
            )));
        }
        serde_json::to_string(&value).map_err(|e| {
            fail(tile_error_to_call_result(
                crate::error::TileServerError::Internal(anyhow::anyhow!(
                    "failed to serialize rewritten style: {e}"
                )),
            ))
        })
    }
}

/// A tool-level failure, boxed: `CallToolResult` is too large to return by
/// value in an `Err`.
type ToolFailure = Box<CallToolResult>;

fn fail(result: CallToolResult) -> ToolFailure {
    Box::new(result)
}

/// Both sides in `1..=MAX_RENDER_DIMENSION`.
fn check_render_dimensions(width: u32, height: u32) -> Result<(), ToolFailure> {
    if width == 0 || height == 0 {
        return Err(fail(tool_error(
            "width and height must be greater than zero",
        )));
    }
    if width > MAX_RENDER_DIMENSION || height > MAX_RENDER_DIMENSION {
        return Err(fail(tool_error(format!(
            "image dimensions exceed MCP cap (max {MAX_RENDER_DIMENSION}x{MAX_RENDER_DIMENSION}); got {width}x{height}"
        ))));
    }
    Ok(())
}

/// Base64 WebP image content, refused above [`MAX_RENDER_BYTES`].
fn webp_image_result(bytes: &[u8]) -> Result<CallToolResult, ToolFailure> {
    if bytes.len() > MAX_RENDER_BYTES {
        return Err(fail(tool_error(format!(
            "rendered image is {} bytes; exceeds MCP soft cap of {MAX_RENDER_BYTES} bytes \u{2014} request smaller dimensions",
            bytes.len()
        ))));
    }
    Ok(CallToolResult::success(vec![ContentBlock::image(
        BASE64_STANDARD.encode(bytes),
        "image/webp",
    )]))
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
        match ContentBlock::json(&tilejsons) {
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
        match ContentBlock::json(&tj) {
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
        match ContentBlock::json(&infos) {
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
        match ContentBlock::json(&style.style_json) {
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
        match ContentBlock::json(source.metadata()) {
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
        match ContentBlock::json(&payload) {
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
        Ok(self
            .render_static_map(args)
            .await
            .unwrap_or_else(|failure| *failure))
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
        match ContentBlock::json(&summary) {
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
    fn get_info(&self) -> ServerConfig {
        let capabilities = ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_prompts()
            .build();
        ServerConfig::new(capabilities)
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
    ) -> Result<ReadResourceResponse, McpError> {
        Ok(read_resource(&request.uri, &self.state)?.into())
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
    ) -> Result<GetPromptResponse, McpError> {
        Ok(get_prompt(&request)?.into())
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
    let table = match postgres_table(
        state,
        &args.source_id,
        "; feature query at point is only supported for PostgreSQL table sources currently",
    ) {
        Ok(t) => t,
        Err(failure) => return Ok(*failure),
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
    Ok(
        match table
            .query_features_geojson(Some(bbox), 4326, 4326, None, None, limit, 0)
            .await
        {
            Ok((features, _)) => feature_collection_result(&features, None),
            Err(e) => tile_error_to_call_result(e),
        },
    )
}

#[cfg(feature = "postgres")]
async fn cql2_query_postgres(
    state: &Arc<AppState>,
    args: &QueryFeaturesCql2Args,
) -> Result<CallToolResult, McpError> {
    let table = match postgres_table(state, &args.source_id, "") {
        Ok(t) => t,
        Err(failure) => return Ok(*failure),
    };
    // The same gate as the OGC `filter=` parameter: only the table's own
    // columns, and the expression must survive a parse/print round trip.
    // Translating the raw CQL2 instead would let a caller put any SQL
    // function call (for example `pg_sleep(10) = 1`) or any column into the
    // WHERE clause.
    let sql = match crate::routes::ogc_filter::translate_filter_to_sql(
        args.cql2.trim(),
        Some("cql2-text"),
        &table.table_info().properties,
    ) {
        Ok(sql) => sql,
        Err(e) => return Ok(tool_error(format!("invalid CQL2 filter: {e}"))),
    };
    let limit = args
        .limit
        .unwrap_or(CQL2_QUERY_DEFAULT_LIMIT)
        .clamp(1, 1000);
    Ok(
        match table
            .query_features_geojson(args.bbox, 4326, 4326, Some(&sql), Some(4326), limit, 0)
            .await
        {
            Ok((features, matched)) => feature_collection_result(&features, Some(matched)),
            Err(e) => tile_error_to_call_result(e),
        },
    )
}

/// The PostgreSQL table source `source_id`, or the tool error to return.
#[cfg(feature = "postgres")]
fn postgres_table<'a>(
    state: &'a AppState,
    source_id: &str,
    not_a_table_hint: &str,
) -> Result<&'a crate::sources::postgres::PostgresTableSource, ToolFailure> {
    let source = state.sources.get(source_id).ok_or_else(|| {
        fail(tile_error_to_call_result(
            crate::error::TileServerError::SourceNotFound(source_id.to_string()),
        ))
    })?;
    source
        .as_any()
        .downcast_ref::<crate::sources::postgres::PostgresTableSource>()
        .ok_or_else(|| {
            fail(tool_error(format!(
                "source '{source_id}' is not a PostgreSQL table source{not_a_table_hint}"
            )))
        })
}

/// A GeoJSON FeatureCollection as JSON tool content. `numberMatched` is
/// included when known.
#[cfg(feature = "postgres")]
fn feature_collection_result(
    features: &[serde_json::Value],
    matched: Option<i64>,
) -> CallToolResult {
    let mut fc = serde_json::json!({
        "type": "FeatureCollection",
        "features": features,
        "numberReturned": features.len(),
    });
    if let Some(matched) = matched {
        fc["numberMatched"] = matched.into();
    }
    match ContentBlock::json(&fc) {
        Ok(c) => CallToolResult::success(vec![c]),
        Err(e) => tile_error_to_call_result(crate::error::TileServerError::Internal(
            anyhow::anyhow!("failed to serialize FeatureCollection: {e}"),
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
    match ContentBlock::json(&fc) {
        Ok(c) => Ok(CallToolResult::success(vec![c])),
        Err(e) => Ok(tile_error_to_call_result(
            crate::error::TileServerError::Internal(anyhow::anyhow!(
                "failed to serialize STAC FeatureCollection: {e}"
            )),
        )),
    }
}

#[cfg(test)]
mod render_static_tests {
    use super::*;

    fn text(result: &CallToolResult) -> String {
        serde_json::to_string(result).unwrap()
    }

    #[test]
    fn dimensions_must_be_positive_and_within_the_cap() {
        assert!(check_render_dimensions(1, MAX_RENDER_DIMENSION).is_ok());
        let zero = check_render_dimensions(0, 10).unwrap_err();
        assert_eq!(zero.is_error, Some(true));
        assert!(text(&zero).contains("greater than zero"));
        let big = check_render_dimensions(MAX_RENDER_DIMENSION + 1, 10).unwrap_err();
        assert!(text(&big).contains(&format!("got {}x10", MAX_RENDER_DIMENSION + 1)));
    }

    #[test]
    fn webp_result_is_base64_image_and_refuses_oversize() {
        let ok = webp_image_result(&[1, 2, 3]).unwrap();
        let json = text(&ok);
        assert!(
            json.contains("image/webp") && json.contains("AQID"),
            "{json}"
        );
        let err = webp_image_result(&vec![0; MAX_RENDER_BYTES + 1]).unwrap_err();
        assert_eq!(err.is_error, Some(true));
        assert!(text(&err).contains("soft cap"));
    }
}

/// MCP feature queries against a live PostGIS (`DATABASE_URL`, loaded with
/// `benchmarks/postgres/init.sql`).
#[cfg(all(test, feature = "postgres-integration"))]
mod postgis_tests {
    use super::*;

    async fn state() -> Arc<AppState> {
        let url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://tileserver:tileserver@localhost:5432/tiles".into());
        let config: crate::config::PostgresConfig = toml::from_str(&format!(
            r#"
connection_string = "{url}"
[[tables]]
id = "cities"
table = "cities"
geometry_column = "geom"
id_column = "id"
srid = 4326
"#
        ))
        .unwrap();
        let sources = crate::sources::SourceManager::from_configs_with_postgres(&[], Some(&config))
            .await
            .expect("PostGIS must be reachable: set DATABASE_URL");
        Arc::new(AppState {
            sources: Arc::new(sources),
            styles: Arc::new(crate::styles::StyleManager::new()),
            renderer: None,
            base_url: "http://localhost:8080".into(),
            render_base_url: "http://127.0.0.1:8080".into(),
            ui_enabled: false,
            fonts_dir: None,
            files_dir: None,
            upload_dir: None,
        })
    }

    fn body(result: &CallToolResult) -> serde_json::Value {
        serde_json::to_value(result).unwrap()
    }

    fn cql2(source_id: &str, cql2: &str) -> QueryFeaturesCql2Args {
        serde_json::from_value(serde_json::json!({
            "source_id": source_id, "cql2": cql2, "limit": 5
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn cql2_query_filters_on_table_columns() {
        let state = state().await;
        let result = cql2_query_postgres(&state, &cql2("cities", "population > 1000000"))
            .await
            .unwrap();
        assert_ne!(result.is_error, Some(true), "{:?}", body(&result));
        let fc: serde_json::Value =
            serde_json::from_str(body(&result)["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(fc["type"], "FeatureCollection");
        assert!(fc["numberMatched"].as_i64().unwrap() > 0);
        assert!(fc["numberReturned"].as_u64().unwrap() <= 5);
    }

    #[tokio::test]
    async fn cql2_query_rejects_sql_functions_and_unknown_columns() {
        let state = state().await;
        for injection in ["pg_sleep(1) = 1", "password = 'x'"] {
            let result = cql2_query_postgres(&state, &cql2("cities", injection))
                .await
                .unwrap();
            assert_eq!(result.is_error, Some(true), "{injection} must be refused");
            assert!(
                body(&result).to_string().contains("invalid CQL2 filter"),
                "{}",
                body(&result)
            );
        }
    }

    #[tokio::test]
    async fn point_query_returns_features_near_a_city() {
        let state = state().await;
        let args: QueryFeaturesAtPointArgs = serde_json::from_value(serde_json::json!({
            "source_id": "cities", "lon": 0.0, "lat": 0.0, "radius_deg": 180.0, "limit": 3
        }))
        .unwrap();
        let result = point_query_postgres(&state, &args).await.unwrap();
        let fc: serde_json::Value =
            serde_json::from_str(body(&result)["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(fc["numberReturned"], 3);
        assert!(fc.get("numberMatched").is_none());
        let missing: QueryFeaturesAtPointArgs = serde_json::from_value(serde_json::json!({
            "source_id": "nope", "lon": 0.0, "lat": 0.0
        }))
        .unwrap();
        assert_eq!(
            point_query_postgres(&state, &missing)
                .await
                .unwrap()
                .is_error,
            Some(true)
        );
    }
}
