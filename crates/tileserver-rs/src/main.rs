//! Server entry point for tileserver-rs.

use axum::{
    Router, ServiceExt,
    http::{
        Method,
        header::{ACCEPT, CONTENT_TYPE},
    },
    response::IntoResponse,
};
#[cfg(feature = "frontend")]
use axum::{
    http::{HeaderMap, HeaderValue, StatusCode, Uri, header::CACHE_CONTROL},
    response::Html,
};
#[cfg(feature = "frontend")]
use rust_embed::Embed;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tileserver_rs::trailing_slash::SelectiveTrailingSlashLayer;
use tokio::net::TcpListener;
use tower::Layer;
use tower_http::{compression::CompressionLayer, cors::CorsLayer};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;

mod cli;
mod logging;
mod telemetry;

use cli::Cli;
#[cfg(feature = "mcp")]
use cli::Commands;
use tileserver_rs::admin;
use tileserver_rs::autodetect;
use tileserver_rs::config;
#[cfg(feature = "mcp")]
use tileserver_rs::mcp;
use tileserver_rs::metrics;
use tileserver_rs::openapi;
use tileserver_rs::reload::{
    self, ReloadController, ReloadMeta, RuntimeSettings, SharedState, build_app_state,
    now_unix_seconds,
};
use tileserver_rs::routes;
use tileserver_rs::startup;

