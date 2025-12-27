use axum::{
    extract::{Path, Query, State},
    http::{
        header::{ACCEPT, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE},
        HeaderMap, HeaderValue, Method, StatusCode, Uri,
    },
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use rust_embed::Embed;
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod cache_control;
mod cli;
mod config;
mod error;
mod render;
mod sources;
mod styles;
mod telemetry;
mod wmts;

use cli::Cli;
use config::Config;
use error::TileServerError;
use render::{ImageFormat, RenderOptions, Renderer, StaticQueryParams, StaticType};
use sources::{SourceManager, TileJson};
use styles::{StyleInfo, StyleManager};

/// Embedded SPA assets (built from apps/client)
#[derive(Embed)]
#[folder = "apps/client/.output/public"]
struct Assets;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub sources: Arc<SourceManager>,
    pub styles: Arc<StyleManager>,
    pub renderer: Option<Arc<Renderer>>,
    pub base_url: String,
    pub ui_enabled: bool,
    pub fonts_dir: Option<PathBuf>,
    pub files_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // Parse CLI arguments
    let cli = Cli::parse_args();
    let ui_enabled = cli.ui_enabled();
    let verbose = cli.verbose;

    // Load configuration early to get telemetry settings
    let mut config = Config::load(cli.config)?;

    // Initialize tracing with OpenTelemetry
    let filter = if verbose {
        EnvFilter::from_default_env().add_directive("tileserver_rs=debug".parse()?)
    } else {
        EnvFilter::from_default_env()
    };

    let fmt_layer = tracing_subscriber::fmt::layer().compact();

    let registry = tracing_subscriber::registry().with(filter).with(fmt_layer);

    // Add OpenTelemetry layer if enabled
    if let Some(otel_layer) = telemetry::init_telemetry(&config.telemetry) {
        registry.with(otel_layer).init();
    } else {
        registry.init();
    }

    // Override with CLI arguments
    if let Some(host) = cli.host {
        config.server.host = host;
    }
    if let Some(port) = cli.port {
        config.server.port = port;
    }

    // Load tile sources
    let sources = SourceManager::from_configs(&config.sources).await?;
    tracing::info!("Loaded {} tile source(s)", sources.len());

    // Load styles
    let styles = StyleManager::from_configs(&config.styles)?;
    tracing::info!("Loaded {} style(s)", styles.len());

    // Initialize native renderer for rendering (if styles are configured)
    let renderer = if !styles.is_empty() {
        match Renderer::new() {
            Ok(r) => {
                tracing::info!("Native MapLibre renderer initialized");
                Some(Arc::new(r))
            }
            Err(e) => {
                tracing::warn!("Failed to initialize renderer: {}. Rendering disabled.", e);
                None
            }
        }
    } else {
        None
    };

    // Build base URL
    // Convert 0.0.0.0 to localhost for a valid fetchable URL
    let host_for_url = if config.server.host == "0.0.0.0" {
        "localhost"
    } else {
        &config.server.host
    };
    let base_url = format!("http://{}:{}", host_for_url, config.server.port);

    // Log fonts directory if configured
    if let Some(ref fonts_path) = config.fonts {
        if fonts_path.exists() {
            tracing::info!("Fonts directory: {}", fonts_path.display());
        } else {
            tracing::warn!("Fonts directory not found: {}", fonts_path.display());
        }
    }

    // Log files directory if configured
    if let Some(ref files_path) = config.files {
        if files_path.exists() {
            tracing::info!("Files directory: {}", files_path.display());
        } else {
            tracing::warn!("Files directory not found: {}", files_path.display());
        }
    }

    let state = AppState {
        sources: Arc::new(sources),
        styles: Arc::new(styles),
        renderer,
        base_url,
        ui_enabled,
        fonts_dir: config.fonts,
        files_dir: config.files,
    };

    if ui_enabled {
        tracing::info!("Web UI enabled at /");
    } else {
        tracing::info!("Web UI disabled (use --ui to enable)");
    }

    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_headers([ACCEPT, CONTENT_TYPE])
        .max_age(Duration::from_secs(86400))
        .allow_origin(
            config
                .server
                .cors_origins
                .first()
                .unwrap_or(&"*".to_string())
                .parse::<HeaderValue>()?,
        )
        .allow_methods([Method::GET, Method::OPTIONS, Method::HEAD]);

    // Build router
    let mut router = Router::new().merge(api_router(state.clone()));

    // Add embedded SPA if UI is enabled
    if ui_enabled {
        router = router.fallback(serve_spa);
    }

    let router = router
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    tracing::info!("Starting tileserver on http://{}", addr);

    let listener = TcpListener::bind(addr).await?;

    // Run the server with graceful shutdown
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Shutdown OpenTelemetry
    telemetry::shutdown_telemetry();

    Ok(())
}

