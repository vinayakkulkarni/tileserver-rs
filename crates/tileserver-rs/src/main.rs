//! Server entry point for tileserver-rs.

use axum::{Router, ServiceExt};
#[cfg(feature = "frontend")]
use axum::{
    http::{
        HeaderMap, HeaderValue, StatusCode, Uri,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    response::{Html, IntoResponse},
};
#[cfg(feature = "frontend")]
use rust_embed::Embed;
use std::{net::SocketAddr, sync::Arc};
use tileserver_rs::trailing_slash::SelectiveTrailingSlashLayer;
use tokio::net::TcpListener;
use tower::Layer;
use tower_http::compression::CompressionLayer;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod logging;
mod telemetry;

use cli::Cli;
use cli::Commands;
use tileserver_rs::admin;
use tileserver_rs::autodetect;
use tileserver_rs::config;
#[cfg(feature = "mcp")]
use tileserver_rs::mcp;
use tileserver_rs::metrics;
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

/// `tileserver-rs inspect`: print the archive report and exit.
async fn run_inspect(
    target: &str,
    tile: Option<&str>,
    verbose: bool,
    json: bool,
) -> anyhow::Result<()> {
    print!(
        "{}",
        tileserver_rs::inspect::run(target, tile, verbose, json).await?
    );
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse_args();

    if let Some(Commands::Inspect {
        target,
        tile,
        verbose,
        json,
    }) = cli.command.as_ref()
    {
        return run_inspect(target, tile.as_deref(), *verbose, *json).await;
    }
    #[cfg(feature = "mcp")]
    if let Some(Commands::McpStdio { config, verbose }) = cli.command.as_ref() {
        return mcp::run_stdio_from_config(config.clone(), *verbose).await;
    }

    serve(cli).await
}

/// Load the config, build the app, start the side listeners and serve
/// until a shutdown signal.
async fn serve(cli: Cli) -> anyhow::Result<()> {
    let (config, auto_report) = startup::load_runtime_config(cli.config.clone(), cli.path.clone())?;
    let prometheus_registry = init_tracing(&config, cli.verbose)?;
    serve_until(
        &cli,
        config,
        auto_report.as_ref(),
        prometheus_registry,
        shutdown_signal(),
    )
    .await?;
    telemetry::shutdown_telemetry();
    Ok(())
}

/// Everything after config load and logging setup: build the state, bind,
/// start the side tasks and serve until `shutdown` resolves.
async fn serve_until(
    cli: &Cli,
    mut config: config::Config,
    auto_report: Option<&autodetect::AutoDetectReport>,
    prometheus_registry: Option<prometheus::Registry>,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    metrics::init(config.telemetry.metrics_label_cardinality.into());
    if let Some(report) = auto_report {
        log_auto_detect_report(report);
    }
    prepare_config(&mut config, cli)?;
    let controller = build_controller(&config, cli).await?;
    let shared = SharedState::new(Arc::clone(&controller));

    let (app, oauth_store) = build_app(&config, &shared, cli.ui_enabled())?;
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Starting tileserver on http://{}", listener.local_addr()?);

    if let Some(admin_addr) = startup::admin_addr(&config.server.admin_bind)? {
        spawn_admin_server(admin_addr, shared.clone(), oauth_store);
    }
    tokio::spawn(reload::reload_signal(Arc::clone(&controller)));
    tokio::spawn(tileserver_rs::live::run_watcher(shared.clone()));
    if let Some(registry) = prometheus_registry {
        spawn_prometheus(&config, registry, &controller).await;
    }

    axum::serve(
        listener,
        ServiceExt::<axum::extract::Request>::into_make_service(app),
    )
    .with_graceful_shutdown(shutdown)
    .await?;
    Ok(())
}

/// Apply CLI overrides and make sure the cache directory is writable.
fn prepare_config(config: &mut config::Config, cli: &Cli) -> anyhow::Result<()> {
    apply_cli_overrides(config, cli);
    #[cfg(feature = "sftp")]
    configure_sftp(config, cli);
    let cache_dir = config.resolve_cache_dir(cli.cache_dir.as_deref());
    config::Config::ensure_cache_dir_writable(&cache_dir)?;
    tracing::info!("Cache directory: {}", cache_dir.display());
    config.cache.dir = Some(cache_dir);
    Ok(())
}

/// Build the app state and the reload controller that owns it.
async fn build_controller(
    config: &config::Config,
    cli: &Cli,
) -> anyhow::Result<Arc<ReloadController>> {
    let runtime = RuntimeSettings {
        ui_enabled: cli.ui_enabled(),
        runtime_host: config.server.host.clone(),
        runtime_port: config.server.port,
        public_url_override: None,
    };
    let state = build_app_state(config, &runtime).await?;
    let meta = ReloadMeta {
        config_hash: startup::config_hash(cli.config.as_ref(), config)?,
        loaded_at_unix: now_unix_seconds(),
        loaded_sources: state.sources.len(),
        loaded_styles: state.styles.len(),
        renderer_enabled: state.renderer.is_some(),
        prometheus_listener_active: false,
    };
    Ok(Arc::new(ReloadController::new(
        state,
        meta,
        config.clone(),
        cli.config.clone(),
        runtime,
    )))
}

/// Install the tracing subscriber (plus OpenTelemetry when configured) and
/// return the Prometheus registry, if metrics export is on.
fn init_tracing(
    config: &config::Config,
    verbose: bool,
) -> anyhow::Result<Option<prometheus::Registry>> {
    let level = if verbose { "debug" } else { "info" };
    let filter =
        EnvFilter::from_default_env().add_directive(format!("tileserver_rs={level}").parse()?);
    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().compact());
    let telemetry_output = telemetry::init_telemetry(&config.telemetry);
    if let Some(otel_layer) = telemetry_output.tracing_layer {
        registry.with(otel_layer).init();
    } else {
        registry.init();
    }
    Ok(telemetry_output.prometheus_registry)
}

