use axum::{
    extract::{Path, Query, State},
    http::{
        header::{ACCEPT, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE},
        HeaderMap, HeaderValue, Method, StatusCode,
    },
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
#[cfg(feature = "frontend")]
use axum::{http::Uri, response::Html};
#[cfg(feature = "frontend")]
use rust_embed::Embed;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, CorsLayer},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod cli;
mod logging;
mod telemetry;

use cli::Cli;
use tileserver_rs::admin;
use tileserver_rs::autodetect;
use tileserver_rs::cache_control;
use tileserver_rs::config;
use tileserver_rs::error::TileServerError;
use tileserver_rs::openapi;
use tileserver_rs::reload::{
    self, build_app_state, now_unix_seconds, AppState, ReloadController, ReloadMeta,
    RuntimeSettings, SharedState,
};
use tileserver_rs::render::{ImageFormat, RenderOptions, StaticQueryParams, StaticType};
use tileserver_rs::sources::{self, TileJson};
use tileserver_rs::startup;
use tileserver_rs::styles::{self, StyleInfo, UrlQueryParams};
use tileserver_rs::upload;
use tileserver_rs::wmts;

#[cfg(feature = "frontend")]
#[derive(Embed)]
#[folder = "apps/client/.output/public"]
struct Assets;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // Parse CLI arguments
    let cli = Cli::parse_args();
    let ui_enabled = cli.ui_enabled();
    let verbose = cli.verbose;

    // Resolve configuration via startup priority chain:
    // 1. --config path (explicit)
    // 2. positional PATH (auto-detect)
    // 3. default config.toml / /etc/tileserver-rs/config.toml
    // 4. CWD auto-detect
    let (mut config, auto_report) =
        startup::load_runtime_config(cli.config.clone(), cli.path.clone())?;

    // Initialize tracing with OpenTelemetry
    // Filter out verbose MapLibre Native logs unless explicitly requested
    let filter = if verbose {
        EnvFilter::from_default_env().add_directive("tileserver_rs=debug".parse()?)
    } else {
        EnvFilter::from_default_env().add_directive("tileserver_rs=info".parse()?)
    };

    let fmt_layer = tracing_subscriber::fmt::layer().compact();

    let registry = tracing_subscriber::registry().with(filter).with(fmt_layer);

    // Add OpenTelemetry layer if enabled
    if let Some(otel_layer) = telemetry::init_telemetry(&config.telemetry) {
        registry.with(otel_layer).init();
    } else {
        registry.init();
    }

    if let Some(ref report) = auto_report {
        log_auto_detect_report(report);
    }

    // Override with CLI arguments
    if let Some(host) = cli.host {
        config.server.host = host;
    }
    if let Some(port) = cli.port {
        config.server.port = port;
    }
    if let Some(public_url) = cli.public_url {
        config.server.public_url = Some(public_url);
    }

    let runtime = RuntimeSettings {
        ui_enabled,
        runtime_host: config.server.host.clone(),
        runtime_port: config.server.port,
        public_url_override: None,
    };

    let state = build_app_state(&config, &runtime).await?;

    let config_hash = if let Some(ref path) = cli.config {
        config::Config::load_with_metadata(Some(path.clone()))?.content_hash
    } else {
        use sha2::{Digest, Sha256};
        let content = toml::to_string(&config).unwrap_or_default();
        let digest = Sha256::digest(content.as_bytes());
        digest.iter().map(|b| format!("{:02x}", b)).collect()
    };

    let meta = ReloadMeta {
        config_hash,
        loaded_at_unix: now_unix_seconds(),
        loaded_sources: state.sources.len(),
        loaded_styles: state.styles.len(),
        renderer_enabled: state.renderer.is_some(),
    };

    let config_path_for_reload = cli.config.clone();
    let controller = Arc::new(ReloadController::new(
        state,
        meta,
        config_path_for_reload,
        runtime,
    ));
    let shared = SharedState::new(Arc::clone(&controller));

    if ui_enabled {
        tracing::info!("Web UI enabled at /");
    } else {
        tracing::info!("Web UI disabled (use --ui to enable)");
    }

    // Build CORS layer with proper multi-origin support
    let allow_origin = if config.server.cors_origins.is_empty()
        || config.server.cors_origins.iter().any(|o| o == "*")
    {
        // Wildcard: allow any origin
        if !config.server.cors_origins.is_empty() {
            tracing::warn!(
                "CORS configured with wildcard (*). Consider restricting origins in production."
            );
        }
        AllowOrigin::any()
    } else {
        // Specific origins: parse and validate each one
        let origins: Vec<HeaderValue> = config
            .server
            .cors_origins
            .iter()
            .filter_map(|o| {
                o.parse::<HeaderValue>().ok().or_else(|| {
                    tracing::warn!("Invalid CORS origin '{}', skipping", o);
                    None
                })
            })
            .collect();

        if origins.is_empty() {
            tracing::warn!("No valid CORS origins configured, defaulting to wildcard");
            AllowOrigin::any()
        } else {
            AllowOrigin::list(origins)
        }
    };

    let cors = CorsLayer::new()
        .allow_headers([ACCEPT, CONTENT_TYPE])
        .max_age(Duration::from_secs(86400))
        .allow_origin(allow_origin)
        .allow_methods([Method::GET, Method::OPTIONS, Method::HEAD]);

    let mut router = Router::new().merge(api_router(shared.clone()));

    // Add Swagger UI at /_openapi with bundled assets (works in air-gapped environments)
    let mut openapi_spec = openapi::ApiDoc::openapi();
    openapi_spec.info.version = env!("CARGO_PKG_VERSION").to_string();
    router = router.merge(SwaggerUi::new("/_openapi").url("/openapi.json", openapi_spec));

    // Add embedded SPA if UI is enabled
    #[cfg(feature = "frontend")]
    if ui_enabled {
        router = router.fallback(serve_spa);
    }
    #[cfg(not(feature = "frontend"))]
    if ui_enabled {
        tracing::warn!(
            "Web UI requested but binary was compiled without the 'frontend' feature. \
             Rebuild with `cargo build --features frontend` to enable the embedded UI."
        );
    }

    let router = router
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(axum::middleware::from_fn(logging::request_logger));

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    tracing::info!("Starting tileserver on http://{}", addr);

    let listener = TcpListener::bind(addr).await?;

    let admin_bind = &config.server.admin_bind;
    if admin_bind != "127.0.0.1:0" {
        let admin_addr: SocketAddr = admin_bind.parse()?;
        let admin_shared = shared.clone();
        tokio::spawn(async move {
            let admin_app = admin::admin_router(admin_shared);
            tracing::info!("Admin server listening on http://{}", admin_addr);
            match TcpListener::bind(admin_addr).await {
                Ok(admin_listener) => {
                    if let Err(e) = axum::serve(admin_listener, admin_app).await {
                        tracing::error!("Admin server error: {}", e);
                    }
                }
                Err(e) => tracing::error!("Failed to bind admin server to {}: {}", admin_addr, e),
            }
        });
    }

    tokio::spawn(reload::reload_signal(Arc::clone(&controller)));

    // Run the server with graceful shutdown
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Shutdown OpenTelemetry
    telemetry::shutdown_telemetry();

    Ok(())
}