#[cfg(feature = "frontend")]
#[derive(Embed)]
// `rust-embed` resolves this path relative to CARGO_MANIFEST_DIR (= this
// crate's Cargo.toml). Since the crate is at crates/tileserver-rs/ and the
// build output lives at repo-root apps/client/.output/public, we must hop
// up two levels. DO NOT change to `apps/client/.output/public` without also
// moving the apps/client workspace under crates/tileserver-rs/ — the former
// would silently embed an empty Assets struct in debug builds and fail
// release builds with the "folder does not exist" error.
#[folder = "../../apps/client/.output/public"]
struct Assets;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse_args();

    #[cfg(feature = "mcp")]
    if let Some(Commands::McpStdio { config, verbose }) = cli.command.as_ref() {
        return mcp::run_stdio_from_config(config.clone(), *verbose).await;
    }

    let ui_enabled = cli.ui_enabled();
    let verbose = cli.verbose;

    // Resolve configuration via startup priority chain
    let (mut config, auto_report) =
        startup::load_runtime_config(cli.config.clone(), cli.path.clone())?;

    // Initialize tracing with OpenTelemetry
    let filter = if verbose {
        EnvFilter::from_default_env().add_directive("tileserver_rs=debug".parse()?)
    } else {
        EnvFilter::from_default_env().add_directive("tileserver_rs=info".parse()?)
    };

    let fmt_layer = tracing_subscriber::fmt::layer().compact();
    let registry = tracing_subscriber::registry().with(filter).with(fmt_layer);

    let telemetry_output = telemetry::init_telemetry(&config.telemetry);
    if let Some(otel_layer) = telemetry_output.tracing_layer {
        registry.with(otel_layer).init();
    } else {
        registry.init();
    }

    metrics::init(config.telemetry.metrics_label_cardinality.into());

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

    let cache_dir = config.resolve_cache_dir(cli.cache_dir.as_deref());
    config::Config::ensure_cache_dir_writable(&cache_dir)?;
    tracing::info!("Cache directory: {}", cache_dir.display());
    config.cache.dir = Some(cache_dir);

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
        use std::fmt::Write;
        let content = toml::to_string(&config).unwrap_or_default();
        let digest = Sha256::digest(content.as_bytes());
        let mut hex = String::with_capacity(64);
        for b in digest {
            write!(hex, "{b:02x}").expect("write to String never fails");
        }
        hex
    };

    let meta = ReloadMeta {
        config_hash,
        loaded_at_unix: now_unix_seconds(),
        loaded_sources: state.sources.len(),
        loaded_styles: state.styles.len(),
        renderer_enabled: state.renderer.is_some(),
        prometheus_listener_active: false,
    };

    let config_path_for_reload = cli.config.clone();
    let controller = Arc::new(ReloadController::new(
        state,
        meta,
        config.clone(),
        config_path_for_reload,
        runtime,
    ));
    let shared = SharedState::new(Arc::clone(&controller));

    if ui_enabled {
        tracing::info!("Web UI enabled at /");
    } else {
        tracing::info!("Web UI disabled (use --ui to enable)");
    }

    let allow_origin = tileserver_rs::cors_origin::build_allow_origin(&config.server.cors_origins)?;

    let cors = CorsLayer::new()
        .allow_headers([ACCEPT, CONTENT_TYPE])
        .max_age(Duration::from_secs(86400))
        .allow_origin(allow_origin)
        .allow_methods([Method::GET, Method::OPTIONS, Method::HEAD]);

    let mut router = Router::new().merge(routes::api_router(shared.clone()));

    #[cfg(feature = "mcp")]
    let mcp_oauth_store: Option<std::sync::Arc<dyn mcp::auth_store::OAuthBackend>> =
        if config.mcp.enabled {
            tracing::info!("MCP server enabled at /mcp (Streamable HTTP)");
            let auth_mode = mcp::transport::McpAuthMode::from_config(&config.mcp)?;
            let oauth_store = match &auth_mode {
                mcp::transport::McpAuthMode::OAuth(state) => Some(state.store.clone()),
                _ => None,
            };
            router = router.merge(mcp::mcp_router(
                shared.clone(),
                auth_mode,
                &config.mcp.cors_origins,
            ));
            oauth_store
        } else {
            None
        };

    // OpenAPI JSON endpoint (must be before SPA fallback)
    let mut openapi_spec = openapi::ApiDoc::openapi();
    openapi_spec.info.version = env!("CARGO_PKG_VERSION").to_string();
    let openapi_json = openapi_spec.to_pretty_json().unwrap();
    router = router.route(
        "/openapi.json",
        axum::routing::get(move || async move {
            (
                [(axum::http::header::CONTENT_TYPE, "application/json")],
                openapi_json.clone(),
            )
        }),
    );

    // Scalar API Reference (self-hosted, no CDN)
    let scalar_config = serde_json::json!({
        "url": "/openapi.json",
        "layout": "classic",
    });
    let scalar_html =
        scalar_api_reference::axum::scalar_response(&scalar_config, Some("/_openapi/scalar.js"));
    router = router
        .route(
            "/_openapi",
            axum::routing::get(move || async move { scalar_html.clone() }),
        )
        .route(
            "/_openapi/scalar.js",
            axum::routing::get(|| async {
                match scalar_api_reference::get_asset_with_mime("scalar.js") {
                    Some((mime, content)) => (
                        axum::http::StatusCode::OK,
                        [(axum::http::header::CONTENT_TYPE, mime)],
                        content,
                    )
                        .into_response(),
                    None => axum::http::StatusCode::NOT_FOUND.into_response(),
                }
            }),
        );

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
        .layer(axum::middleware::from_fn(metrics::record_http_request))
        .layer(axum::middleware::from_fn(logging::request_logger));

    let router = tileserver_rs::response_headers::apply_extra_response_headers(
        router,
        config.server.extra_response_headers.as_ref(),
    );

    // Normalize trailing slashes before routing so `/_openapi/` matches the
    // `/_openapi` route (and likewise for /health, /ping, etc.) instead of
    // 404ing — EXCEPT the SPA viewer routes `/styles/{id}/` and `/data/{id}/`,
    // whose trailing slash is load-bearing (it selects the embedded UI over the
    // greedy `/styles/{style_json}` / `/data/{source}` API routes). A blanket
    // trim collapsed every viewer link onto the API and 404'd it. The layer must
    // wrap the Router from the OUTSIDE: routing happens before inner `.layer()`
    // middleware, so applying it via `Router::layer()` would run too late. The
    // resulting service is served via `ServiceExt::into_make_service`.
    let app = SelectiveTrailingSlashLayer.layer(router);

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    tracing::info!("Starting tileserver on http://{}", addr);

    let listener = TcpListener::bind(addr).await?;

    let admin_bind = &config.server.admin_bind;
    if admin_bind != "127.0.0.1:0" {
        let admin_addr: SocketAddr = admin_bind.parse()?;
        if !admin_addr.ip().is_loopback() {
            tracing::warn!(
                bind = %admin_addr,
                "admin_bind is NOT a loopback address — destructive admin endpoints (/__admin/reload, /__admin/cache/flush, /__admin/oauth/*) are reachable on ALL interfaces. \
                 In production, set [server].admin_bind to '127.0.0.1:<port>' and reverse-proxy with auth. \
                 See https://github.com/vinayakkulkarni/tileserver-rs/blob/main/apps/docs/content/4.guides/16.mcp.md#admin-security"
            );
        }
        let admin_shared = shared.clone();
        #[cfg(feature = "mcp")]
        let admin_oauth_store = mcp_oauth_store.clone();
        tokio::spawn(async move {
            #[allow(unused_mut)]
            let mut admin_app = admin::admin_router(admin_shared);
            #[cfg(feature = "mcp")]
            if let Some(store) = admin_oauth_store {
                admin_app = admin_app.merge(mcp::admin_routes::admin_router(store));
                tracing::info!("MCP admin OAuth routes mounted at /__admin/oauth/*");
            }
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

    if let Some(registry) = telemetry_output.prometheus_registry
        && let Some(bind_str) = config.telemetry.prometheus_bind.as_ref()
    {
        match bind_str.parse::<SocketAddr>() {
            Ok(prom_addr) => {
                let path = config.telemetry.prometheus_path.clone();
                match metrics::spawn_metrics_server(prom_addr, path, registry).await {
                    Ok(_handle) => {
                        tracing::info!(
                            bind = %prom_addr,
                            "Prometheus /metrics endpoint enabled"
                        );
                        let mut updated = (*controller.meta.load_full()).clone();
                        updated.prometheus_listener_active = true;
                        controller.meta.store(Arc::new(updated));
                    }
                    Err(e) => {
                        tracing::error!(
                            bind = %prom_addr,
                            error = %e,
                            "Failed to bind Prometheus /metrics listener; tile serving continues"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    prometheus_bind = %bind_str,
                    error = %e,
                    "Invalid prometheus_bind address; Prometheus /metrics disabled"
                );
            }
        }
    }

    axum::serve(
        listener,
        ServiceExt::<axum::extract::Request>::into_make_service(app),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    telemetry::shutdown_telemetry();

    Ok(())
}

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

#[cfg(feature = "frontend")]
async fn serve_spa(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if let Some(content) = Assets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        let mut headers = HeaderMap::new();
        let content_type = HeaderValue::from_str(mime.as_ref())
            .unwrap_or(HeaderValue::from_static("application/octet-stream"));
        headers.insert(CONTENT_TYPE, content_type);

        if path.starts_with("_nuxt/") {
            headers.insert(
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
        }

        return (headers, content.data.to_vec()).into_response();
    }

    if let Some(index) = Assets::get("index.html") {
        return Html(index.data.to_vec()).into_response();
    }

    (StatusCode::NOT_FOUND, "Not Found").into_response()
}