/// `--host`, `--port` and `--public-url` win over the config file.
fn apply_cli_overrides(config: &mut config::Config, cli: &Cli) {
    if let Some(host) = &cli.host {
        config.server.host.clone_from(host);
    }
    if let Some(port) = cli.port {
        config.server.port = port;
    }
    if let Some(public_url) = &cli.public_url {
        config.server.public_url = Some(public_url.clone());
    }
}

#[cfg(feature = "sftp")]
fn configure_sftp(config: &config::Config, cli: &Cli) {
    use tileserver_rs::sources::pmtiles::sftp;
    sftp::set_cli_ssh_identity(cli.ssh_identity.clone());
    sftp::set_cli_insecure_skip_host_key_verify(cli.ssh_insecure_skip_host_key_verify);
    sftp::set_global_known_hosts_path(
        config
            .sftp
            .as_ref()
            .and_then(|s| s.known_hosts_path.clone()),
    );
    if cli.ssh_insecure_skip_host_key_verify {
        tracing::warn!(
            flag = "--ssh-insecure-skip-host-key-verify",
            "SSH host key verification disabled — DO NOT USE IN PRODUCTION. Server accepts ANY host key."
        );
    }
}

#[cfg(feature = "mcp")]
type OAuthStore = Option<Arc<dyn mcp::auth_store::OAuthBackend>>;
#[cfg(not(feature = "mcp"))]
type OAuthStore = ();

/// The public router: API, MCP, API docs, the optional SPA, middleware, and
/// the outer subfolder-strip and trailing-slash layers.
fn build_app(
    config: &config::Config,
    shared: &SharedState,
    ui_enabled: bool,
) -> anyhow::Result<(
    tileserver_rs::base_path::BasePathStrip<
        tileserver_rs::trailing_slash::SelectiveTrailingSlash<Router>,
    >,
    OAuthStore,
)> {
    if ui_enabled {
        tracing::info!("Web UI enabled at /");
    } else {
        tracing::info!("Web UI disabled (use --ui to enable)");
    }
    let mut router = Router::new().merge(routes::api_router(shared.clone()));

    #[cfg(feature = "mcp")]
    let oauth_store = if config.mcp.enabled {
        tracing::info!("MCP server enabled at /mcp (Streamable HTTP)");
        let auth_mode = mcp::transport::McpAuthMode::from_config(&config.mcp)?;
        let store = match &auth_mode {
            mcp::transport::McpAuthMode::OAuth(state) => Some(state.store.clone()),
            _ => None,
        };
        router = router.merge(mcp::mcp_router(
            shared.clone(),
            auth_mode,
            &config.mcp.cors_origins,
        ));
        store
    } else {
        None
    };
    #[cfg(not(feature = "mcp"))]
    let oauth_store = ();

    router = router.merge(startup::docs_router()?);

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
        .layer(startup::cors_layer(&config.server.cors_origins)?)
        .layer(CompressionLayer::new())
        .layer(axum::middleware::from_fn(metrics::record_http_request))
        .layer(axum::middleware::from_fn(logging::request_logger));
    let router = tileserver_rs::response_headers::apply_extra_response_headers(
        router,
        config.server.extra_response_headers.as_ref(),
    );

    // Layers wrap the router from the OUTSIDE: routing runs before inner
    // `.layer()` middleware. Strip the subfolder prefix first (nested mode
    // only; in proxy-strip mode the proxy does it and the base is empty),
    // then normalize trailing slashes — except on the SPA viewer routes
    // `/styles/{id}/` and `/data/{id}/`, whose slash selects the UI over the
    // API routes — then route.
    let base_path = tileserver_rs::config::derive_base_path(config.server.public_url.as_deref());
    #[cfg(feature = "frontend")]
    init_spa_cache(&base_path);
    let strip_base = config.server.subfolder_mode.strip_base(&base_path);
    let app = tileserver_rs::base_path::BasePathStripLayer::new(strip_base.as_str())
        .layer(SelectiveTrailingSlashLayer.layer(router));
    Ok((app, oauth_store))
}