/// Signal handler for graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}

/// Serve embedded SPA assets
async fn serve_spa(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try to serve the exact file
    if let Some(content) = Assets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).unwrap());

        // Cache static assets (hashed files) for 1 year
        if path.starts_with("_nuxt/") {
            headers.insert(
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
        }

        return (headers, content.data.to_vec()).into_response();
    }

    // For SPA routing, serve index.html for non-file paths
    if let Some(index) = Assets::get("index.html") {
        return Html(index.data.to_vec()).into_response();
    }

    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/index.json", get(get_index_json))
        // Style endpoints
        .route("/styles.json", get(get_all_styles))
        .route("/styles/{style_json}", get(get_style_tilejson))
        .route("/styles/{style}/style.json", get(get_style_json))
        .route("/styles/{style}/wmts.xml", get(get_wmts_capabilities))
        .route("/styles/{style}/{sprite_file}", get(get_sprite))
        .route("/styles/{style}/{z}/{x}/{y_fmt}", get(get_raster_tile))
        .route(
            "/styles/{style}/{tile_size}/{z}/{x}/{y_fmt}",
            get(get_raster_tile_with_size),
        )
        .route(
            "/styles/{style}/static/{static_type}/{size_fmt}",
            get(get_static_image),
        )
        // Font endpoints
        .route("/fonts.json", get(get_fonts_list))
        .route("/fonts/{fontstack}/{range}", get(get_font_glyphs))
        // Data endpoints
        .route("/data.json", get(get_all_sources))
        .route("/data/{source}", get(get_source_tilejson))
        .route("/data/{source}/{z}/{x}/{y_fmt}", get(get_tile))
        .route("/data/{source}/{z}/{x}/{y}.geojson", get(get_tile_geojson))
        // Static files endpoint
        .route("/files/{*filepath}", get(get_static_file))
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}

/// Combined index entry for /index.json
#[derive(serde::Serialize)]
#[serde(untagged)]
enum IndexEntry {
    Data(TileJson),
    Style(RasterTileJson),
}

/// Get combined TileJSON array for all data sources and styles
/// Route: GET /index.json
async fn get_index_json(State(state): State<AppState>) -> Json<Vec<IndexEntry>> {
    let mut entries = Vec::new();

    // Add all data sources
    for metadata in state.sources.all_metadata() {
        entries.push(IndexEntry::Data(metadata.to_tilejson(&state.base_url)));
    }

    // Add all styles as raster tile sources
    for style in state.styles.all() {
        let tile_url = format!(
            "{}/styles/{}/{{z}}/{{x}}/{{y}}.png",
            state.base_url, style.id
        );
        entries.push(IndexEntry::Style(RasterTileJson {
            tilejson: "3.0.0",
            name: style.name.clone(),
            tiles: vec![tile_url],
            minzoom: 0,
            maxzoom: 22,
            attribution: None,
        }));
    }

    Json(entries)
}

/// Get all available styles
async fn get_all_styles(State(state): State<AppState>) -> Json<Vec<StyleInfo>> {
    Json(state.styles.all_infos(&state.base_url))
}

/// Get style.json for a specific style
async fn get_style_json(
    State(state): State<AppState>,
    Path(style_id): Path<String>,
) -> Result<Json<serde_json::Value>, TileServerError> {
    let style = state
        .styles
        .get(&style_id)
        .ok_or_else(|| TileServerError::StyleNotFound(style_id))?;

    Ok(Json(style.style_json.clone()))
}

/// TileJSON response for raster style tiles
#[derive(serde::Serialize)]
struct RasterTileJson {
    tilejson: &'static str,
    name: String,
    tiles: Vec<String>,
    minzoom: u8,
    maxzoom: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    attribution: Option<String>,
}

