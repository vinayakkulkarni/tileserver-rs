//! Admin server for hot-reload and runtime diagnostics (`/ping`, `/__admin/reload`).

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
};

use crate::config_schema::{CONFIG_SCHEMA, ConfigSectionSchema};
use crate::reload::SharedState;

#[derive(Debug, serde::Deserialize, Default)]
struct ReloadQueryParams {
    flush: Option<bool>,
}

#[derive(serde::Serialize)]
pub struct PingResponse {
    status: &'static str,
    config_hash: String,
    loaded_at_unix: u64,
    loaded_sources: usize,
    loaded_styles: usize,
    renderer_enabled: bool,
    prometheus_listener_active: bool,
    version: &'static str,
    cache_enabled: bool,
    cache_entries: u64,
    cache_bytes: u64,
    // Capability flags surfaced to the client UI (Hero pills + admin badges).
    // NOTE: `/ping` is public + unauthenticated — `cors_origins` and `cache_dir`
    // are deliberately exposed here per operator request; production deployments
    // that treat CORS allow-lists or filesystem paths as sensitive should gate
    // `/ping` behind their reverse proxy.
    render_enabled: bool,
    ogc_enabled: bool,
    compression_enabled: bool,
    compression_br_quality: u8,
    compression_zstd_level: i32,
    cors_origins: Vec<String>,
    cache_dir: String,
    /// `GET /ws` is mounted (`[live] enabled = true`).
    live_enabled: bool,
}

#[derive(serde::Serialize)]
struct ReloadResponse {
    ok: bool,
    reloaded: bool,
    config_hash: String,
    loaded_at_unix: u64,
    loaded_sources: usize,
    loaded_styles: usize,
    renderer_enabled: bool,
    prometheus_listener_active: bool,
    version: &'static str,
}

#[derive(serde::Serialize)]
struct ReloadErrorResponse {
    ok: bool,
    error: String,
}

#[derive(serde::Serialize)]
struct CacheFlushResponse {
    ok: bool,
    invalidated_entries: u64,
    freed_bytes: u64,
}

#[derive(serde::Serialize)]
struct ConfigViewResponse {
    ok: bool,
    toml: String,
    source_path: Option<String>,
    config_hash: String,
    loaded_at_unix: u64,
}

#[derive(serde::Serialize)]
struct ConfigViewErrorResponse {
    ok: bool,
    error: String,
}

#[derive(serde::Serialize)]
struct ConfigSchemaResponse {
    ok: bool,
    sections: &'static [ConfigSectionSchema],
}

/// The admin listener's router: [`admin_router`] plus, with the `mcp`
/// feature, the OAuth admin routes at `/__admin/oauth/*`.
///
/// The OAuth routes are always mounted so the admin UI can render its
/// connected-apps and device-sessions pages. Without a backing store (OAuth
/// off, or a static bearer token) an empty in-memory store answers with
/// empty lists instead of `404`.
pub fn admin_app(
    state: SharedState,
    #[cfg(feature = "mcp")] oauth_store: Option<
        std::sync::Arc<dyn crate::mcp::auth_store::OAuthBackend>,
    >,
) -> Router {
    let guard_state = state.clone();
    let app = admin_router(state);
    #[cfg(feature = "mcp")]
    let app = {
        let store = oauth_store
            .unwrap_or_else(|| std::sync::Arc::new(crate::mcp::auth_store::InMemoryStore::new()));
        tracing::info!("MCP admin OAuth routes mounted at /__admin/oauth/*");
        app.merge(crate::mcp::admin_routes::admin_router(store))
    };
    app.layer(axum::middleware::from_fn_with_state(
        guard_state,
        admin_host_guard,
    ))
}

/// Host names the admin listener always answers to.
const LOOPBACK_HOSTS: [&str; 4] = ["localhost", "127.0.0.1", "::1", "[::1]"];

/// The host part of a `Host` header or an `Origin` URL, lowercased and
/// without the port.
fn host_part(value: &str) -> String {
    let rest = value.split_once("://").map_or(value, |(_, r)| r);
    let authority = rest.split('/').next().unwrap_or(rest);
    let host = if authority.starts_with('[') {
        authority.split_inclusive(']').next().unwrap_or(authority)
    } else {
        authority.rsplit_once(':').map_or(authority, |(h, _)| h)
    };
    host.to_ascii_lowercase()
}