/// Serve the admin API on its own listener.
fn spawn_admin_server(addr: SocketAddr, shared: SharedState, oauth_store: OAuthStore) {
    #[cfg(not(feature = "mcp"))]
    let () = oauth_store;
    tokio::spawn(async move {
        let app = admin::admin_app(
            shared,
            #[cfg(feature = "mcp")]
            oauth_store,
        );
        tracing::info!("Admin server listening on http://{}", addr);
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                if let Err(e) = axum::serve(listener, app).await {
                    tracing::error!("Admin server error: {}", e);
                }
            }
            Err(e) => tracing::error!("Failed to bind admin server to {}: {}", addr, e),
        }
    });
}

/// Start the Prometheus `/metrics` listener when `prometheus_bind` is set.
/// A bad address or bind failure is logged; tile serving continues.
async fn spawn_prometheus(
    config: &config::Config,
    registry: prometheus::Registry,
    controller: &ReloadController,
) {
    let Some(bind_str) = config.telemetry.prometheus_bind.as_ref() else {
        return;
    };
    let prom_addr = match bind_str.parse::<SocketAddr>() {
        Ok(addr) => addr,
        Err(e) => {
            tracing::error!(
                prometheus_bind = %bind_str,
                error = %e,
                "Invalid prometheus_bind address; Prometheus /metrics disabled"
            );
            return;
        }
    };
    let path = config.telemetry.prometheus_path.clone();
    match metrics::spawn_metrics_server(prom_addr, path, registry).await {
        Ok(_handle) => {
            tracing::info!(bind = %prom_addr, "Prometheus /metrics endpoint enabled");
            let mut updated = (*controller.meta.load_full()).clone();
            updated.prometheus_listener_active = true;
            controller.meta.store(Arc::new(updated));
        }
        Err(e) => tracing::error!(
            bind = %prom_addr,
            error = %e,
            "Failed to bind Prometheus /metrics listener; tile serving continues"
        ),
    }
}

