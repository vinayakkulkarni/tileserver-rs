//! Server entry point for tileserver-rs.

use axum::{
    Router,
    http::{
        HeaderValue, Method,
        header::{ACCEPT, CONTENT_TYPE},
    },
};
#[cfg(feature = "frontend")]
use axum::{
    http::{HeaderMap, StatusCode, Uri, header::CACHE_CONTROL},
    response::{Html, IntoResponse},
};
#[cfg(feature = "frontend")]
use rust_embed::Embed;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, CorsLayer},
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;

mod cli;
mod logging;
mod telemetry;

use cli::Cli;
use tileserver_rs::admin;
use tileserver_rs::autodetect;
use tileserver_rs::config;
use tileserver_rs::openapi;
use tileserver_rs::reload::{
    self, ReloadController, ReloadMeta, RuntimeSettings, SharedState, build_app_state,
    now_unix_seconds,
};
use tileserver_rs::routes;
use tileserver_rs::startup;

#[cfg(feature = "frontend")]
#[derive(Embed)]
#[folder = "apps/client/.output/public"]
struct Assets;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse_args();
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

    // Build CORS layer
    let allow_origin = if config.server.cors_origins.is_empty()
        || config.server.cors_origins.iter().any(|o| o == "*")
    {
        if !config.server.cors_origins.is_empty() {
            tracing::warn!(
                "CORS configured with wildcard (*). Consider restricting origins in production."
            );
        }
        AllowOrigin::any()
    } else {
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

    let mut router = Router::new().merge(routes::api_router(shared.clone()));

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

    axum::serve(listener, router)
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