/// Get TileJSON for raster tiles of a style
/// Route: GET /styles/{style}.json
async fn get_style_tilejson(
    State(state): State<AppState>,
    Path(style_json): Path<String>,
) -> Result<Json<RasterTileJson>, TileServerError> {
    // Only handle requests ending with .json
    let style_id = style_json
        .strip_suffix(".json")
        .ok_or_else(|| TileServerError::StyleNotFound(style_json.clone()))?;

    let style = state
        .styles
        .get(style_id)
        .ok_or_else(|| TileServerError::StyleNotFound(style_id.to_string()))?;

    // Build raster tile URL template
    let tile_url = format!(
        "{}/styles/{}/{{z}}/{{x}}/{{y}}.png",
        state.base_url, style_id
    );

    Ok(Json(RasterTileJson {
        tilejson: "3.0.0",
        name: style.name.clone(),
        tiles: vec![tile_url],
        minzoom: 0,
        maxzoom: 22,
        attribution: None,
    }))
}

/// Get all available tile sources
async fn get_all_sources(State(state): State<AppState>) -> Json<Vec<TileJson>> {
    let sources: Vec<TileJson> = state
        .sources
        .all_metadata()
        .iter()
        .map(|m| m.to_tilejson(&state.base_url))
        .collect();

    Json(sources)
}

/// Get TileJSON for a specific source
async fn get_source_tilejson(
    State(state): State<AppState>,
    Path(source): Path<String>,
) -> Result<Json<TileJson>, TileServerError> {
    // Strip .json extension if present
    let source_id = source.strip_suffix(".json").unwrap_or(&source);

    let source_ref = state
        .sources
        .get(source_id)
        .ok_or_else(|| TileServerError::SourceNotFound(source_id.to_string()))?;

    let tilejson = source_ref.metadata().to_tilejson(&state.base_url);
    Ok(Json(tilejson))
}

/// Tile request parameters (raw from URL)
#[derive(serde::Deserialize)]
struct TileParams {
    source: String,
    z: u8,
    x: u32,
    y_fmt: String, // e.g., "123.pbf" or "123.mvt"
}

impl TileParams {
    /// Parse y coordinate and format from "123.pbf" style string
    fn parse_y_and_format(&self) -> Option<(u32, &str)> {
        let (y_str, format) = self.y_fmt.rsplit_once('.')?;
        let y = y_str.parse().ok()?;
        Some((y, format))
    }
}

/// Get a tile from a source
async fn get_tile(
    State(state): State<AppState>,
    Path(params): Path<TileParams>,
) -> Result<Response, TileServerError> {
    let (y, _format) = params
        .parse_y_and_format()
        .ok_or(TileServerError::InvalidTileRequest)?;

    let source = state
        .sources
        .get(&params.source)
        .ok_or_else(|| TileServerError::SourceNotFound(params.source.clone()))?;

    let tile =
        source
            .get_tile(params.z, params.x, y)
            .await?
            .ok_or(TileServerError::TileNotFound {
                z: params.z,
                x: params.x,
                y,
            })?;

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(tile.format.content_type()),
    );
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());

    // Add content-encoding if tile is compressed
    if let Some(encoding) = tile.compression.content_encoding() {
        headers.insert(CONTENT_ENCODING, HeaderValue::from_static(encoding));
    }

    Ok((headers, tile.data).into_response())
}

/// GeoJSON tile request parameters
#[derive(serde::Deserialize)]
struct GeoJsonTileParams {
    source: String,
    z: u8,
    x: u32,
    y: u32,
}

