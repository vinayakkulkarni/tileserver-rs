use axum::{
    extract::{Path, State},
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
mod sources;

use cli::Cli;
use config::Config;
use error::TileServerError;
use sources::{SourceManager, TileJson};

/// Embedded SPA assets (built from apps/client)
#[derive(Embed)]
#[folder = "apps/client/.output/public"]
struct Assets;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub sources: Arc<SourceManager>,
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

    tracing_subscriber::fmt().compact().with_env_filter(filter).init();

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

    // Build base URL
    let base_url = format!("http://{}:{}", config.server.host, config.server.port);

    let state = AppState {
        sources: Arc::new(sources),
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
    let mut router = Router::new().nest("/", api_router(state.clone()));

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
        .route("/data.json", get(get_all_sources))
        .route("/data/{source}.json", get(get_source_tilejson))
        .route("/data/{source}/{z}/{x}/{y}.{format}", get(get_tile))
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
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
    let source_ref = state
        .sources
        .get(&source)
        .ok_or_else(|| TileServerError::SourceNotFound(source.clone()))?;

    let tilejson = source_ref.metadata().to_tilejson(&state.base_url);
    Ok(Json(tilejson))
}

/// Tile request parameters
#[derive(serde::Deserialize)]
struct TileParams {
    source: String,
    z: u8,
    x: u32,
    y: u32,
    #[allow(dead_code)]
    format: String,
}

/// Get a tile from a source
async fn get_tile(
    State(state): State<AppState>,
    Path(params): Path<TileParams>,
) -> Result<Response, TileServerError> {
    let source = state
        .sources
        .get(&params.source)
        .ok_or_else(|| TileServerError::SourceNotFound(params.source.clone()))?;

    let tile = source
        .get_tile(params.z, params.x, params.y)
        .await?
        .ok_or(TileServerError::TileNotFound {
            z: params.z,
            x: params.x,
            y: params.y,
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
