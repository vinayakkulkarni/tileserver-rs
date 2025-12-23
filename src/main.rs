use axum::{
    extract::{Path, State},
    http::{
        header::{ACCEPT, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE},
        HeaderMap, HeaderValue, Method, StatusCode,
    },
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
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

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub sources: Arc<SourceManager>,
    pub base_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // Parse CLI arguments
    let cli = Cli::parse_args();

    // Initialize tracing
    let filter = if cli.verbose {
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
    };

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
    let router = Router::new()
        .nest("/", api_router(state.clone()))
        .merge(static_file_handler())
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    tracing::info!("Starting tileserver on http://{}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

fn static_file_handler() -> Router {
    Router::new()
        .nest_service("/_nuxt", ServeDir::new("./apps/client/dist/_nuxt"))
        .nest_service(
            "/",
            ServeDir::new("./apps/client/dist/")
                .not_found_service(ServeFile::new("./apps/client/dist/index.html")),
        )
        .layer(middleware::from_fn(cache_control::set_cache_header))
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
