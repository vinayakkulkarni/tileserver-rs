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
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;

mod cache_control;
mod cli;
mod config;
mod error;
mod render;
mod sources;
mod styles;

use cli::Cli;
use config::Config;
use error::TileServerError;
use render::{BrowserPool, ImageFormat, RenderOptions, Renderer, StaticQueryParams, StaticType};
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
    pub browser_pool: Option<Arc<BrowserPool>>,
    pub base_url: String,
    pub ui_enabled: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // Parse CLI arguments
    let cli = Cli::parse_args();
    let ui_enabled = cli.ui_enabled();
    let verbose = cli.verbose;

    // Initialize tracing
    let filter = if verbose {
        EnvFilter::from_default_env().add_directive("tileserver_rs=debug".parse()?)
    } else {
        EnvFilter::from_default_env()
    };

    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(filter)
        .init();

    // Load configuration
    let mut config = Config::load(cli.config)?;

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

    // Initialize browser pool for rendering (if enabled)
    let browser_pool = if !styles.is_empty() {
        match BrowserPool::new(4).await {
            Ok(pool) => {
                tracing::info!("Browser pool initialized for rendering");
                Some(Arc::new(pool))
            }
            Err(e) => {
                tracing::warn!("Failed to initialize browser pool: {}. Rendering disabled.", e);
                None
            }
        }
    } else {
        None
    };

    // Build base URL
    let base_url = format!("http://{}:{}", config.server.host, config.server.port);

    let state = AppState {
        sources: Arc::new(sources),
        styles: Arc::new(styles),
        browser_pool,
        base_url,
        ui_enabled,
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
    axum::serve(listener, router).await?;

    Ok(())
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
        .route("/styles.json", get(get_all_styles))
        .route("/styles/{style}/style.json", get(get_style_json))
        .route("/styles/{style}/{z}/{x}/{y_fmt}", get(get_raster_tile))
        .route("/styles/{style}/static/{static_type}/{size_fmt}", get(get_static_image))
        .route("/data.json", get(get_all_sources))
        .route("/data/{source}", get(get_source_tilejson))
        .route("/data/{source}/{z}/{x}/{y_fmt}", get(get_tile))
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
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
    let browser_pool = state
        .browser_pool
        .as_ref()
        .ok_or_else(|| TileServerError::RenderError("Rendering not available".to_string()))?;

    // Parse parameters
    let (y, scale, format) = params
        .parse()
        .ok_or(TileServerError::InvalidTileRequest)?;

    // Get style
    let style = state
        .styles
        .get(&params.style)
        .ok_or_else(|| TileServerError::StyleNotFound(params.style.clone()))?;

    // Create render options
    let options = RenderOptions::for_tile(
        params.style.clone(),
        style.style_json.to_string(),
        params.z,
        params.x,
        y,
        scale,
        format,
    );

    // Acquire permit and render
    let _permit = browser_pool.acquire_permit().await?;
    let renderer = Renderer::new(browser_pool.browser());
    let image_data = renderer.render(options).await?;

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
    static_type: String,  // e.g., "-122.4,37.8,12" or "auto"
    size_fmt: String,      // e.g., "800x600.png" or "800x600@2x.webp"
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
    let browser_pool = state
        .browser_pool
        .as_ref()
        .ok_or_else(|| TileServerError::RenderError("Rendering not available".to_string()))?;

    // Parse parameters
    let (width, height, scale, format) = params
        .parse()
        .ok_or_else(|| {
            TileServerError::RenderError(format!("Invalid size format: {}", params.size_fmt))
        })?;

    // Parse static type
    let static_type = StaticType::from_str(&params.static_type)
        .map_err(TileServerError::RenderError)?;

    // Get style
    let style = state
        .styles
        .get(&params.style)
        .ok_or_else(|| TileServerError::StyleNotFound(params.style.clone()))?;

    // Create render options
    let options = RenderOptions::for_static(
        params.style.clone(),
        style.style_json.to_string(),
        static_type,
        width,
        height,
        scale,
        format,
        query,
    )
    .map_err(TileServerError::RenderError)?;

    // Acquire permit and render
    let _permit = browser_pool.acquire_permit().await?;
    let renderer = Renderer::new(browser_pool.browser());
    let image_data = renderer.render(options).await?;

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