/// Get a tile as GeoJSON (converts PBF to GeoJSON)
/// Route: GET /data/{source}/{z}/{x}/{y}.geojson
async fn get_tile_geojson(
    State(state): State<AppState>,
    Path(params): Path<GeoJsonTileParams>,
) -> Result<Response, TileServerError> {
    use flate2::read::GzDecoder;
    use geozero::mvt::{Message, Tile};
    use geozero::ProcessToJson;
    use sources::TileCompression;
    use std::io::Read;

    let source = state
        .sources
        .get(&params.source)
        .ok_or_else(|| TileServerError::SourceNotFound(params.source.clone()))?;

    // Check if source is vector format
    if source.metadata().format != sources::TileFormat::Pbf {
        return Err(TileServerError::RenderError(
            "GeoJSON conversion only supported for vector tiles (PBF)".to_string(),
        ));
    }

    let tile = source.get_tile(params.z, params.x, params.y).await?.ok_or(
        TileServerError::TileNotFound {
            z: params.z,
            x: params.x,
            y: params.y,
        },
    )?;

    // Decompress if needed
    let raw_data = match tile.compression {
        TileCompression::Gzip => {
            let mut decoder = GzDecoder::new(&tile.data[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).map_err(|e| {
                TileServerError::RenderError(format!("Failed to decompress tile: {}", e))
            })?;
            decompressed
        }
        TileCompression::None => tile.data.to_vec(),
        _ => {
            return Err(TileServerError::RenderError(format!(
                "Unsupported compression: {:?}",
                tile.compression
            )));
        }
    };

    // Parse MVT tile using prost
    let mvt_tile = Tile::decode(raw_data.as_slice())
        .map_err(|e| TileServerError::RenderError(format!("Failed to decode MVT tile: {}", e)))?;

    // Convert each layer to GeoJSON and combine into a FeatureCollection
    let mut all_features: Vec<serde_json::Value> = Vec::new();

    for mut layer in mvt_tile.layers {
        // Each layer implements GeozeroDatasource which can convert to JSON
        if let Ok(layer_json) = layer.to_json() {
            // Parse the layer GeoJSON (it's a FeatureCollection)
            if let Ok(fc) = serde_json::from_str::<serde_json::Value>(&layer_json) {
                if let Some(features) = fc.get("features").and_then(|f| f.as_array()) {
                    // Add layer name to each feature's properties
                    for feature in features {
                        let mut feature = feature.clone();
                        if let Some(props) = feature.get_mut("properties") {
                            if let Some(props_obj) = props.as_object_mut() {
                                props_obj.insert(
                                    "_layer".to_string(),
                                    serde_json::Value::String(layer.name.clone()),
                                );
                            }
                        }
                        all_features.push(feature);
                    }
                }
            }
        }
    }

    // Build final FeatureCollection
    let geojson = serde_json::json!({
        "type": "FeatureCollection",
        "features": all_features
    });

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/geo+json"),
    );
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());

    Ok((headers, geojson.to_string()).into_response())
}

/// Raster tile request parameters
#[derive(serde::Deserialize)]
struct RasterTileParams {
    style: String,
    z: u8,
    x: u32,
    y_fmt: String, // e.g., "123.png" or "123@2x.webp"
}

impl RasterTileParams {
    /// Parse y, scale, and format from "123@2x.png" style string
    fn parse(&self) -> Option<(u32, u8, ImageFormat)> {
        // Split extension first: "123@2x" and "png"
        let (y_and_scale, format_str) = self.y_fmt.rsplit_once('.')?;

        let format = ImageFormat::from_str(format_str)?;

        // Check for scale: "123@2x" or just "123"
        if let Some((y_str, scale_str)) = y_and_scale.split_once('@') {
            let y = y_str.parse().ok()?;
            // Parse scale like "2x" -> 2
            let scale = scale_str.strip_suffix('x')?.parse().ok()?;
            // Validate scale range (1-9)
            if (1..=9).contains(&scale) {
                Some((y, scale, format))
            } else {
                None
            }
        } else {
            // No scale, default to 1
            let y = y_and_scale.parse().ok()?;
            Some((y, 1, format))
        }
    }
}

/// Get a raster tile (rendered from style)
/// Route: GET /styles/{style}/{z}/{x}/{y}[@{scale}x].{format}
async fn get_raster_tile(
    State(state): State<AppState>,
    Path(params): Path<RasterTileParams>,
) -> Result<Response, TileServerError> {
    // Check if rendering is available
    let renderer = state
        .renderer
        .as_ref()
        .ok_or_else(|| TileServerError::RenderError("Rendering not available".to_string()))?;

    // Parse parameters
    let (y, scale, format) = params.parse().ok_or(TileServerError::InvalidTileRequest)?;

    // Get style
    let style = state
        .styles
        .get(&params.style)
        .ok_or_else(|| TileServerError::StyleNotFound(params.style.clone()))?;

    // Rewrite style to inline tile URLs for native rendering
    let rewritten_style =
        styles::rewrite_style_for_native(&style.style_json, &state.base_url, &state.sources);

    // Render the tile
    let image_data = renderer
        .render_tile(
            &rewritten_style.to_string(),
            params.z,
            params.x,
            y,
            scale,
            format,
        )
        .await?;

    // Build response
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(format.content_type()),
    );
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());

    Ok((headers, image_data).into_response())
}

