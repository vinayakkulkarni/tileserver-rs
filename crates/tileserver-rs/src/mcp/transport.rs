//! Transport wiring for the MCP server — stdio + Streamable HTTP.
//!
//! The HTTP path mounts a `tower::Service` at `/mcp` inside the existing
//! axum router (same listener as the main HTTP API). Session management is
//! handled by [`LocalSessionManager`] — `Mcp-Session-Id` headers are
//! threaded automatically.
//!
//! The stdio path is exposed via the `mcp-stdio` subcommand and runs the
//! server against `(stdin, stdout)` — exactly what Claude Desktop and other
//! local MCP clients spawn.

use std::sync::Arc;

use axum::Router;
use axum::extract::Request;
use axum::http::{StatusCode, header::AUTHORIZATION};
use axum::middleware::{self, Next};
use axum::response::Response;
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::{
    StreamableHttpServerConfig, StreamableHttpService,
};

use crate::error::{Result, TileServerError};
use crate::mcp::handlers::McpHandler;
use crate::reload::{AppState, SharedState};

/// Bearer token expected on the `Authorization` header when MCP auth is
/// enabled. Wrapping `String` in a newtype prevents accidental confusion
/// with other config strings at call sites and lets us implement bespoke
/// `Debug` (masking the secret) later without changing the public API.
#[derive(Clone, Debug)]
struct McpBearerToken(Arc<String>);

/// Build an axum router exposing the MCP Streamable HTTP service.
///
/// The returned router is meant to be `.merge`d into the main application
/// router so it shares the same TCP listener, CORS layer, and metrics
/// middleware. `Mcp-Session-Id` headers are handled by the session manager.
///
/// When `auth_token` is `Some`, a bearer-token middleware is applied to the
/// `/mcp` route and rejects any request whose `Authorization` header does
/// not exactly match `Bearer <token>`.
pub fn mcp_router(shared: SharedState, auth_token: Option<String>) -> Router {
    let factory_state = shared.clone();
    let svc = StreamableHttpService::new(
        move || Ok(McpHandler::new(factory_state.load())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );

    let mut mcp = Router::new().nest_service("/mcp", svc);

    if let Some(token) = auth_token {
        let state = McpBearerToken(Arc::new(token));
        mcp = mcp.layer(middleware::from_fn_with_state(state, bearer_auth));
    }

    mcp
}

async fn bearer_auth(
    axum::extract::State(token): axum::extract::State<McpBearerToken>,
    req: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    let expected = token.0.as_str();
    let provided = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match provided {
        Some(actual) if actual == expected => Ok(next.run(req).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Run the MCP server over stdio until the client disconnects.
///
/// Used by the `mcp-stdio` subcommand; blocks the calling task until the
/// session ends.
///
/// # Errors
///
/// Returns [`TileServerError::Internal`] if the rmcp service fails to start
/// or terminates with an error.
pub async fn run_stdio(state: Arc<AppState>) -> Result<()> {
    let handler = McpHandler::new(state);
    let running = handler.serve(stdio()).await.map_err(|e| {
        TileServerError::Internal(anyhow::anyhow!("failed to start MCP stdio service: {e}"))
    })?;
    running.waiting().await.map_err(|e| {
        TileServerError::Internal(anyhow::anyhow!("MCP stdio service exited abnormally: {e}"))
    })?;
    Ok(())
}