fn admin_host_allowed(host: &str, extra: &[String]) -> bool {
    LOOPBACK_HOSTS.contains(&host)
        || host
            .trim_matches(|c| c == '[' || c == ']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
        || extra.iter().any(|h| h.eq_ignore_ascii_case(host))
}

/// Reject admin requests whose `Host` or `Origin` names another site.
///
/// The admin listener has no login; it relies on being reachable only from
/// the machine (or a trusted proxy). A web page in the operator's browser
/// could still reach it through DNS rebinding (`Host: attacker.example`)
/// or a cross-site request (`Origin: https://attacker.example`). Loopback
/// names and `[server].admin_allowed_hosts` pass; everything else gets 403.
async fn admin_host_guard(
    State(shared): State<SharedState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let config = shared.config();
    let extra = &config.server.admin_allowed_hosts;
    let headers = request.headers();
    let header = |name| {
        headers
            .get(name)
            .and_then(|v: &axum::http::HeaderValue| v.to_str().ok())
    };
    let host_ok =
        header(axum::http::header::HOST).is_none_or(|h| admin_host_allowed(&host_part(h), extra));
    let origin_ok = header(axum::http::header::ORIGIN)
        .is_none_or(|o| o != "null" && admin_host_allowed(&host_part(o), extra));
    if host_ok && origin_ok {
        next.run(request).await
    } else {
        (
            StatusCode::FORBIDDEN,
            "admin listener: Host or Origin not allowed; add it to [server].admin_allowed_hosts",
        )
            .into_response()
    }
}

pub fn admin_router(state: SharedState) -> Router {
    Router::new()
        .route("/__admin/reload", post(admin_reload))
        .route("/__admin/cache/flush", post(admin_cache_flush))
        .route("/__admin/config", get(admin_config))
        .route("/__admin/config/schema", get(admin_config_schema))
        .route("/__admin/stats", get(admin_stats))
        .with_state(state)
}

async fn admin_config_schema() -> Json<ConfigSchemaResponse> {
    Json(ConfigSchemaResponse {
        ok: true,
        sections: CONFIG_SCHEMA,
    })
}

pub async fn ping_check(State(shared): State<SharedState>) -> Json<PingResponse> {
    let meta = shared.meta();
    let state = shared.load();
    let config = shared.config();
    let (cache_enabled, cache_entries, cache_bytes) = if let Some(cache) = state.sources.cache() {
        (true, cache.entry_count(), cache.weighted_size())
    } else {
        (false, 0, 0)
    };
    Json(PingResponse {
        status: "ok",
        config_hash: meta.config_hash.clone(),
        loaded_at_unix: meta.loaded_at_unix,
        loaded_sources: meta.loaded_sources,
        loaded_styles: meta.loaded_styles,
        renderer_enabled: meta.renderer_enabled,
        prometheus_listener_active: meta.prometheus_listener_active,
        version: env!("CARGO_PKG_VERSION"),
        cache_enabled,
        cache_entries,
        cache_bytes,
        render_enabled: !config.server.disable_render,
        ogc_enabled: !config.server.disable_ogc,
        compression_enabled: !config.compression.minimal_recompression,
        compression_br_quality: config.compression.br_quality,
        compression_zstd_level: config.compression.zstd_level,
        live_enabled: config.live.enabled,
        cors_origins: config.server.cors_origins.clone(),
        cache_dir: config.resolve_cache_dir(None).display().to_string(),
    })
}

async fn admin_config(
    State(shared): State<SharedState>,
) -> Result<Json<ConfigViewResponse>, (StatusCode, Json<ConfigViewErrorResponse>)> {
    let config = shared.config();
    let meta = shared.meta();
    let source_path = shared.config_path().map(|p| p.display().to_string());
    let toml = toml::to_string_pretty(&*config).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ConfigViewErrorResponse {
                ok: false,
                error: format!("failed to serialize loaded config as TOML: {err}"),
            }),
        )
    })?;
    Ok(Json(ConfigViewResponse {
        ok: true,
        toml,
        source_path,
        config_hash: meta.config_hash.clone(),
        loaded_at_unix: meta.loaded_at_unix,
    }))
}

/// Analytics dashboard snapshot: request totals, latency percentiles,
/// per-source and per-zoom counts, a request heatmap, the most requested
/// tiles, recent requests and the global tile cache state.
async fn admin_stats(
    State(shared): State<SharedState>,
) -> Json<crate::metrics::dashboard::DashboardSnapshot> {
    let state = shared.load();
    let cache = state
        .sources
        .cache()
        .map(|cache| crate::metrics::dashboard::CacheSnapshot {
            entries: cache.entry_count(),
            size_bytes: cache.weighted_size(),
            capacity_bytes: cache.capacity_bytes(),
        });
    Json(crate::metrics::dashboard::snapshot(cache))
}