/// Raster tile request parameters with variable tile size
#[derive(serde::Deserialize)]
struct RasterTileWithSizeParams {
    style: String,
    tile_size: u16, // e.g., 256 or 512
    z: u8,
    x: u32,
    y_fmt: String, // e.g., "123.png" or "123@2x.webp"
}

impl RasterTileWithSizeParams {
    /// Parse y, scale, and format from "123@2x.png" style string
    fn parse(&self) -> Option<(u32, u8, ImageFormat)> {
        // Split extension first: "123@2x" and "png"
        let (y_and_scale, format_str) = self.y_fmt.rsplit_once('.')?;

        let format = ImageFormat::from_str(format_str)?;

        // Check for scale: "123@2x" or just "123"
        if let Some((y_str, scale_str)) = y_and_scale.split_once('@') {
            let y = y_str.parse().ok()?;
            // Parse scale like "2x" -> 2
            let scale = scale_str.strip_suffix('x')?.parse().ok()?;
            // Validate scale range (1-9)
            if (1..=9).contains(&scale) {
                Some((y, scale, format))
            } else {
                None
            }
        } else {
            // No scale, default to 1
            let y = y_and_scale.parse().ok()?;
            Some((y, 1, format))
        }
    }
}

/// Get a raster tile with variable tile size
/// Route: GET /styles/{style}/{tile_size}/{z}/{x}/{y}[@{scale}x].{format}
async fn get_raster_tile_with_size(
    State(state): State<AppState>,
    Path(params): Path<RasterTileWithSizeParams>,
) -> Result<Response, TileServerError> {
    // Validate tile size (only 256 and 512 are supported)
    if params.tile_size != 256 && params.tile_size != 512 {
        return Err(TileServerError::RenderError(format!(
            "Invalid tile size: {}. Only 256 and 512 are supported.",
            params.tile_size
        )));
    }

    // Check if rendering is available
    let renderer = state
        .renderer
        .as_ref()
        .ok_or_else(|| TileServerError::RenderError("Rendering not available".to_string()))?;

    // Parse parameters
    let (y, additional_scale, format) =
        params.parse().ok_or(TileServerError::InvalidTileRequest)?;

    // Calculate effective scale
    // For 512px tiles, we use scale=2 (renders at 512px)
    // For 256px tiles, we use scale=1 (renders at 256px)
    // Additional scale from URL (e.g., @2x) multiplies on top
    let base_scale: u8 = if params.tile_size == 512 { 2 } else { 1 };
    let effective_scale = base_scale * additional_scale;

    // Clamp to valid range
    let scale = effective_scale.min(9);

    // Get style
    let style = state
        .styles
        .get(&params.style)
        .ok_or_else(|| TileServerError::StyleNotFound(params.style.clone()))?;

    // Rewrite style to inline tile URLs for native rendering
    let rewritten_style =
        styles::rewrite_style_for_native(&style.style_json, &state.base_url, &state.sources);

    // Render the tile
    let image_data = renderer
        .render_tile(
            &rewritten_style.to_string(),
            params.z,
            params.x,
            y,
            scale,
            format,
        )
        .await?;

    // Build response
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(format.content_type()),
    );
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());

    Ok((headers, image_data).into_response())
}

/// Static image request parameters
#[derive(serde::Deserialize)]
struct StaticImageParams {
    style: String,
    static_type: String, // e.g., "-122.4,37.8,12" or "auto"
    size_fmt: String,    // e.g., "800x600.png" or "800x600@2x.webp"
}

impl StaticImageParams {
    /// Parse size, scale, and format from "800x600@2x.png" style string
    fn parse(&self) -> Option<(u32, u32, u8, ImageFormat)> {
        // Split extension: "800x600@2x" and "png"
        let (size_and_scale, format_str) = self.size_fmt.rsplit_once('.')?;

        let format = ImageFormat::from_str(format_str)?;

        // Check for scale: "800x600@2x" or just "800x600"
        let (size_str, scale) = if let Some((size, scale_str)) = size_and_scale.split_once('@') {
            let scale = scale_str.strip_suffix('x')?.parse().ok()?;
            if !(1..=9).contains(&scale) {
                return None;
            }
            (size, scale)
        } else {
            (size_and_scale, 1)
        };

        // Parse width and height: "800x600"
        let (width_str, height_str) = size_str.split_once('x')?;
        let width = width_str.parse().ok()?;
        let height = height_str.parse().ok()?;

        Some((width, height, scale, format))
    }
}