fn log_auto_detect_report(report: &autodetect::AutoDetectReport) {
    for line in report.summary() {
        tracing::info!("{line}");
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

/// Rewritten embedded SPA assets, keyed by their root-relative path (e.g.
/// `_nuxt/entry.js`). Populated once at startup for subfolder deployments;
/// empty for root deployments, in which case [`serve_spa`] serves the raw
/// embedded bytes unchanged (zero behavioural change for the common case).
#[cfg(feature = "frontend")]
static SPA_CACHE: std::sync::OnceLock<std::collections::HashMap<String, Vec<u8>>> =
    std::sync::OnceLock::new();

/// Build the rewritten-asset cache for a subfolder deployment. For an empty
/// `base_path` (root deployment) the cache stays empty and every asset is
/// served from the raw embed. Idempotent: only the first call populates it.
#[cfg(feature = "frontend")]
fn init_spa_cache(base_path: &str) {
    let mut map = std::collections::HashMap::new();
    if !base_path.is_empty() {
        for file in Assets::iter() {
            let path = file.as_ref();
            if let Some(content) = Assets::get(path)
                && let Some(rewritten) =
                    tileserver_rs::spa::rewrite_asset(path, &content.data, base_path)
            {
                map.insert(path.to_owned(), rewritten);
            }
        }
        tracing::info!(
            base_path,
            rewritten_assets = map.len(),
            "Rebased embedded GUI for subfolder deployment"
        );
    }
    let _ = SPA_CACHE.set(map);
}

/// Resolve embedded SPA bytes for `path`, preferring the rewritten cache and
/// falling back to the raw embed.
#[cfg(feature = "frontend")]
fn resolve_spa_asset(path: &str) -> Option<Vec<u8>> {
    if let Some(bytes) = SPA_CACHE.get().and_then(|cache| cache.get(path)) {
        return Some(bytes.clone());
    }
    Assets::get(path).map(|content| content.data.to_vec())
}

#[cfg(feature = "frontend")]
async fn serve_spa(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if let Some(data) = resolve_spa_asset(path) {
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

        return (headers, data).into_response();
    }

    // A missing build asset must be a real 404. Serving index.html here gives
    // the browser HTML with status 200 for a script/worker URL, which hides
    // the defect (MapLibre v6 worker 404 rendered as a silent empty map).
    if path.starts_with("_nuxt/") {
        return (StatusCode::NOT_FOUND, "Not Found").into_response();
    }

    if let Some(index) = resolve_spa_asset("index.html") {
        return Html(index).into_response();
    }

    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

#[cfg(all(test, feature = "frontend"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_build_asset_returns_404() {
        let response = serve_spa(Uri::from_static("/_nuxt/missing-chunk.mjs"))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unknown_client_route_serves_index_html() {
        let response = serve_spa(Uri::from_static("/styles/india-dark/"))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok());
        assert_eq!(content_type, Some("text/html; charset=utf-8"));
    }
}

#[cfg(test)]
mod startup_tests {
    use super::*;
    use clap::Parser;

    #[tokio::test]
    async fn serve_until_starts_and_stops_on_shutdown() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        std::fs::write(&config_path, "[server]\nhost = \"127.0.0.1\"\nport = 0\n").unwrap();
        let cache = dir.path().join("cache");
        let cli = Cli::parse_from([
            "tileserver-rs",
            "--config",
            config_path.to_str().unwrap(),
            "--cache-dir",
            cache.to_str().unwrap(),
            "--no-ui",
        ]);
        let (config, report) =
            startup::load_runtime_config(cli.config.clone(), cli.path.clone()).unwrap();
        let shutdown = tokio::time::sleep(std::time::Duration::from_millis(100));
        serve_until(&cli, config, report.as_ref(), None, shutdown)
            .await
            .unwrap();
        assert!(cache.is_dir(), "prepare_config creates the cache dir");
    }

    #[tokio::test]
    async fn serve_until_reports_a_bad_address() {
        let dir = tempfile::tempdir().unwrap();
        let cli = Cli::parse_from([
            "tileserver-rs",
            "--cache-dir",
            dir.path().to_str().unwrap(),
            "--host",
            "not a host",
            "--no-ui",
        ]);
        let err = serve_until(&cli, config::Config::default(), None, None, async {})
            .await
            .unwrap_err();
        assert!(err.to_string().contains("invalid socket address"), "{err}");
    }

    #[test]
    fn cli_overrides_win_over_the_config_file() {
        let mut config = config::Config::default();
        config.server.public_url = Some("https://old.example".to_string());
        let cli = Cli::try_parse_from([
            "tileserver-rs",
            "--host",
            "10.0.0.1",
            "--port",
            "9100",
            "--public-url",
            "https://tiles.example",
        ])
        .unwrap();
        apply_cli_overrides(&mut config, &cli);
        assert_eq!(config.server.host, "10.0.0.1");
        assert_eq!(config.server.port, 9100);
        assert_eq!(
            config.server.public_url.as_deref(),
            Some("https://tiles.example")
        );
    }

    #[test]
    fn missing_cli_flags_keep_the_config_values() {
        let mut config = config::Config::default();
        let before = (config.server.host.clone(), config.server.port);
        apply_cli_overrides(
            &mut config,
            &Cli::try_parse_from(["tileserver-rs"]).unwrap(),
        );
        assert_eq!((config.server.host, config.server.port), before);
    }

    #[tokio::test]
    async fn build_app_serves_the_api_and_docs() {
        let config = config::Config::default();
        let state = tileserver_rs::reload::AppState {
            sources: Arc::new(tileserver_rs::sources::SourceManager::new()),
            styles: Arc::new(tileserver_rs::styles::StyleManager::new()),
            renderer: None,
            base_url: "http://localhost:8080".to_string(),
            render_base_url: "http://127.0.0.1:8080".to_string(),
            ui_enabled: false,
            fonts_dir: None,
            files_dir: None,
            upload_dir: None,
        };
        let meta = ReloadMeta {
            config_hash: String::new(),
            loaded_at_unix: 0,
            loaded_sources: 0,
            loaded_styles: 0,
            renderer_enabled: false,
            prometheus_listener_active: false,
        };
        let runtime = RuntimeSettings {
            ui_enabled: false,
            runtime_host: "127.0.0.1".to_string(),
            runtime_port: 8080,
            public_url_override: None,
        };
        let shared = SharedState::new(Arc::new(ReloadController::new(
            state,
            meta,
            config.clone(),
            None,
            runtime,
        )));
        let (app, _oauth) = build_app(&config, &shared, false).unwrap();
        let server = axum_test::TestServer::new(
            ServiceExt::<axum::extract::Request>::into_make_service(app),
        );
        server.get("/health").await.assert_status_ok();
        server.get("/openapi.json").await.assert_status_ok();
    }
}
