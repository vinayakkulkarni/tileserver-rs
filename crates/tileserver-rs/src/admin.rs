//! Admin server for hot-reload and runtime diagnostics (`/ping`, `/__admin/reload`).

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::post,
};

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

pub fn admin_router(state: SharedState) -> Router {
    Router::new()
        .route("/__admin/reload", post(admin_reload))
        .route("/__admin/cache/flush", post(admin_cache_flush))
        .with_state(state)
}

pub async fn ping_check(State(shared): State<SharedState>) -> Json<PingResponse> {
    let meta = shared.meta();
    let state = shared.load();
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
    })
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
}