async fn admin_cache_flush(State(shared): State<SharedState>) -> Json<CacheFlushResponse> {
    let state = shared.load();
    if let Some(cache) = state.sources.cache() {
        let entries = cache.entry_count();
        let bytes = cache.weighted_size();
        cache.invalidate_all();
        Json(CacheFlushResponse {
            ok: true,
            invalidated_entries: entries,
            freed_bytes: bytes,
        })
    } else {
        Json(CacheFlushResponse {
            ok: true,
            invalidated_entries: 0,
            freed_bytes: 0,
        })
    }
}

async fn admin_reload(
    State(shared): State<SharedState>,
    Query(query): Query<ReloadQueryParams>,
) -> Result<Json<ReloadResponse>, (StatusCode, Json<ReloadErrorResponse>)> {
    let flush = query.flush.unwrap_or(false);
    match shared.reload(flush).await {
        Ok(result) => Ok(Json(ReloadResponse {
            ok: true,
            reloaded: result.reloaded,
            config_hash: result.config_hash,
            loaded_at_unix: result.loaded_at_unix,
            loaded_sources: result.loaded_sources,
            loaded_styles: result.loaded_styles,
            renderer_enabled: result.renderer_enabled,
            prometheus_listener_active: result.prometheus_listener_active,
            version: env!("CARGO_PKG_VERSION"),
        })),
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ReloadErrorResponse {
                ok: false,
                error: err.to_string(),
            }),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_part_strips_scheme_port_and_path() {
        assert_eq!(host_part("127.0.0.1:9001"), "127.0.0.1");
        assert_eq!(host_part("http://LocalHost:9001/x"), "localhost");
        assert_eq!(host_part("[::1]:9001"), "[::1]");
        assert_eq!(host_part("admin.example.com"), "admin.example.com");
    }

    #[test]
    fn admin_host_allowed_accepts_loopback_and_listed_hosts_only() {
        let extra = vec!["admin.example.com".to_string()];
        for ok in [
            "localhost",
            "127.0.0.1",
            "127.0.0.2",
            "[::1]",
            "admin.example.com",
        ] {
            assert!(admin_host_allowed(ok, &extra), "{ok}");
        }
        for bad in ["attacker.example", "10.0.0.5", "localhost.attacker.example"] {
            assert!(!admin_host_allowed(bad, &extra), "{bad}");
        }
    }
    use crate::config::Config;
    use crate::reload::{
        ReloadController, ReloadMeta, RuntimeSettings, SharedState, build_app_state,
        now_unix_seconds,
    };
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use axum::routing::get;
    use std::fs;
    use std::io::Write;
    use std::sync::Arc;
    use tempfile::NamedTempFile;
    use tower::ServiceExt;

    fn config_with_style(style_path: Option<&std::path::Path>) -> String {
        let mut config = String::from(
            r#"
[server]
host = "127.0.0.1"
port = 0
cors_origins = ["*"]
"#,
        );

        if let Some(path) = style_path {
            config.push_str(&format!(
                r#"

[[styles]]
id = "test-style"
path = "{}"
"#,
                path.display()
            ));
        }

        config
    }

    fn write_style_file() -> anyhow::Result<(NamedTempFile, std::path::PathBuf)> {
        let mut style_file = NamedTempFile::new()?;
        let style_json = r#"{"version":8,"name":"Test","sources":{},"layers":[]}"#;
        style_file.write_all(style_json.as_bytes())?;
        let path = style_file.path().to_path_buf();
        Ok((style_file, path))
    }

    async fn build_shared_state(config_path: &std::path::Path) -> SharedState {
        let load = Config::load_with_metadata(Some(config_path.to_path_buf())).unwrap();
        let runtime = RuntimeSettings {
            ui_enabled: false,
            runtime_host: load.config.server.host.clone(),
            runtime_port: load.config.server.port,
            public_url_override: None,
        };
        let state = build_app_state(&load.config, &runtime).await.unwrap();
        let meta = ReloadMeta {
            config_hash: load.content_hash,
            loaded_at_unix: now_unix_seconds(),
            loaded_sources: state.sources.len(),
            loaded_styles: state.styles.len(),
            renderer_enabled: state.renderer.is_some(),
            prometheus_listener_active: false,
        };

        let controller = Arc::new(ReloadController::new(
            state,
            meta,
            load.config.clone(),
            Some(config_path.to_path_buf()),
            runtime,
        ));

        SharedState::new(controller)
    }

    async fn ping_payload(shared: SharedState) -> serde_json::Value {
        let ping = axum::Router::new()
            .route("/ping", get(ping_check))
            .with_state(shared)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/ping")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ping.status(), StatusCode::OK);
        let ping_body = to_bytes(ping.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&ping_body).unwrap()
    }

    #[tokio::test]
    async fn ping_returns_runtime_metadata_contract() {
        let mut config_file = NamedTempFile::new().unwrap();
        let config = config_with_style(None);
        config_file.write_all(config.as_bytes()).unwrap();

        let shared = build_shared_state(config_file.path()).await;
        let payload = ping_payload(shared).await;

        assert_eq!(payload["status"], "ok");
        assert!(
            payload["config_hash"]
                .as_str()
                .map(|s| !s.is_empty())
                .unwrap_or(false)
        );
        assert!(payload["loaded_at_unix"].as_u64().is_some());
        assert!(payload["loaded_sources"].as_u64().is_some());
        assert!(payload["loaded_styles"].as_u64().is_some());
        assert!(payload["renderer_enabled"].as_bool().is_some());
        assert!(
            payload["version"]
                .as_str()
                .map(|s| !s.is_empty())
                .unwrap_or(false)
        );
    }

    #[tokio::test]
    async fn admin_reload_without_change_reports_noop() {
        let mut config_file = NamedTempFile::new().unwrap();
        let config = config_with_style(None);
        config_file.write_all(config.as_bytes()).unwrap();

        let shared = build_shared_state(config_file.path()).await;
        let before_ping = ping_payload(shared.clone()).await;

        let admin = admin_router(shared.clone());
        let response = admin
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/__admin/reload")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["ok"], true);
        assert_eq!(payload["reloaded"], false);
        assert_eq!(payload["config_hash"], before_ping["config_hash"]);

        let after_ping = ping_payload(shared).await;
        assert_eq!(after_ping["config_hash"], before_ping["config_hash"]);
        assert_eq!(after_ping["loaded_at_unix"], before_ping["loaded_at_unix"]);
    }

    #[tokio::test]
    async fn admin_reload_applies_updated_config() {
        let mut config_file = NamedTempFile::new().unwrap();
        let config = config_with_style(None);
        config_file.write_all(config.as_bytes()).unwrap();

        let shared = build_shared_state(config_file.path()).await;
        let before_ping = ping_payload(shared.clone()).await;
        let admin = admin_router(shared.clone());

        let (_style_file, style_path) = write_style_file().unwrap();
        let updated_config = config_with_style(Some(&style_path));
        fs::write(config_file.path(), updated_config).unwrap();

        let response = admin
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/__admin/reload")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["ok"], true);
        assert_eq!(payload["reloaded"], true);
        assert_eq!(payload["loaded_styles"], 1);
        assert_ne!(payload["config_hash"], before_ping["config_hash"]);

        let after_ping = ping_payload(shared).await;
        assert_eq!(after_ping["loaded_styles"], 1);
        assert_eq!(after_ping["config_hash"], payload["config_hash"]);
        assert_ne!(after_ping["config_hash"], before_ping["config_hash"]);
    }

    #[tokio::test]
    async fn admin_reload_failure_keeps_existing_state() {
        let (_style_file, style_path) = write_style_file().unwrap();
        let mut config_file = NamedTempFile::new().unwrap();
        let config = config_with_style(Some(&style_path));
        config_file.write_all(config.as_bytes()).unwrap();

        let shared = build_shared_state(config_file.path()).await;
        let before_ping = ping_payload(shared.clone()).await;
        let admin = admin_router(shared.clone());

        fs::write(config_file.path(), "this is not valid toml").unwrap();

        let response = admin
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/__admin/reload")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let after_ping = ping_payload(shared).await;
        assert_eq!(after_ping["loaded_styles"], 1);
        assert_eq!(after_ping["config_hash"], before_ping["config_hash"]);
    }

    fn config_with_cache_enabled() -> String {
        String::from(
            r#"
[server]
host = "127.0.0.1"
port = 0
cors_origins = ["*"]

[cache]
enabled = true
"#,
        )
    }

    async fn admin_json(
        shared: SharedState,
        method: &str,
        uri: &str,
    ) -> (StatusCode, serde_json::Value) {
        let admin = admin_router(shared);
        let response = admin
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    #[tokio::test]
    async fn admin_stats_returns_dashboard_snapshot() {
        let mut config_file = NamedTempFile::new().unwrap();
        config_file
            .write_all(config_with_style(None).as_bytes())
            .unwrap();
        let shared = build_shared_state(config_file.path()).await;

        let (status, payload) = admin_json(shared, "GET", "/__admin/stats").await;
        assert_eq!(status, StatusCode::OK);
        for key in [
            "enabled",
            "totals",
            "latency",
            "latency_buckets",
            "per_minute",
            "sources",
            "zooms",
            "heatmap",
            "top_tiles",
            "recent",
        ] {
            assert!(payload.get(key).is_some(), "missing {key}");
        }
        assert_eq!(payload["per_minute"].as_array().map(Vec::len), Some(60));
        // The test config has no `[cache]`, so there is no cache block.
        assert!(payload["cache"].is_null());
    }

    #[tokio::test]
    async fn admin_config_returns_toml_view() {
        let mut config_file = NamedTempFile::new().unwrap();
        config_file
            .write_all(config_with_style(None).as_bytes())
            .unwrap();
        let shared = build_shared_state(config_file.path()).await;

        let (status, payload) = admin_json(shared, "GET", "/__admin/config").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(payload["ok"], true);
        assert!(
            payload["toml"]
                .as_str()
                .is_some_and(|s| s.contains("[server]"))
        );
        assert!(payload["source_path"].as_str().is_some());
        assert!(
            payload["config_hash"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
        );
    }

    #[tokio::test]
    async fn admin_app_serves_admin_routes_and_oauth_routes_with_mcp() {
        let mut config_file = NamedTempFile::new().unwrap();
        config_file
            .write_all(config_with_style(None).as_bytes())
            .unwrap();
        let shared = build_shared_state(config_file.path()).await;
        let app = admin_app(
            shared,
            #[cfg(feature = "mcp")]
            None,
        );

        let config = app
            .clone()
            .oneshot(Request::get("/__admin/config").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(config.status(), StatusCode::OK);

        let clients = app
            .oneshot(
                Request::get("/__admin/oauth/clients")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let expected = if cfg!(feature = "mcp") {
            StatusCode::OK
        } else {
            StatusCode::NOT_FOUND
        };
        assert_eq!(clients.status(), expected);
    }

    #[tokio::test]
    async fn admin_app_rejects_foreign_host_and_origin() {
        let mut config_file = NamedTempFile::new().unwrap();
        config_file
            .write_all(config_with_style(None).as_bytes())
            .unwrap();
        let shared = build_shared_state(config_file.path()).await;
        let app = admin_app(
            shared,
            #[cfg(feature = "mcp")]
            None,
        );
        let status = |headers: &[(&str, &str)]| {
            let mut req = Request::get("/__admin/config");
            for (k, v) in headers {
                req = req.header(*k, *v);
            }
            let app = app.clone();
            async move {
                app.oneshot(req.body(Body::empty()).unwrap())
                    .await
                    .unwrap()
                    .status()
            }
        };
        assert_eq!(status(&[("host", "127.0.0.1:9001")]).await, StatusCode::OK);
        assert_eq!(
            status(&[
                ("host", "localhost:9001"),
                ("origin", "http://localhost:9001")
            ])
            .await,
            StatusCode::OK
        );
        assert_eq!(
            status(&[("host", "attacker.example:9001")]).await,
            StatusCode::FORBIDDEN,
            "DNS rebinding"
        );
        assert_eq!(
            status(&[
                ("host", "127.0.0.1:9001"),
                ("origin", "https://attacker.example")
            ])
            .await,
            StatusCode::FORBIDDEN,
            "cross-site request"
        );
        assert_eq!(
            status(&[("host", "127.0.0.1:9001"), ("origin", "null")]).await,
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn admin_cache_flush_without_cache_reports_zero() {
        let mut config_file = NamedTempFile::new().unwrap();
        config_file
            .write_all(config_with_style(None).as_bytes())
            .unwrap();
        let shared = build_shared_state(config_file.path()).await;

        let (status, payload) = admin_json(shared, "POST", "/__admin/cache/flush").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(payload["ok"], true);
        assert_eq!(payload["invalidated_entries"], 0);
        assert_eq!(payload["freed_bytes"], 0);
    }

    #[tokio::test]
    async fn admin_cache_flush_with_cache_enabled_reports_ok() {
        let mut config_file = NamedTempFile::new().unwrap();
        config_file
            .write_all(config_with_cache_enabled().as_bytes())
            .unwrap();
        let shared = build_shared_state(config_file.path()).await;

        let ping = ping_payload(shared.clone()).await;
        assert_eq!(ping["cache_enabled"], true);

        let (status, payload) = admin_json(shared, "POST", "/__admin/cache/flush").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(payload["ok"], true);
        assert_eq!(payload["invalidated_entries"], 0);
    }
}