/// Get a static image
/// Route: GET /styles/{style}/static/{static_type}/{width}x{height}[@{scale}x].{format}
async fn get_static_image(
    State(state): State<AppState>,
    Path(params): Path<StaticImageParams>,
    Query(query): Query<StaticQueryParams>,
) -> Result<Response, TileServerError> {
    // Check if rendering is available
    let renderer = state
        .renderer
        .as_ref()
        .ok_or_else(|| TileServerError::RenderError("Rendering not available".to_string()))?;

    // Parse parameters
    let (width, height, scale, format) = params.parse().ok_or_else(|| {
        TileServerError::RenderError(format!("Invalid size format: {}", params.size_fmt))
    })?;

    // Parse static type
    let static_type =
        StaticType::from_str(&params.static_type).map_err(TileServerError::RenderError)?;

    // Get style
    let style = state
        .styles
        .get(&params.style)
        .ok_or_else(|| TileServerError::StyleNotFound(params.style.clone()))?;

    // Rewrite style to inline tile URLs for native rendering
    let rewritten_style =
        styles::rewrite_style_for_native(&style.style_json, &state.base_url, &state.sources);

    // Create render options
    let options = RenderOptions::for_static(
        params.style.clone(),
        rewritten_style.to_string(),
        static_type,
        width,
        height,
        scale,
        format,
        query,
    )
    .map_err(TileServerError::RenderError)?;

    // Render static image
    let image_data = renderer.render_static(options).await?;

    // Build response
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(format.content_type()),
    );
    // Cache static images for 1 hour
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );

    Ok((headers, image_data).into_response())
}

/// Sprite request parameters
#[derive(serde::Deserialize)]
struct SpriteParams {
    style: String,
    sprite_file: String, // e.g., "sprite.png", "sprite@2x.json", "sprite.json"
}

/// Get sprite image or metadata for a style
/// Route: GET /styles/{style}/sprite[@{scale}x].{format}
async fn get_sprite(
    State(state): State<AppState>,
    Path(params): Path<SpriteParams>,
) -> Result<Response, TileServerError> {
    // Only handle sprite files
    if !params.sprite_file.starts_with("sprite") {
        return Err(TileServerError::InvalidTileRequest);
    }

    // Get style to find its directory
    let style = state
        .styles
        .get(&params.style)
        .ok_or_else(|| TileServerError::StyleNotFound(params.style.clone()))?;

    // Get the style directory (parent of style.json)
    let style_dir = style
        .path
        .parent()
        .ok_or_else(|| TileServerError::StyleNotFound(params.style.clone()))?;

    // Build path to sprite file
    let sprite_path = style_dir.join(&params.sprite_file);

    // Read the sprite file
    let data = tokio::fs::read(&sprite_path).await.map_err(|e| {
        tracing::debug!("Sprite file not found: {} ({})", sprite_path.display(), e);
        TileServerError::SpriteNotFound(params.sprite_file.clone())
    })?;

    // Determine content type
    let content_type = if params.sprite_file.ends_with(".json") {
        "application/json"
    } else if params.sprite_file.ends_with(".png") {
        "image/png"
    } else {
        "application/octet-stream"
    };

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());

    Ok((headers, data).into_response())
}

/// Get WMTS GetCapabilities document for a style
/// Route: GET /styles/{style}/wmts.xml
async fn get_wmts_capabilities(
    State(state): State<AppState>,
    Path(style_id): Path<String>,
) -> Result<Response, TileServerError> {
    // Get style
    let style = state
        .styles
        .get(&style_id)
        .ok_or_else(|| TileServerError::StyleNotFound(style_id.clone()))?;

    // Generate WMTS capabilities XML
    let xml = wmts::generate_wmts_capabilities(
        &state.base_url,
        &style_id,
        &style.name,
        0,  // minzoom
        22, // maxzoom
    );

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/xml"));
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );

    Ok((headers, xml).into_response())
}