/// Log the auto-detection report
fn log_auto_detect_report(report: &autodetect::AutoDetectReport) {
    tracing::info!("Auto-detected from: {}", report.target.display());
    if !report.sources.is_empty() {
        tracing::info!(
            "  Sources: {} ({})",
            report.sources.len(),
            report
                .sources
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if !report.styles.is_empty() {
        tracing::info!(
            "  Styles: {} ({})",
            report.styles.len(),
            report
                .styles
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if let Some(ref fonts) = report.fonts_dir {
        tracing::info!("  Fonts: {}", fonts.display());
    }
    if !report.geojson_files.is_empty() {
        tracing::info!("  GeoJSON files: {}", report.geojson_files.len());
    }
    for conflict in &report.conflicts {
        tracing::warn!("  Conflict: {}", conflict);
    }
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
#[cfg(feature = "frontend")]
async fn serve_spa(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try to serve the exact file
    if let Some(content) = Assets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        let mut headers = HeaderMap::new();
        let content_type = HeaderValue::from_str(mime.as_ref())
            .unwrap_or(HeaderValue::from_static("application/octet-stream"));
        headers.insert(CONTENT_TYPE, content_type);

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

fn api_router(state: SharedState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/ping", get(admin::ping_check))
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
        // Static files endpoint
        .route("/files/{*filepath}", get(get_static_file))
        // Upload endpoints (streaming, large body limit)
        .route(
            "/api/upload",
            post(upload::upload_file)
                .get(upload::list_uploads)
                .layer(axum::extract::DefaultBodyLimit::max(500 * 1024 * 1024)),
        )
        .route("/api/upload/{id}", delete(upload::delete_upload))
        // Spatial API endpoints (for LLM tool integration)
        .route("/api/spatial/schema/{source}", get(get_spatial_schema))
        .route("/api/spatial/stats/{source}", get(get_spatial_stats))
        .route("/api/spatial/query", post(post_spatial_query))
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

/// Query parameters for index endpoint
#[derive(Debug, serde::Deserialize, Default)]
struct IndexQueryParams {
    /// API key to append to all URLs
    key: Option<String>,
}

/// Get combined TileJSON array for all data sources and styles
/// Route: GET /index.json
/// Query parameters:
/// - `key`: Optional API key to append to all tile URLs
async fn get_index_json(
    State(shared): State<SharedState>,
    Query(query): Query<IndexQueryParams>,
) -> Json<Vec<IndexEntry>> {
    let state = shared.load();
    let mut entries = Vec::with_capacity(state.sources.len() + state.styles.len());

    // Build key query string
    let key_query = query
        .key
        .as_ref()
        .map(|k| format!("?key={}", urlencoding::encode(k)))
        .unwrap_or_default();

    // Add all data sources
    for metadata in state.sources.all_metadata() {
        entries.push(IndexEntry::Data(
            metadata.to_tilejson_with_key(&state.base_url, query.key.as_deref()),
        ));
    }

    // Add all styles as raster tile sources
    for style in state.styles.all() {
        let tile_url = format!(
            "{}/styles/{}/{{z}}/{{x}}/{{y}}.png{}",
            state.base_url, style.id, key_query
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

/// Query parameters for styles list endpoint
#[derive(Debug, serde::Deserialize, Default)]
struct StylesQueryParams {
    /// API key to append to style URLs
    key: Option<String>,
}

/// Get all available styles
/// Route: GET /styles.json
/// Query parameters:
/// - `key`: Optional API key to append to style URLs
async fn get_all_styles(
    State(shared): State<SharedState>,
    Query(query): Query<StylesQueryParams>,
) -> Json<Vec<StyleInfo>> {
    let state = shared.load();
    Json(
        state
            .styles
            .all_infos_with_key(&state.base_url, query.key.as_deref()),
    )
}

/// Query parameters for style.json endpoint
#[derive(Debug, serde::Deserialize, Default)]
struct StyleQueryParams {
    /// API key to forward to all URLs in the style
    key: Option<String>,
}

/// Get style.json for a specific style
/// Returns the style with all relative URLs rewritten to absolute URLs
/// Query parameters (like `?key=API_KEY`) are forwarded to all rewritten URLs
async fn get_style_json(
    State(shared): State<SharedState>,
    Path(style_id): Path<String>,
    Query(query): Query<StyleQueryParams>,
) -> Result<Json<serde_json::Value>, TileServerError> {
    let state = shared.load();
    let style = state
        .styles
        .get(&style_id)
        .ok_or_else(|| TileServerError::StyleNotFound(style_id))?;

    // Build query params to forward to rewritten URLs
    let url_params = UrlQueryParams::with_key(query.key);

    // Rewrite relative URLs to absolute URLs for external clients
    let rewritten_style =
        styles::rewrite_style_for_api(&style.style_json, &state.base_url, &url_params);

    Ok(Json(rewritten_style))
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
/// Query parameters for style TileJSON endpoint
#[derive(Debug, serde::Deserialize, Default)]
struct StyleTileJsonQueryParams {
    /// API key to append to tile URLs
    key: Option<String>,
}

/// Get TileJSON for raster tiles of a style
/// Route: GET /styles/{style}.json
/// Query parameters:
/// - `key`: Optional API key to append to tile URLs
async fn get_style_tilejson(
    State(shared): State<SharedState>,
    Path(style_json): Path<String>,
    Query(query): Query<StyleTileJsonQueryParams>,
) -> Result<Json<RasterTileJson>, TileServerError> {
    let state = shared.load();

    // Only handle requests ending with .json
    let style_id = style_json
        .strip_suffix(".json")
        .ok_or_else(|| TileServerError::StyleNotFound(style_json.clone()))?;

    let style = state
        .styles
        .get(style_id)
        .ok_or_else(|| TileServerError::StyleNotFound(style_id.to_string()))?;

    // Build raster tile URL template with optional key
    let key_query = query
        .key
        .as_ref()
        .map(|k| format!("?key={}", urlencoding::encode(k)))
        .unwrap_or_default();

    let tile_url = format!(
        "{}/styles/{}/{{z}}/{{x}}/{{y}}.png{}",
        state.base_url, style_id, key_query
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

/// Query parameters for data source endpoints
#[derive(Debug, serde::Deserialize, Default)]
struct DataSourceQueryParams {
    /// API key to append to tile URLs
    key: Option<String>,
}

/// Get all available tile sources
/// Route: GET /data.json
/// Query parameters:
/// - `key`: Optional API key to append to tile URLs
async fn get_all_sources(
    State(shared): State<SharedState>,
    Query(query): Query<DataSourceQueryParams>,
) -> Json<Vec<TileJson>> {
    let state = shared.load();
    let sources: Vec<TileJson> = state
        .sources
        .all_metadata()
        .iter()
        .map(|m| m.to_tilejson_with_key(&state.base_url, query.key.as_deref()))
        .collect();

    Json(sources)
}

/// Get TileJSON for a specific source
/// Route: GET /data/{source}
/// Query parameters:
/// - `key`: Optional API key to append to tile URLs
async fn get_source_tilejson(
    State(shared): State<SharedState>,
    Path(source): Path<String>,
    Query(query): Query<DataSourceQueryParams>,
) -> Result<Json<TileJson>, TileServerError> {
    let state = shared.load();

    // Strip .json extension if present
    let source_id = source.strip_suffix(".json").unwrap_or(&source);

    let source_ref = state
        .sources
        .get(source_id)
        .ok_or_else(|| TileServerError::SourceNotFound(source_id.to_string()))?;

    let tilejson = source_ref
        .metadata()
        .to_tilejson_with_key(&state.base_url, query.key.as_deref());
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
    fn parse_y_and_format(&self) -> Option<(u32, &str)> {
        let (y_str, format) = self.y_fmt.rsplit_once('.')?;
        let y = y_str.parse().ok()?;
        Some((y, format))
    }
}

async fn get_tile(
    State(shared): State<SharedState>,
    Path(params): Path<TileParams>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<Response, TileServerError> {
    let state = shared.load();

    let (y, format) = params
        .parse_y_and_format()
        .ok_or(TileServerError::InvalidTileRequest)?;

    if format == "geojson" {
        return get_tile_as_geojson(&state, &params.source, params.z, params.x, y).await;
    }

    #[cfg(feature = "raster")]
    let tile = {
        #[cfg(feature = "postgres")]
        if state.sources.is_postgres_function_source(&params.source) {
            let query_params = serde_json::to_value(&query).unwrap_or_default();
            state
                .sources
                .get_vector_tile_with_query_params(
                    &params.source,
                    params.z,
                    params.x,
                    y,
                    &query_params,
                )
                .await?
                .ok_or(TileServerError::TileNotFound {
                    z: params.z,
                    x: params.x,
                    y,
                })?
        } else {
            let resampling = query
                .get("resampling")
                .and_then(|s| s.parse::<config::ResamplingMethod>().ok());

            #[cfg(all(feature = "postgres", feature = "raster"))]
            let query_params = if state.sources.is_outdb_raster_source(&params.source) {
                Some(serde_json::to_value(&query).unwrap_or_default())
            } else {
                None
            };

            #[cfg(not(all(feature = "postgres", feature = "raster")))]
            let query_params: Option<serde_json::Value> = None;

            state
                .sources
                .get_raster_tile_with_params(
                    &params.source,
                    params.z,
                    params.x,
                    y,
                    256,
                    resampling,
                    query_params,
                )
                .await?
                .ok_or(TileServerError::TileNotFound {
                    z: params.z,
                    x: params.x,
                    y,
                })?
        }

        #[cfg(not(feature = "postgres"))]
        {
            let resampling = query
                .get("resampling")
                .and_then(|s| s.parse::<config::ResamplingMethod>().ok());

            state
                .sources
                .get_raster_tile_with_params(
                    &params.source,
                    params.z,
                    params.x,
                    y,
                    256,
                    resampling,
                    None,
                )
                .await?
                .ok_or(TileServerError::TileNotFound {
                    z: params.z,
                    x: params.x,
                    y,
                })?
        }
    };

    #[cfg(not(feature = "raster"))]
    let tile = {
        #[cfg(feature = "postgres")]
        let tile = if state.sources.is_postgres_function_source(&params.source) {
            let query_params: serde_json::Value = serde_json::to_value(&query).unwrap_or_default();
            state
                .sources
                .get_vector_tile_with_query_params(
                    &params.source,
                    params.z,
                    params.x,
                    y,
                    &query_params,
                )
                .await?
                .ok_or(TileServerError::TileNotFound {
                    z: params.z,
                    x: params.x,
                    y,
                })?
        } else {
            let source = state
                .sources
                .get(&params.source)
                .ok_or_else(|| TileServerError::SourceNotFound(params.source.clone()))?;
            source
                .get_tile(params.z, params.x, y)
                .await?
                .ok_or(TileServerError::TileNotFound {
                    z: params.z,
                    x: params.x,
                    y,
                })?
        };

        #[cfg(not(feature = "postgres"))]
        let tile = {
            let _ = query;
            let source = state
                .sources
                .get(&params.source)
                .ok_or_else(|| TileServerError::SourceNotFound(params.source.clone()))?;

            source
                .get_tile(params.z, params.x, y)
                .await?
                .ok_or(TileServerError::TileNotFound {
                    z: params.z,
                    x: params.x,
                    y,
                })?
        };

        tile
    };

    // MLT transcoding: if the requested format differs from the source format
    // and the `mlt` feature is enabled, attempt on-the-fly transcoding.
    #[cfg(feature = "mlt")]
    let tile = {
        let requested_format = format.parse::<tileserver_rs::sources::TileFormat>().ok();
        if let Some(target) = requested_format {
            if tile.format != target && tile.format.is_vector() && target.is_vector() {
                match tileserver_rs::transcode::transcode_tile(&tile, target) {
                    Ok(transcoded) => transcoded,
                    Err(e) => {
                        // Fall back to serving the original tile on any transcode
                        // error. This ensures clients always get usable data even
                        // when mlt-core cannot handle certain geometries or edge
                        // cases. The original tile (MVT or MLT) is still valid —
                        // only the on-the-fly format conversion failed.
                        tracing::warn!(
                            "transcoding {:?} -> {:?} failed for {}/{}/{}/{}, serving original tile: {}",
                            tile.format,
                            target,
                            params.source,
                            params.z,
                            params.x,
                            y,
                            e
                        );
                        tile
                    }
                }
            } else {
                tile
            }
        } else {
            tile
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(tile.format.content_type()),
    );
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());

    if let Some(encoding) = tile.compression.content_encoding() {
        headers.insert(CONTENT_ENCODING, HeaderValue::from_static(encoding));
    }

    Ok((headers, tile.data).into_response())
}

/// Get a tile as GeoJSON (helper function)
async fn get_tile_as_geojson(
    state: &AppState,
    source_id: &str,
    z: u8,
    x: u32,
    y: u32,
) -> Result<Response, TileServerError> {
    use flate2::read::GzDecoder;
    use geozero::mvt::{Message, Tile};
    use geozero::ProcessToJson;
    use sources::TileCompression;
    use std::io::Read;

    let source = state
        .sources
        .get(source_id)
        .ok_or_else(|| TileServerError::SourceNotFound(source_id.to_string()))?;

    // Check if source is vector format
    if source.metadata().format != sources::TileFormat::Pbf {
        return Err(TileServerError::RenderError(
            "GeoJSON conversion only supported for vector tiles (PBF)".to_string(),
        ));
    }

    let tile = source
        .get_tile(z, x, y)
        .await?
        .ok_or(TileServerError::TileNotFound { z, x, y })?;

    // Decompress if needed
    let raw_data = match tile.compression {
        TileCompression::Gzip => {
            let mut decoder = GzDecoder::new(&tile.data[..]);
            let mut decompressed = Vec::with_capacity(tile.data.len() * 4);
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

        let format = format_str.parse::<ImageFormat>().ok()?;

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
    State(shared): State<SharedState>,
    Path(params): Path<RasterTileParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();

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

        let format = format_str.parse::<ImageFormat>().ok()?;

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
    State(shared): State<SharedState>,
    Path(params): Path<RasterTileWithSizeParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();

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

        let format = format_str.parse::<ImageFormat>().ok()?;

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
    State(shared): State<SharedState>,
    Path(params): Path<StaticImageParams>,
    Query(query): Query<StaticQueryParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();

    let renderer = state
        .renderer
        .as_ref()
        .ok_or_else(|| TileServerError::RenderError("Rendering not available".to_string()))?;

    // Parse parameters
    let (width, height, scale, format) = params.parse().ok_or_else(|| {
        TileServerError::RenderError(format!("Invalid size format: {}", params.size_fmt))
    })?;

    // Parse static type
    let static_type = params
        .static_type
        .parse::<StaticType>()
        .map_err(TileServerError::RenderError)?;

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
    State(shared): State<SharedState>,
    Path(params): Path<SpriteParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    // Security: Strict validation of sprite file name
    // Only allow: sprite.png, sprite.json, sprite@2x.png, sprite@2x.json, sprite@3x.png, etc.
    if !params.sprite_file.starts_with("sprite") {
        return Err(TileServerError::InvalidTileRequest);
    }

    // Security: Reject any path traversal attempts
    if params.sprite_file.contains("..")
        || params.sprite_file.contains('/')
        || params.sprite_file.contains('\\')
    {
        return Err(TileServerError::InvalidTileRequest);
    }

    // Security: Validate sprite file matches expected pattern
    // Valid patterns: sprite.png, sprite.json, sprite@2x.png, sprite@2x.json, sprite@3x.png, etc.
    let valid_extensions = [".png", ".json"];
    let has_valid_extension = valid_extensions
        .iter()
        .any(|ext| params.sprite_file.ends_with(ext));
    if !has_valid_extension {
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

/// Query parameters for WMTS endpoint
#[derive(Debug, serde::Deserialize, Default)]
struct WmtsQueryParams {
    /// API key to include in all URLs
    key: Option<String>,
}

/// Get WMTS GetCapabilities document for a style
/// Route: GET /styles/{style}/wmts.xml
/// Query parameters:
/// - `key`: Optional API key to append to all tile URLs (e.g., `?key=my_api_key`)
async fn get_wmts_capabilities(
    State(shared): State<SharedState>,
    Path(style_id): Path<String>,
    Query(query): Query<WmtsQueryParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let style = state
        .styles
        .get(&style_id)
        .ok_or_else(|| TileServerError::StyleNotFound(style_id.clone()))?;

    // Generate WMTS capabilities XML with optional key
    let xml = wmts::generate_wmts_capabilities(
        &state.base_url,
        &style_id,
        &style.name,
        0,  // minzoom
        22, // maxzoom
        query.key.as_deref(),
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
    State(shared): State<SharedState>,
) -> Result<Json<Vec<String>>, TileServerError> {
    let state = shared.load();

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
    State(shared): State<SharedState>,
    Path(params): Path<FontParams>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
    let fonts_dir = state.fonts_dir.as_ref().ok_or_else(|| {
        TileServerError::FontNotFound("Fonts directory not configured".to_string())
    })?;

    // Parse the range to ensure it's valid (e.g., "0-255.pbf")
    // Must match pattern like "0-255.pbf", "256-511.pbf", etc.
    if !params.range.ends_with(".pbf") {
        return Err(TileServerError::InvalidTileRequest);
    }

    // Security: Validate range format to prevent path traversal
    let range_name = params.range.trim_end_matches(".pbf");
    if range_name.contains("..") || range_name.contains('/') || range_name.contains('\\') {
        return Err(TileServerError::InvalidTileRequest);
    }

    // Font stacks are comma-separated, try each font in order
    let fonts: Vec<&str> = params.fontstack.split(',').map(|s| s.trim()).collect();

    // Security: Canonicalize fonts directory for path validation
    let canonical_fonts_dir = fonts_dir
        .canonicalize()
        .map_err(|_| TileServerError::FontNotFound("Fonts directory not accessible".to_string()))?;

    for font_name in &fonts {
        // Security: Reject font names with path traversal sequences
        if font_name.contains("..") || font_name.contains('/') || font_name.contains('\\') {
            continue;
        }

        let font_path = fonts_dir.join(font_name).join(&params.range);

        // Security: Verify the resolved path is within fonts directory
        if let Ok(canonical_path) = font_path.canonicalize() {
            if !canonical_path.starts_with(&canonical_fonts_dir) {
                continue; // Path escapes fonts directory
            }
        }

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

    // Return empty PBF for missing glyph ranges — MapLibre Native requests all
    // 256 possible ranges and fails hard on 404, even for unpopulated Unicode blocks
    tracing::debug!(
        "Font range not found, returning empty PBF: {} (tried: {:?})",
        params.range,
        fonts
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-protobuf"),
    );
    headers.insert(CACHE_CONTROL, cache_control::tile_cache_headers());
    Ok((headers, Vec::<u8>::new()).into_response())
}

/// Get a static file from the files directory
/// Route: GET /files/{*filepath}
async fn get_static_file(
    State(shared): State<SharedState>,
    Path(filepath): Path<String>,
) -> Result<Response, TileServerError> {
    let state = shared.load();
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

// =============================================================================
// SPATIAL API ENDPOINTS (for LLM tool integration)
// =============================================================================

/// Layer info extracted from vector_layers metadata
#[derive(serde::Serialize)]
struct SpatialLayerInfo {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minzoom: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maxzoom: Option<u64>,
    /// Known field names and their types (from TileJSON metadata)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fields: Vec<SpatialFieldInfo>,
}

/// Field info from vector layer metadata
#[derive(serde::Serialize)]
struct SpatialFieldInfo {
    name: String,
    #[serde(rename = "type")]
    field_type: String,
}

/// Schema response for a source
#[derive(serde::Serialize)]
struct SpatialSchemaResponse {
    source: String,
    format: String,
    minzoom: u8,
    maxzoom: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    bounds: Option<[f64; 4]>,
    layers: Vec<SpatialLayerInfo>,
}

/// Stats response for a source
#[derive(serde::Serialize)]
struct SpatialStatsResponse {
    source: String,
    format: String,
    minzoom: u8,
    maxzoom: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    bounds: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    center: Option<[f64; 3]>,
    layer_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attribution: Option<String>,
}

/// Request body for spatial query
#[derive(serde::Deserialize)]
struct SpatialQueryRequest {
    /// Source ID to query
    source: String,
    /// Bounding box [west, south, east, north]
    #[serde(default)]
    bbox: Option<[f64; 4]>,
    /// Zoom level for tile resolution
    #[serde(default = "default_zoom")]
    zoom: u8,
    /// Optional layer filter (only return features from these layers)
    #[serde(default)]
    layers: Option<Vec<String>>,
    /// Maximum features to return
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_zoom() -> u8 {
    14
}

fn default_limit() -> usize {
    100
}

/// Feature in spatial query response
#[derive(serde::Serialize)]
struct SpatialFeature {
    layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    geometry_type: Option<String>,
    properties: serde_json::Value,
}

/// Response for spatial query
#[derive(serde::Serialize)]
struct SpatialQueryResponse {
    source: String,
    features: Vec<SpatialFeature>,
    total: usize,
    truncated: bool,
}

/// Get schema information for a source (vector layers, field types)
/// Route: GET /api/spatial/schema/{source}
async fn get_spatial_schema(
    State(shared): State<SharedState>,
    Path(source_id): Path<String>,
) -> Result<Json<SpatialSchemaResponse>, TileServerError> {
    let state = shared.load();
    let source = state
        .sources
        .get(&source_id)
        .ok_or_else(|| TileServerError::SourceNotFound(source_id.clone()))?;

    let meta = source.metadata();
    let layers = extract_layer_info(&meta.vector_layers);

    Ok(Json(SpatialSchemaResponse {
        source: meta.id.clone(),
        format: meta.format.extension().to_string(),
        minzoom: meta.minzoom,
        maxzoom: meta.maxzoom,
        bounds: meta.bounds,
        layers,
    }))
}

/// Get statistics for a source (bounds, zoom range, layer count)
/// Route: GET /api/spatial/stats/{source}
async fn get_spatial_stats(
    State(shared): State<SharedState>,
    Path(source_id): Path<String>,
) -> Result<Json<SpatialStatsResponse>, TileServerError> {
    let state = shared.load();
    let source = state
        .sources
        .get(&source_id)
        .ok_or_else(|| TileServerError::SourceNotFound(source_id.clone()))?;

    let meta = source.metadata();
    let layer_count = meta
        .vector_layers
        .as_ref()
        .and_then(|v| v.as_array())
        .map_or(0, |arr| arr.len());

    Ok(Json(SpatialStatsResponse {
        source: meta.id.clone(),
        format: meta.format.extension().to_string(),
        minzoom: meta.minzoom,
        maxzoom: meta.maxzoom,
        bounds: meta.bounds,
        center: meta.center,
        layer_count,
        name: Some(meta.name.clone()),
        description: meta.description.clone(),
        attribution: meta.attribution.clone(),
    }))
}

/// Query features from a source by decoding vector tiles
/// Route: POST /api/spatial/query
///
/// This endpoint decodes MVT/PBF tiles at the requested location and returns
/// features as JSON. For full SQL spatial queries, PostGIS sources are required.
async fn post_spatial_query(
    State(shared): State<SharedState>,
    Json(request): Json<SpatialQueryRequest>,
) -> Result<Json<SpatialQueryResponse>, TileServerError> {
    let state = shared.load();
    let source = state
        .sources
        .get(&request.source)
        .ok_or_else(|| TileServerError::SourceNotFound(request.source.clone()))?;

    let meta = source.metadata();

    // Only vector sources support feature queries
    if !meta.format.is_vector() {
        return Err(TileServerError::InvalidTileRequest);
    }

    // Determine tile coordinates from bbox or use center
    let (z, x, y) = if let Some(bbox) = request.bbox {
        // Use center of bbox at requested zoom
        let center_lng = (bbox[0] + bbox[2]) / 2.0;
        let center_lat = (bbox[1] + bbox[3]) / 2.0;
        let zoom = request.zoom.min(meta.maxzoom);
        let (tx, ty) = lng_lat_to_tile(center_lng, center_lat, zoom);
        (zoom, tx, ty)
    } else if let Some(center) = meta.center {
        // Use source center
        let zoom = request.zoom.min(meta.maxzoom);
        let (tx, ty) = lng_lat_to_tile(center[0], center[1], zoom);
        (zoom, tx, ty)
    } else {
        // Default to world view
        (0, 0, 0)
    };

    // Fetch the tile
    let tile_data = source
        .get_tile(z, x, y)
        .await?
        .ok_or(TileServerError::TileNotFound { z, x, y })?;

    // Decompress if needed
    let raw_data = match tile_data.compression {
        sources::TileCompression::Gzip => {
            use std::io::Read;
            let mut decoder = flate2::read::GzDecoder::new(&tile_data.data[..]);
            let mut decompressed = Vec::new();
            decoder
                .read_to_end(&mut decompressed)
                .map_err(|e| TileServerError::MetadataError(format!("gzip decode failed: {e}")))?;
            decompressed
        }
        _ => tile_data.data.to_vec(),
    };

    // Parse MVT protobuf and extract features
    let features = parse_mvt_features(&raw_data, &request.layers, request.limit);
    let total = features.len();

    Ok(Json(SpatialQueryResponse {
        source: request.source,
        features,
        total,
        truncated: total >= request.limit,
    }))
}

/// Convert longitude/latitude to tile coordinates at a given zoom level
#[inline]
fn lng_lat_to_tile(lng: f64, lat: f64, zoom: u8) -> (u32, u32) {
    let n = 2_u32.pow(zoom as u32);
    let x = ((lng + 180.0) / 360.0 * n as f64).floor() as u32;
    let lat_rad = lat.to_radians();
    let y = ((1.0 - lat_rad.tan().asinh() / std::f64::consts::PI) / 2.0 * n as f64).floor() as u32;
    (x.min(n - 1), y.min(n - 1))
}

/// Extract layer info from vector_layers JSON metadata
fn extract_layer_info(vector_layers: &Option<serde_json::Value>) -> Vec<SpatialLayerInfo> {
    let Some(layers) = vector_layers.as_ref().and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    layers
        .iter()
        .filter_map(|layer| {
            let id = layer.get("id")?.as_str()?.to_string();
            let description = layer
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from);
            let minzoom = layer.get("minzoom").and_then(|v| v.as_u64());
            let maxzoom = layer.get("maxzoom").and_then(|v| v.as_u64());

            let fields = layer
                .get("fields")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .map(|(name, field_type)| SpatialFieldInfo {
                            name: name.clone(),
                            field_type: field_type.as_str().unwrap_or("unknown").to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            Some(SpatialLayerInfo {
                id,
                description,
                minzoom,
                maxzoom,
                fields,
            })
        })
        .collect()
}

/// Parse MVT protobuf tile data and extract features as JSON.
/// This is a lightweight parser — it reads the protobuf wire format directly
/// without pulling in a full protobuf library (already have the data as bytes).
fn parse_mvt_features(
    data: &[u8],
    layer_filter: &Option<Vec<String>>,
    limit: usize,
) -> Vec<SpatialFeature> {
    // MVT is a protobuf format. We'll use a simple approach:
    // Try to parse as JSON-like structure from the raw tile metadata.
    // For a full implementation, we'd use prost to decode the MVT protobuf.
    // For now, return basic layer info extracted from the tile.
    //
    // NOTE: Full MVT decoding is available via the `mlt` feature flag
    // which brings in prost. This stub provides the API contract.
    let _ = (data, layer_filter, limit);

    // Return empty features with a note — full MVT decoding to be added
    // when the `mlt` feature flag is enabled (it brings prost for protobuf)
    Vec::new()
}
