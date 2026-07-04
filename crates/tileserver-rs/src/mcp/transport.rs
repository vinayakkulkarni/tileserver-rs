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
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, WWW_AUTHENTICATE},
};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::{
    StreamableHttpServerConfig, StreamableHttpService,
};
use subtle::ConstantTimeEq;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::config::McpConfig;
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

impl McpAuthMode {
    /// Resolve the runtime authentication mode from `[mcp]` configuration.
    ///
    /// Priority matches the precedence enforced by config validation:
    ///
    /// 1. `oauth.enabled = true` → [`McpAuthMode::OAuth`], requires both
    ///    `oauth.issuer_url` and `oauth.signing_key_path` to be set and
    ///    the signing key to parse as RSA PEM.
    /// 2. `auth_token = Some(_)` → [`McpAuthMode::StaticBearer`].
    /// 3. Otherwise → [`McpAuthMode::None`] (a warning is logged later by
    ///    [`mcp_router`] before the route is mounted).
    ///
    /// # Errors
    ///
    /// Returns an error when:
    /// - `oauth.enabled = true` but `oauth.issuer_url` is `None`.
    /// - `oauth.enabled = true` but `oauth.signing_key_path` is `None`.
    /// - The signing key file cannot be read or parsed as RSA PEM
    ///   (propagated from [`OAuthState::from_file`]).
    pub fn from_config(cfg: &McpConfig) -> anyhow::Result<Self> {
        if cfg.oauth.enabled {
            let issuer = cfg
                .oauth
                .issuer_url
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("`[mcp.oauth].issuer_url` is required"))?;
            let key_path = cfg
                .oauth
                .signing_key_path
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("`[mcp.oauth].signing_key_path` is required"))?;
            let state =
                OAuthState::from_file(issuer.to_string(), key_path, cfg.oauth.token_ttl_secs)?;
            let state = match cfg.oauth.store_path.as_deref() {
                #[cfg(feature = "mcp-persistence")]
                Some(store_path) => {
                    let store = super::auth_store_sqlite::SqliteStore::open(store_path)
                        .map_err(|e| anyhow::anyhow!("open `[mcp.oauth].store_path`: {e}"))?;
                    tracing::info!(path = %store_path.display(), "MCP OAuth store: SQLite");
                    OAuthState {
                        store: std::sync::Arc::new(store),
                        ..state
                    }
                }
                #[cfg(not(feature = "mcp-persistence"))]
                Some(store_path) => anyhow::bail!(
                    "`[mcp.oauth].store_path` is set ({}) but this binary was built \
                     without the `mcp-persistence` feature; rebuild with \
                     `--features mcp,mcp-persistence` or unset store_path",
                    store_path.display()
                ),
                None => state,
            };
            tracing::info!(issuer = %issuer, "MCP OAuth authorization server enabled");
            Ok(Self::OAuth(Box::new(state)))
        } else if let Some(token) = cfg.auth_token.clone() {
            Ok(Self::StaticBearer(token))
        } else {
            Ok(Self::None)
        }
    }
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
) -> Response {
    let expected = token.0.as_bytes();
    let provided = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let ok = match provided {
        Some(actual) => actual.as_bytes().ct_eq(expected).unwrap_u8() == 1,
        None => false,
    };

    if ok {
        next.run(req).await
    } else {
        static_bearer_challenge()
    }
}