/// Get list of available fonts
/// Route: GET /fonts.json
async fn get_fonts_list(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, TileServerError> {
    let fonts_dir = match &state.fonts_dir {
        Some(dir) => dir,
        None => return Ok(Json(Vec::new())),
    };

    let mut fonts = Vec::new();

    // Read the fonts directory to find font families
    // Each subdirectory is a font family (e.g., "Noto Sans Regular")
    if let Ok(mut entries) = tokio::fs::read_dir(fonts_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(file_type) = entry.file_type().await {
                if file_type.is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        // Only include directories that have at least one .pbf file
                        let font_dir = entry.path();
                        if has_pbf_files(&font_dir).await {
                            fonts.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    // Sort alphabetically for consistent output
    fonts.sort();

    Ok(Json(fonts))
}

/// Check if a directory contains at least one .pbf file
async fn has_pbf_files(dir: &std::path::Path) -> bool {
    if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".pbf") {
                    return true;
                }
            }
        }
    }
    false
}

/// Font glyph request parameters
#[derive(serde::Deserialize)]
struct FontParams {
    fontstack: String, // e.g., "Noto Sans Regular" or "Open Sans Bold,Arial Unicode MS Regular"
    range: String,     // e.g., "0-255.pbf"
}

/// Get font glyphs (PBF format)
/// Route: GET /fonts/{fontstack}/{start}-{end}.pbf
async fn get_font_glyphs(
    State(state): State<AppState>,
    Path(params): Path<FontParams>,
) -> Result<Response, TileServerError> {
    // Check if fonts directory is configured
    let fonts_dir = state.fonts_dir.as_ref().ok_or_else(|| {
        TileServerError::FontNotFound("Fonts directory not configured".to_string())
    })?;

    // Parse the range to ensure it's valid (e.g., "0-255.pbf")
    if !params.range.ends_with(".pbf") {
        return Err(TileServerError::InvalidTileRequest);
    }

    // Font stacks are comma-separated, try each font in order
    let fonts: Vec<&str> = params.fontstack.split(',').map(|s| s.trim()).collect();

    for font_name in &fonts {
        let font_path = fonts_dir.join(font_name).join(&params.range);

        if let Ok(data) = tokio::fs::read(&font_path).await {
            let mut headers = HeaderMap::new();
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-protobuf"),
            );
            headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());

            tracing::debug!("Serving font: {}/{}", font_name, params.range);
            return Ok((headers, data).into_response());
        }
    }

    // No font found in the stack
    tracing::debug!("Font not found: {} (tried: {:?})", params.range, fonts);
    Err(TileServerError::FontNotFound(params.fontstack))
}

/// Get a static file from the files directory
/// Route: GET /files/{*filepath}
async fn get_static_file(
    State(state): State<AppState>,
    Path(filepath): Path<String>,
) -> Result<Response, TileServerError> {
    // Check if files directory is configured
    let files_dir = state
        .files_dir
        .as_ref()
        .ok_or_else(|| TileServerError::NotFound("Files directory not configured".to_string()))?;

    // Sanitize the filepath to prevent directory traversal attacks
    let filepath = filepath.trim_start_matches('/');
    if filepath.contains("..") || filepath.starts_with('/') {
        return Err(TileServerError::NotFound("Invalid file path".to_string()));
    }

    let file_path = files_dir.join(filepath);

    // Ensure the resolved path is still within the files directory
    let canonical_files_dir = files_dir
        .canonicalize()
        .map_err(|_| TileServerError::NotFound("Files directory not accessible".to_string()))?;
    let canonical_file_path = file_path
        .canonicalize()
        .map_err(|_| TileServerError::NotFound(format!("File not found: {}", filepath)))?;

    if !canonical_file_path.starts_with(&canonical_files_dir) {
        return Err(TileServerError::NotFound("Invalid file path".to_string()));
    }

    // Read the file
    let data = tokio::fs::read(&canonical_file_path)
        .await
        .map_err(|_| TileServerError::NotFound(format!("File not found: {}", filepath)))?;

    // Determine content type from extension
    let content_type = mime_guess::from_path(&canonical_file_path)
        .first_or_octet_stream()
        .to_string();

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&content_type)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    // Cache static files for 1 hour
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );

    Ok((headers, data).into_response())
}
