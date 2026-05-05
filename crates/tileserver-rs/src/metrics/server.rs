//! Standalone Axum listener that serves Prometheus exposition at a
//! configurable bind address and path.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use opentelemetry_prometheus_text_exporter::PrometheusExporter;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

#[derive(Clone)]
struct ServerState {
    exporter: Arc<PrometheusExporter>,
}

/// Owned handle for the spawned metrics listener task.
///
/// Drop the handle to abort the task; otherwise the listener runs until
/// graceful shutdown is signalled by the main server.
pub struct MetricsServerHandle {
    pub addr: SocketAddr,
    pub task: JoinHandle<()>,
}

/// Spawn the Prometheus metrics listener.
///
/// Returns `Ok(handle)` on successful bind. The caller is responsible for
/// keeping the [`MetricsServerHandle`] alive for the lifetime of the server
/// (e.g. by storing it in startup state).
pub async fn spawn_metrics_server(
    bind: SocketAddr,
    path: String,
    exporter: PrometheusExporter,
) -> std::io::Result<MetricsServerHandle> {
    let listener = TcpListener::bind(bind).await?;
    let addr = listener.local_addr()?;

    let state = ServerState {
        exporter: Arc::new(exporter),
    };

    let normalized = if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    };

    let router = Router::new()
        .route(&normalized, get(scrape))
        .route("/metrics/health", get(health))
        .with_state(state);

    let task = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!("metrics listener exited with error: {e}");
        }
    });

    tracing::info!(
        bind = %addr,
        path = %normalized,
        "Prometheus metrics endpoint listening"
    );

    Ok(MetricsServerHandle { addr, task })
}

async fn scrape(State(state): State<ServerState>) -> Response {
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    if let Err(e) = state.exporter.export(&mut buf) {
        tracing::warn!("metrics scrape failed: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "metrics scrape failed").into_response();
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
    );

    let body = match String::from_utf8(buf) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("metrics output not utf-8: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "metrics output not utf-8",
            )
                .into_response();
        }
    };

    (StatusCode::OK, headers, body).into_response()
}

async fn health() -> &'static str {
    "OK"
}