fn static_bearer_challenge() -> Response {
    let mut resp = StatusCode::UNAUTHORIZED.into_response();
    resp.headers_mut().insert(
        WWW_AUTHENTICATE,
        HeaderValue::from_static(r#"Bearer realm="mcp", error="invalid_token""#),
    );
    resp
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
    use std::collections::HashMap;
    use tower::ServiceExt;

    use crate::reload::{
        AppState, ReloadController, ReloadMeta, RuntimeSettings, SharedState, now_unix_seconds,
    };
    use crate::sources::SourceManager;
    use crate::styles::StyleManager;

    const ALLOW_ORIGIN_HEADER: &str = "access-control-allow-origin";
    const PREFLIGHT_METHOD_HEADER: &str = "access-control-request-method";
    const PREFLIGHT_HEADERS_HEADER: &str = "access-control-request-headers";

    fn router_with_cors(origins: &[String]) -> Router {
        Router::new()
            .route("/mcp", get(|| async { "ok" }))
            .layer(build_cors_layer(origins))
    }

    fn minimal_shared_state() -> SharedState {
        let state = AppState {
            sources: Arc::new(SourceManager::from_sources(HashMap::new())),
            styles: Arc::new(StyleManager::new()),
            renderer: None,
            base_url: "http://localhost:8080".to_string(),
            render_base_url: "http://127.0.0.1:8080".to_string(),
            ui_enabled: false,
            fonts_dir: None,
            files_dir: None,
            upload_dir: None,
        };
        let meta = ReloadMeta {
            config_hash: "transport-test".to_string(),
            loaded_at_unix: now_unix_seconds(),
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
        let controller = Arc::new(ReloadController::new(
            state,
            meta,
            crate::config::Config::default(),
            None,
            runtime,
        ));
        SharedState::new(controller)
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

    async fn send_get(router: Router, uri: &str) -> axum::http::Response<Body> {
        router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(uri)
                    .body(Body::empty())
                    .expect("build GET request"),
            )
            .await
            .expect("oneshot GET")
    }

    async fn send_get_with_bearer(
        router: Router,
        uri: &str,
        token: &str,
    ) -> axum::http::Response<Body> {
        router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(uri)
                    .header(AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("build authed GET request"),
            )
            .await
            .expect("oneshot authed GET")
    }

    #[tokio::test]
    async fn mcp_router_none_auth_does_not_require_authorization_header() {
        let router = mcp_router(minimal_shared_state(), McpAuthMode::None, &[]);

        let response = send_get(router, "/mcp").await;

        assert_ne!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "no-auth mode must not reject unauthenticated requests"
        );
    }

    #[tokio::test]
    async fn mcp_router_static_bearer_rejects_missing_authorization() {
        let router = mcp_router(
            minimal_shared_state(),
            McpAuthMode::StaticBearer("the-secret".to_string()),
            &[],
        );

        let response = send_get(router, "/mcp").await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn mcp_router_static_bearer_rejects_wrong_token() {
        let router = mcp_router(
            minimal_shared_state(),
            McpAuthMode::StaticBearer("the-secret".to_string()),
            &[],
        );

        let response = send_get_with_bearer(router, "/mcp", "wrong-token").await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn mcp_router_static_bearer_passes_with_matching_token() {
        let router = mcp_router(
            minimal_shared_state(),
            McpAuthMode::StaticBearer("the-secret".to_string()),
            &[],
        );

        let response = send_get_with_bearer(router, "/mcp", "the-secret").await;

        assert_ne!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "matching bearer must pass middleware"
        );
    }

    #[tokio::test]
    async fn mcp_router_static_bearer_rejects_authorization_without_bearer_prefix() {
        let router = mcp_router(
            minimal_shared_state(),
            McpAuthMode::StaticBearer("the-secret".to_string()),
            &[],
        );

        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/mcp")
                    .header(AUTHORIZATION, "the-secret")
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("oneshot");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn mcp_router_oauth_mode_mounts_well_known_metadata_route() {
        let pem = include_bytes!("../../tests/fixtures/oauth_test_key.pem");
        let oauth_state = OAuthState::from_pem("http://localhost:8080".to_string(), pem, 3600)
            .expect("test PEM parses");
        let router = mcp_router(
            minimal_shared_state(),
            McpAuthMode::OAuth(Box::new(oauth_state)),
            &[],
        );

        let response = send_get(router, "/.well-known/oauth-authorization-server").await;

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "oauth mode must expose the RFC 8414 discovery doc"
        );
    }

    #[tokio::test]
    async fn mcp_router_oauth_mode_rejects_mcp_without_bearer() {
        let pem = include_bytes!("../../tests/fixtures/oauth_test_key.pem");
        let oauth_state = OAuthState::from_pem("http://localhost:8080".to_string(), pem, 3600)
            .expect("test PEM parses");
        let router = mcp_router(
            minimal_shared_state(),
            McpAuthMode::OAuth(Box::new(oauth_state)),
            &[],
        );

        let response = send_get(router, "/mcp").await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    use crate::config::McpConfig;

    fn write_test_pem_to_tempfile() -> tempfile::NamedTempFile {
        let pem = include_bytes!("../../tests/fixtures/oauth_test_key.pem");
        let file = tempfile::Builder::new()
            .prefix("mcp-auth-mode-")
            .suffix(".pem")
            .tempfile()
            .expect("create tempfile for signing key");
        std::fs::write(file.path(), pem).expect("write PEM to tempfile");
        file
    }

    #[test]
    fn from_config_none_when_disabled() {
        let cfg = McpConfig::default();

        let mode = McpAuthMode::from_config(&cfg).expect("default config builds None mode");

        assert!(
            matches!(mode, McpAuthMode::None),
            "expected McpAuthMode::None for default config, got {mode:?}"
        );
    }

    #[test]
    fn from_config_static_bearer_when_token_set() {
        let cfg = McpConfig {
            auth_token: Some("the-secret".to_string()),
            ..McpConfig::default()
        };

        let mode = McpAuthMode::from_config(&cfg).expect("static-bearer config builds");

        match mode {
            McpAuthMode::StaticBearer(token) => assert_eq!(token, "the-secret"),
            other => panic!("expected StaticBearer, got {other:?}"),
        }
    }

    #[test]
    fn from_config_oauth_when_oauth_enabled_with_valid_key() {
        let key = write_test_pem_to_tempfile();
        let cfg = McpConfig {
            oauth: crate::config::McpOAuthConfig {
                enabled: true,
                issuer_url: Some("http://localhost:8080".to_string()),
                signing_key_path: Some(key.path().to_path_buf()),
                token_ttl_secs: 3600,
                store_path: None,
            },
            ..McpConfig::default()
        };

        let mode = McpAuthMode::from_config(&cfg).expect("oauth config with valid key builds");

        assert!(
            matches!(mode, McpAuthMode::OAuth(_)),
            "expected McpAuthMode::OAuth, got {mode:?}"
        );
    }

    /// `store_path` + `mcp-persistence` selects the disk-backed SQLite
    /// store instead of the in-memory default. Asserted via the store's
    /// `Debug` representation because `OAuthState.store` is a trait object.
    #[cfg(feature = "mcp-persistence")]
    #[test]
    fn from_config_oauth_uses_sqlite_store_when_store_path_set() {
        let key = write_test_pem_to_tempfile();
        let dir = tempfile::tempdir().expect("tempdir");
        let store_path = dir.path().join("oauth-store.sqlite");
        let cfg = McpConfig {
            oauth: crate::config::McpOAuthConfig {
                enabled: true,
                issuer_url: Some("http://localhost:8080".to_string()),
                signing_key_path: Some(key.path().to_path_buf()),
                store_path: Some(store_path.clone()),
                ..crate::config::McpOAuthConfig::default()
            },
            ..McpConfig::default()
        };

        let mode = McpAuthMode::from_config(&cfg).expect("oauth config with store_path builds");

        let McpAuthMode::OAuth(state) = mode else {
            panic!("expected McpAuthMode::OAuth");
        };
        let store_debug = format!("{:?}", state.store);
        assert!(
            store_debug.contains("SqliteStore"),
            "store must be SqliteStore when store_path is set, got: {store_debug}"
        );
        assert!(
            store_path.exists(),
            "SQLite file must be created at the configured store_path"
        );
    }

    /// Without `store_path` the store stays in-memory (the default) even
    /// when the `mcp-persistence` feature is compiled in.
    #[test]
    fn from_config_oauth_uses_memory_store_without_store_path() {
        let key = write_test_pem_to_tempfile();
        let cfg = McpConfig {
            oauth: crate::config::McpOAuthConfig {
                enabled: true,
                issuer_url: Some("http://localhost:8080".to_string()),
                signing_key_path: Some(key.path().to_path_buf()),
                ..crate::config::McpOAuthConfig::default()
            },
            ..McpConfig::default()
        };

        let mode = McpAuthMode::from_config(&cfg).expect("oauth config builds");

        let McpAuthMode::OAuth(state) = mode else {
            panic!("expected McpAuthMode::OAuth");
        };
        let store_debug = format!("{:?}", state.store);
        assert!(
            store_debug.contains("InMemoryStore"),
            "store must default to InMemoryStore without store_path, got: {store_debug}"
        );
    }

    /// `store_path` set but the binary was built WITHOUT `mcp-persistence`:
    /// fail fast at startup instead of silently running in-memory — an
    /// operator who configured persistence must not lose OAuth state to a
    /// silently ignored key.
    #[cfg(not(feature = "mcp-persistence"))]
    #[test]
    fn from_config_errors_when_store_path_set_without_persistence_feature() {
        let key = write_test_pem_to_tempfile();
        let cfg = McpConfig {
            oauth: crate::config::McpOAuthConfig {
                enabled: true,
                issuer_url: Some("http://localhost:8080".to_string()),
                signing_key_path: Some(key.path().to_path_buf()),
                store_path: Some(std::path::PathBuf::from("/tmp/oauth.sqlite")),
                ..crate::config::McpOAuthConfig::default()
            },
            ..McpConfig::default()
        };

        let err = McpAuthMode::from_config(&cfg)
            .expect_err("store_path without mcp-persistence must fail fast");

        let msg = format!("{err}");
        assert!(
            msg.contains("mcp-persistence"),
            "error must tell the operator to rebuild with mcp-persistence, got: {msg}"
        );
    }

    #[test]
    fn from_config_errors_when_oauth_enabled_without_issuer_url() {
        let key = write_test_pem_to_tempfile();
        let cfg = McpConfig {
            oauth: crate::config::McpOAuthConfig {
                enabled: true,
                issuer_url: None,
                signing_key_path: Some(key.path().to_path_buf()),
                ..crate::config::McpOAuthConfig::default()
            },
            ..McpConfig::default()
        };

        let err = McpAuthMode::from_config(&cfg)
            .expect_err("missing issuer_url must error when oauth enabled");

        let msg = format!("{err}");
        assert!(
            msg.contains("issuer_url"),
            "error must mention `issuer_url`, got: {msg}"
        );
    }

    #[test]
    fn from_config_errors_when_oauth_enabled_without_signing_key() {
        let cfg = McpConfig {
            oauth: crate::config::McpOAuthConfig {
                enabled: true,
                issuer_url: Some("http://localhost:8080".to_string()),
                signing_key_path: None,
                ..crate::config::McpOAuthConfig::default()
            },
            ..McpConfig::default()
        };

        let err = McpAuthMode::from_config(&cfg)
            .expect_err("missing signing_key_path must error when oauth enabled");

        let msg = format!("{err}");
        assert!(
            msg.contains("signing_key_path"),
            "error must mention `signing_key_path`, got: {msg}"
        );
    }
}
