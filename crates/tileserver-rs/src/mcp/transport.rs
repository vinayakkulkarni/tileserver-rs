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
use std::time::Duration;

use axum::Router;
use axum::extract::Request;
use axum::http::{
    HeaderValue, Method, StatusCode,
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
};
use axum::middleware::{self, Next};
use axum::response::Response;
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::{
    StreamableHttpServerConfig, StreamableHttpService,
};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::error::{Result, TileServerError};
use crate::mcp::auth::{OAuthState, oauth_router, validate_oauth_bearer};
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
/// router so it shares the same TCP listener and metrics middleware.
/// `Mcp-Session-Id` headers are handled by the session manager.
///
/// CORS is enforced per-MCP using `cors_origins`:
/// - `["*"]` or empty list → wildcard via [`AllowOrigin::any`] (a warning is
///   logged when an explicit `["*"]` is configured).
/// - Explicit origins → [`AllowOrigin::list`] with invalid entries skipped
///   and logged at `warn` level. If every entry is invalid, falls back to
///   wildcard with a warning.
///
/// Authentication mode for the `/mcp` route. `auth_token` and `oauth` are
/// mutually exclusive — config validation rejects the combination before
/// this function is called.
#[derive(Debug, Default)]
#[non_exhaustive]
pub enum McpAuthMode {
    /// No authentication. A warning is logged at startup so operators
    /// don't accidentally expose `/mcp` to the open internet.
    #[default]
    None,
    /// Single shared bearer token compared verbatim against the
    /// `Authorization: Bearer …` header.
    StaticBearer(String),
    /// Full OAuth 2.0 authorization server with RFC 7591 DCR.
    OAuth(Box<OAuthState>),
}

/// When `auth` is [`McpAuthMode::StaticBearer`], a bearer-token middleware
/// is applied to the `/mcp` route. When [`McpAuthMode::OAuth`], the OAuth
/// discovery / token / register routes are also mounted at the router root
/// and JWT validation is layered on `/mcp`.
pub fn mcp_router(shared: SharedState, auth: McpAuthMode, cors_origins: &[String]) -> Router {
    let factory_state = shared.clone();
    let svc = StreamableHttpService::new(
        move || Ok(McpHandler::new(factory_state.load())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );

    let mut mcp = Router::new().nest_service("/mcp", svc);

    let mut oauth_routes: Option<Router> = None;

    match auth {
        McpAuthMode::None => {
            tracing::warn!(
                "MCP `/mcp` is mounted without authentication. Set `[mcp].auth_token` or enable `[mcp.oauth]` before exposing this server publicly.",
            );
        }
        McpAuthMode::StaticBearer(token) => {
            let state = McpBearerToken(Arc::new(token));
            mcp = mcp.layer(middleware::from_fn_with_state(state, bearer_auth));
        }
        McpAuthMode::OAuth(state) => {
            let inner = *state;
            mcp = mcp.layer(middleware::from_fn_with_state(
                inner.clone(),
                validate_oauth_bearer,
            ));
            oauth_routes = Some(oauth_router(inner));
        }
    }

    let merged = if let Some(routes) = oauth_routes {
        mcp.merge(routes)
    } else {
        mcp
    };

    merged.layer(build_cors_layer(cors_origins))
}

/// Build a [`CorsLayer`] for the `/mcp` route from a configured origin list.
///
/// Mirrors the wildcard-vs-explicit logic used by the main HTTP server
/// (see `main.rs` CORS construction) but uses MCP-specific headers
/// (`AUTHORIZATION` for bearer-token clients) and methods (`POST` for the
/// JSON-RPC payload).
fn build_cors_layer(cors_origins: &[String]) -> CorsLayer {
    let allow_origin = if cors_origins.is_empty() || cors_origins.iter().any(|o| o == "*") {
        if !cors_origins.is_empty() {
            tracing::warn!(
                "MCP CORS configured with wildcard (*). Consider restricting origins in production."
            );
        }
        AllowOrigin::any()
    } else {
        let origins: Vec<HeaderValue> = cors_origins
            .iter()
            .filter_map(|o| {
                o.parse::<HeaderValue>().ok().or_else(|| {
                    tracing::warn!("Invalid MCP CORS origin '{}', skipping", o);
                    None
                })
            })
            .collect();

        if origins.is_empty() {
            tracing::warn!("No valid MCP CORS origins configured, defaulting to wildcard");
            AllowOrigin::any()
        } else {
            AllowOrigin::list(origins)
        }
    };

    CorsLayer::new()
        .allow_headers([ACCEPT, CONTENT_TYPE, AUTHORIZATION])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .max_age(Duration::from_secs(86400))
        .allow_origin(allow_origin)
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

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::Body;
    use axum::http::{Request, header::ORIGIN};
    use axum::routing::get;
    use tower::ServiceExt;

    const ALLOW_ORIGIN_HEADER: &str = "access-control-allow-origin";
    const PREFLIGHT_METHOD_HEADER: &str = "access-control-request-method";
    const PREFLIGHT_HEADERS_HEADER: &str = "access-control-request-headers";

    fn router_with_cors(origins: &[String]) -> Router {
        Router::new()
            .route("/mcp", get(|| async { "ok" }))
            .layer(build_cors_layer(origins))
    }

    async fn send_preflight(router: Router, origin: &str) -> axum::http::Response<Body> {
        router
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/mcp")
                    .header(ORIGIN, origin)
                    .header(PREFLIGHT_METHOD_HEADER, "POST")
                    .header(PREFLIGHT_HEADERS_HEADER, "authorization,content-type")
                    .body(Body::empty())
                    .expect("build preflight request"),
            )
            .await
            .expect("oneshot preflight")
    }

    #[tokio::test]
    async fn cors_wildcard_when_list_contains_star() {
        let router = router_with_cors(&["*".to_string()]);

        let response = send_preflight(router, "https://any.example.com").await;

        let header = response
            .headers()
            .get(ALLOW_ORIGIN_HEADER)
            .expect("allow-origin header present");
        assert_eq!(header, "*");
    }

    #[tokio::test]
    async fn cors_wildcard_falls_back_when_list_empty() {
        let router = router_with_cors(&[]);

        let response = send_preflight(router, "https://any.example.com").await;

        let header = response
            .headers()
            .get(ALLOW_ORIGIN_HEADER)
            .expect("allow-origin header present");
        assert_eq!(header, "*");
    }

    #[tokio::test]
    async fn cors_explicit_list_echoes_matching_origin() {
        let allowed = "https://claude.ai";
        let router = router_with_cors(&[allowed.to_string(), "https://app.cursor.com".to_string()]);

        let response = send_preflight(router, allowed).await;

        let header = response
            .headers()
            .get(ALLOW_ORIGIN_HEADER)
            .expect("allow-origin header present");
        assert_eq!(header.to_str().expect("ascii origin"), allowed);
    }

    #[tokio::test]
    async fn cors_invalid_origin_is_skipped_keeping_remaining() {
        let allowed = "https://claude.ai";
        // Newline is not a legal `HeaderValue` byte — parsing must skip it,
        // emit a `warn!`, and continue with the surviving origin.
        let router = router_with_cors(&["bad\norigin".to_string(), allowed.to_string()]);

        let response = send_preflight(router, allowed).await;

        let header = response
            .headers()
            .get(ALLOW_ORIGIN_HEADER)
            .expect("allow-origin header present");
        assert_eq!(header.to_str().expect("ascii origin"), allowed);
    }

    #[tokio::test]
    async fn cors_all_invalid_origins_fall_back_to_wildcard() {
        let router = router_with_cors(&["bad\norigin1".to_string(), "another\rbad".to_string()]);

        let response = send_preflight(router, "https://any.example.com").await;

        let header = response
            .headers()
            .get(ALLOW_ORIGIN_HEADER)
            .expect("allow-origin header present");
        assert_eq!(header, "*");
    }
}
