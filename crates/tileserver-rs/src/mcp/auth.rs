//! OAuth 2.0 authorization server for the MCP HTTP transport.
//!
//! Implements RFC 7591 (Dynamic Client Registration), RFC 8414
//! (Authorization Server Metadata), and the MCP-spec authorization flow
//! used by claude.ai Custom Connectors. Access tokens are JWTs signed
//! with RS256 using a 2048-bit RSA key loaded from disk at startup.
//!
//! # Endpoints
//!
//! | Path | Method | Purpose |
//! |---|---|---|
//! | `/.well-known/oauth-authorization-server` | GET | RFC 8414 discovery |
//! | `/.well-known/oauth-protected-resource` | GET | RFC 9728 metadata |
//! | `/register` | POST | RFC 7591 DCR |
//! | `/authorize` | GET | Render consent page |
//! | `/approve` | POST | Issue auth code (consent submitted) |
//! | `/token` | POST | Exchange code or refresh token |
//!
//! # PKCE
//!
//! Only `S256` is accepted (the MCP spec rejects `plain`). Verification:
//! `BASE64URL-NO-PAD(SHA256(verifier)) == stored_challenge`.
//!
//! # Token store
//!
//! The OAuth store (registered clients, outstanding auth codes, refresh
//! tokens) is held entirely in memory. **Restarting the server invalidates
//! every issued token** — clients must re-authorize. This is acceptable
//! for a tile server; a database-backed store would be a future
//! improvement.
//!
//! # References
//!
//! - <https://spec.modelcontextprotocol.io/specification/draft/basic/authorization/>
//! - <https://claude.com/docs/connectors/building/authentication>
//! - <https://datatracker.ietf.org/doc/html/rfc7591> (DCR)
//! - <https://datatracker.ietf.org/doc/html/rfc8414> (Authorization Server Metadata)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Json;
use axum::Router;
use axum::body::Body;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderValue, Request, StatusCode, header};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode,
    errors::Error as JwtError,
};
use rsa::RsaPrivateKey;
use rsa::pkcs1::{DecodeRsaPrivateKey, EncodeRsaPublicKey, LineEnding};
use rsa::pkcs8::DecodePrivateKey;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Maximum access-token lifetime in seconds (24 hours). Larger values from
/// config are clamped down with a warning.
const MAX_TOKEN_TTL_SECS: u64 = 86_400;

/// Lifetime of an outstanding authorization code before it is purged on
/// next access. RFC 6749 recommends 10 minutes.
const AUTH_CODE_TTL_SECS: u64 = 600;

/// Lifetime of an issued refresh token (30 days).
const REFRESH_TOKEN_TTL_SECS: u64 = 60 * 60 * 24 * 30;

/// Errors raised while building or operating the OAuth authorization
/// server.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OAuthError {
    /// The configured signing key could not be parsed as RSA PEM.
    #[error("invalid RSA signing key: {0}")]
    InvalidSigningKey(#[from] JwtError),
    /// Failed to read the signing key from disk.
    #[error("failed to read signing key from disk: {0}")]
    SigningKeyIo(#[from] std::io::Error),
    /// The configured private key could not be parsed by the `rsa` crate.
    #[error("could not parse RSA private key: {0}")]
    PrivateKey(String),
    /// Failed to encode the derived public key to PEM.
    #[error("could not encode derived public key: {0}")]
    PublicKeyEncode(String),
}

/// JWT claims embedded in every access token issued by this server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// Subject — the registered `client_id` that owns this token.
    pub sub: String,
    /// Issuer — matches [`OAuthState::issuer_url`].
    pub iss: String,
    /// Audience — always `"mcp"` for this server.
    pub aud: String,
    /// Issued-at — seconds since the Unix epoch.
    pub iat: u64,
    /// Expires-at — seconds since the Unix epoch.
    pub exp: u64,
    /// Scope — currently always `"mcp"`.
    pub scope: String,
    /// JWT ID — random per token to guarantee uniqueness across issuances
    /// inside the same second. Optional in encoded tokens to keep
    /// hand-crafted test fixtures small.
    #[serde(default)]
    pub jti: Option<String>,
}

/// A client registered via RFC 7591 Dynamic Client Registration.
#[derive(Clone, Debug)]
pub struct RegisteredClient {
    /// Public client identifier.
    pub client_id: String,
    /// Optional confidential secret (omitted for public PKCE clients).
    pub client_secret: Option<String>,
    /// Approved redirect URIs.
    pub redirect_uris: Vec<String>,
    /// Display name supplied at registration time.
    pub client_name: Option<String>,
}

/// Bound authorization code awaiting exchange at `/token`.
#[derive(Clone, Debug)]
struct AuthCode {
    client_id: String,
    redirect_uri: String,
    code_challenge: String,
    scope: String,
    expires_at: u64,
}

/// Issued refresh token kept in the store until expiry or rotation.
#[derive(Clone, Debug)]
struct RefreshTokenEntry {
    client_id: String,
    scope: String,
    expires_at: u64,
}

/// In-memory backing store for the authorization server.
#[derive(Default, Debug)]
pub struct OAuthStore {
    /// Registered DCR clients keyed by `client_id`.
    pub clients: HashMap<String, RegisteredClient>,
    /// Outstanding (un-redeemed) authorization codes.
    auth_codes: HashMap<String, AuthCode>,
    /// Outstanding refresh tokens.
    refresh_tokens: HashMap<String, RefreshTokenEntry>,
}

/// Shared, cloneable state for the authorization-server routes.
#[derive(Clone)]
pub struct OAuthState {
    /// Public issuer URL — every endpoint URL in the discovery doc is
    /// derived from this.
    pub issuer_url: String,
    /// RS256 signing key wrapped in `Arc` so it's cheap to clone.
    pub encoding_key: Arc<EncodingKey>,
    /// RS256 verification key extracted from the same PEM.
    pub decoding_key: Arc<DecodingKey>,
    /// Access-token lifetime (clamped to [`MAX_TOKEN_TTL_SECS`]).
    pub token_ttl: Duration,
    /// In-memory client + code store.
    pub store: Arc<RwLock<OAuthStore>>,
}

impl std::fmt::Debug for OAuthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthState")
            .field("issuer_url", &self.issuer_url)
            .field("token_ttl", &self.token_ttl)
            .field("encoding_key", &"<redacted>")
            .field("decoding_key", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl OAuthState {
    /// Build a new [`OAuthState`] from an RSA PEM (PKCS#1 or PKCS#8). The
    /// same PEM is used to derive both the encoding and decoding keys.
    ///
    /// # Errors
    ///
    /// Returns [`OAuthError::InvalidSigningKey`] when the PEM cannot be
    /// parsed as an RSA private key.
    pub fn from_pem(
        issuer_url: String,
        signing_key_pem: &[u8],
        token_ttl_secs: u64,
    ) -> Result<Self, OAuthError> {
        let encoding_key = EncodingKey::from_rsa_pem(signing_key_pem)?;
        let public_key_pem = derive_public_key_pem(signing_key_pem)?;
        let decoding_key = DecodingKey::from_rsa_pem(public_key_pem.as_bytes())?;
        let clamped_ttl = if token_ttl_secs > MAX_TOKEN_TTL_SECS {
            tracing::warn!(
                requested = token_ttl_secs,
                max = MAX_TOKEN_TTL_SECS,
                "MCP OAuth token_ttl_secs exceeds 24h cap; clamping",
            );
            MAX_TOKEN_TTL_SECS
        } else {
            token_ttl_secs
        };
        Ok(Self {
            issuer_url,
            encoding_key: Arc::new(encoding_key),
            decoding_key: Arc::new(decoding_key),
            token_ttl: Duration::from_secs(clamped_ttl),
            store: Arc::new(RwLock::new(OAuthStore::default())),
        })
    }

    /// Load an [`OAuthState`] by reading the signing key from disk.
    ///
    /// # Errors
    ///
    /// Returns [`OAuthError::SigningKeyIo`] if the file cannot be read or
    /// [`OAuthError::InvalidSigningKey`] if it isn't a valid RSA PEM.
    pub fn from_file(
        issuer_url: String,
        signing_key_path: &std::path::Path,
        token_ttl_secs: u64,
    ) -> Result<Self, OAuthError> {
        let pem = std::fs::read(signing_key_path)?;
        Self::from_pem(issuer_url, &pem, token_ttl_secs)
    }
}

/// Construct the axum router exposing every OAuth endpoint, with state
/// embedded. The caller should `.merge()` this into the main MCP router.
pub fn oauth_router(state: OAuthState) -> Router {
    Router::new()
        .route(
            "/.well-known/oauth-authorization-server",
            get(authorization_server_metadata),
        )
        .route(
            "/.well-known/oauth-protected-resource",
            get(protected_resource_metadata),
        )
        .route("/register", post(register))
        .route("/authorize", get(authorize))
        .route("/approve", post(approve))
        .route("/token", post(token))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

async fn authorization_server_metadata(State(state): State<OAuthState>) -> Json<serde_json::Value> {
    let issuer = state.issuer_url.as_str();
    Json(json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "registration_endpoint": format!("{issuer}/register"),
        "scopes_supported": ["mcp"],
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none", "client_secret_post"],
    }))
}

async fn protected_resource_metadata(State(state): State<OAuthState>) -> Json<serde_json::Value> {
    Json(json!({
        "resource": state.issuer_url,
        "authorization_servers": [state.issuer_url],
        "bearer_methods_supported": ["header"],
    }))
}

// ---------------------------------------------------------------------------
// Registration (RFC 7591)
// ---------------------------------------------------------------------------

/// RFC 7591 Dynamic Client Registration request body (subset).
#[derive(Debug, Deserialize)]
struct RegisterRequest {
    #[serde(default)]
    client_name: Option<String>,
    #[serde(default)]
    redirect_uris: Vec<String>,
    #[serde(default)]
    grant_types: Vec<String>,
    #[serde(default)]
    token_endpoint_auth_method: Option<String>,
}

async fn register(
    State(state): State<OAuthState>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    if req.redirect_uris.is_empty() {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "redirect_uris must contain at least one URI",
        );
    }

    let client_id = format!("client-{}", Uuid::new_v4());
    let confidential = req.token_endpoint_auth_method.as_deref() == Some("client_secret_post");
    let client_secret = confidential.then(|| random_string(32));
    let issued_at = now_secs();

    let client = RegisteredClient {
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        redirect_uris: req.redirect_uris.clone(),
        client_name: req.client_name.clone(),
    };

    state
        .store
        .write()
        .await
        .clients
        .insert(client_id.clone(), client);

    (
        StatusCode::CREATED,
        Json(json!({
            "client_id": client_id,
            "client_secret": client_secret,
            "client_id_issued_at": issued_at,
            "redirect_uris": req.redirect_uris,
            "grant_types": if req.grant_types.is_empty() {
                vec!["authorization_code".to_string(), "refresh_token".to_string()]
            } else {
                req.grant_types
            },
            "token_endpoint_auth_method": req.token_endpoint_auth_method.unwrap_or_else(|| "none".to_string()),
            "client_name": req.client_name,
        })),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Authorization code (consent)
// ---------------------------------------------------------------------------

/// Query parameters accepted by `GET /authorize`.
#[derive(Debug, Deserialize)]
struct AuthorizeQuery {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    state: Option<String>,
    code_challenge: String,
    code_challenge_method: String,
}

async fn authorize(State(state): State<OAuthState>, Query(q): Query<AuthorizeQuery>) -> Response {
    if q.response_type != "code" {
        return error_json(
            StatusCode::BAD_REQUEST,
            "unsupported_response_type",
            "only response_type=code is supported",
        );
    }
    if q.code_challenge_method != "S256" {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "only code_challenge_method=S256 is supported",
        );
    }

    let client = {
        let store = state.store.read().await;
        store.clients.get(&q.client_id).cloned()
    };
    let Some(client) = client else {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_client",
            "unknown client_id",
        );
    };
    if !client.redirect_uris.contains(&q.redirect_uri) {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "redirect_uri not registered for this client",
        );
    }

    let scope = q.scope.unwrap_or_else(|| "mcp".to_string());
    let state_value = q.state.unwrap_or_default();
    let consent = render_consent_page(
        &q.client_id,
        client.client_name.as_deref().unwrap_or("MCP client"),
        &q.redirect_uri,
        &scope,
        &state_value,
        &q.code_challenge,
    );
    Html(consent).into_response()
}

/// Hand-rolled HTML consent page. Plain `format!` avoids dragging in a
/// template engine and keeps the surface small enough to audit.
fn render_consent_page(
    client_id: &str,
    client_name: &str,
    redirect_uri: &str,
    scope: &str,
    state: &str,
    code_challenge: &str,
) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>Authorize {client_name}</title></head>
<body style="font-family:system-ui;max-width:480px;margin:48px auto;line-height:1.5">
  <h1>Authorize {client_name}</h1>
  <p>The application <code>{client_name}</code> would like to access this MCP server with scope <code>{scope}</code>.</p>
  <form method="post" action="/approve">
    <input type="hidden" name="client_id" value="{client_id}">
    <input type="hidden" name="redirect_uri" value="{redirect_uri}">
    <input type="hidden" name="scope" value="{scope}">
    <input type="hidden" name="state" value="{state}">
    <input type="hidden" name="code_challenge" value="{code_challenge}">
    <input type="hidden" name="code_challenge_method" value="S256">
    <button type="submit" name="approved" value="true">Approve</button>
    <button type="submit" name="approved" value="false">Deny</button>
  </form>
</body>
</html>"#,
        client_id = html_escape(client_id),
        client_name = html_escape(client_name),
        redirect_uri = html_escape(redirect_uri),
        scope = html_escape(scope),
        state = html_escape(state),
        code_challenge = html_escape(code_challenge),
    )
}

/// Minimal HTML escaper — sufficient for the consent page's attribute and
/// text contexts.
fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Form body submitted by the consent page.
#[derive(Debug, Deserialize)]
struct ApproveForm {
    client_id: String,
    redirect_uri: String,
    #[serde(default)]
    scope: String,
    #[serde(default)]
    state: String,
    code_challenge: String,
    code_challenge_method: String,
    approved: String,
}

async fn approve(State(state): State<OAuthState>, Form(form): Form<ApproveForm>) -> Response {
    if form.code_challenge_method != "S256" {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "only code_challenge_method=S256 is supported",
        );
    }

    if form.approved != "true" {
        let mut url = form.redirect_uri.clone();
        url.push_str("?error=access_denied");
        if !form.state.is_empty() {
            url.push_str("&state=");
            url.push_str(&urlencoding::encode(&form.state));
        }
        return Redirect::to(&url).into_response();
    }

    let client = {
        let store = state.store.read().await;
        store.clients.get(&form.client_id).cloned()
    };
    let Some(client) = client else {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_client",
            "unknown client_id",
        );
    };
    if !client.redirect_uris.contains(&form.redirect_uri) {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_redirect_uri",
            "redirect_uri not registered for this client",
        );
    }

    let code = format!("code-{}", Uuid::new_v4());
    let entry = AuthCode {
        client_id: form.client_id.clone(),
        redirect_uri: form.redirect_uri.clone(),
        code_challenge: form.code_challenge.clone(),
        scope: if form.scope.is_empty() {
            "mcp".to_string()
        } else {
            form.scope.clone()
        },
        expires_at: now_secs() + AUTH_CODE_TTL_SECS,
    };
    state
        .store
        .write()
        .await
        .auth_codes
        .insert(code.clone(), entry);

    let mut url = form.redirect_uri.clone();
    url.push_str("?code=");
    url.push_str(&urlencoding::encode(&code));
    if !form.state.is_empty() {
        url.push_str("&state=");
        url.push_str(&urlencoding::encode(&form.state));
    }
    Redirect::to(&url).into_response()
}

// ---------------------------------------------------------------------------
// Token exchange
// ---------------------------------------------------------------------------

/// `application/x-www-form-urlencoded` body accepted by `/token`.
#[derive(Debug, Deserialize)]
struct TokenForm {
    grant_type: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    redirect_uri: Option<String>,
    #[serde(default)]
    client_id: Option<String>,
    #[serde(default)]
    code_verifier: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
}

async fn token(State(state): State<OAuthState>, request: Request<Body>) -> Response {
    let bytes = match axum::body::to_bytes(request.into_body(), 64 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return error_json(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "could not read request body",
            );
        }
    };
    let form: TokenForm = match serde_urlencoded::from_bytes(&bytes) {
        Ok(f) => f,
        Err(e) => {
            return error_json(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                &format!("malformed form: {e}"),
            );
        }
    };

    match form.grant_type.as_str() {
        "authorization_code" => token_authorization_code(state, form).await,
        "refresh_token" => token_refresh(state, form).await,
        other => error_json(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            &format!("unsupported grant_type: {other}"),
        ),
    }
}

async fn token_authorization_code(state: OAuthState, form: TokenForm) -> Response {
    let Some(code) = form.code.as_deref() else {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "code is required",
        );
    };
    let Some(verifier) = form.code_verifier.as_deref() else {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "code_verifier is required (PKCE S256)",
        );
    };
    let Some(client_id) = form.client_id.as_deref() else {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "client_id is required",
        );
    };
    let Some(redirect_uri) = form.redirect_uri.as_deref() else {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "redirect_uri is required",
        );
    };

    let entry = take_auth_code(&state, code).await;
    let Some(entry) = entry else {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "unknown or already-used authorization code",
        );
    };
    if entry.expires_at < now_secs() {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "authorization code has expired",
        );
    }
    if entry.client_id != client_id || entry.redirect_uri != redirect_uri {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "code does not match this client/redirect",
        );
    }

    // PKCE S256 verification: BASE64URL-NO-PAD(SHA256(verifier)) must
    // equal the challenge captured at consent.
    let digest = Sha256::digest(verifier.as_bytes());
    let computed = URL_SAFE_NO_PAD.encode(digest);
    if computed != entry.code_challenge {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "PKCE verifier does not match challenge",
        );
    }

    let access_token = match issue_access_token(&state, &entry.client_id, &entry.scope) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("failed to sign JWT: {e}");
            return error_json(
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error",
                "could not issue access token",
            );
        }
    };
    let refresh_token = issue_refresh_token(&state, &entry.client_id, &entry.scope).await;

    Json(json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": state.token_ttl.as_secs(),
        "refresh_token": refresh_token,
        "scope": entry.scope,
    }))
    .into_response()
}

async fn token_refresh(state: OAuthState, form: TokenForm) -> Response {
    let Some(refresh) = form.refresh_token.as_deref() else {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "refresh_token is required",
        );
    };

    let entry = take_refresh_token(&state, refresh).await;
    let Some(entry) = entry else {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "unknown or already-used refresh token",
        );
    };
    if entry.expires_at < now_secs() {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "refresh token has expired",
        );
    }
    if let Some(client_id) = form.client_id.as_deref()
        && client_id != entry.client_id
    {
        return error_json(
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            "refresh token does not match client_id",
        );
    }

    let access_token = match issue_access_token(&state, &entry.client_id, &entry.scope) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("failed to sign JWT during refresh: {e}");
            return error_json(
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error",
                "could not issue access token",
            );
        }
    };
    let new_refresh = rotate_refresh_token(&state, &entry.client_id, &entry.scope).await;

    Json(json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": state.token_ttl.as_secs(),
        "refresh_token": new_refresh,
        "scope": entry.scope,
    }))
    .into_response()
}

fn issue_access_token(
    state: &OAuthState,
    client_id: &str,
    scope: &str,
) -> Result<String, JwtError> {
    let iat = now_secs();
    let exp = iat.saturating_add(state.token_ttl.as_secs());
    let claims = AccessTokenClaims {
        sub: client_id.to_string(),
        iss: state.issuer_url.clone(),
        aud: "mcp".to_string(),
        iat,
        exp,
        scope: scope.to_string(),
        jti: Some(Uuid::new_v4().to_string()),
    };
    encode(&Header::new(Algorithm::RS256), &claims, &state.encoding_key)
}

/// OAuth 2.1 mandates refresh-token rotation for public clients — every
/// successful refresh exchange must return a brand-new refresh token and
/// invalidate the old one. Callers reach this via [`rotate_refresh_token`]
/// (after consuming the old entry) or directly during code exchange.
async fn issue_refresh_token(state: &OAuthState, client_id: &str, scope: &str) -> String {
    let refresh = format!("rt-{}", Uuid::new_v4());
    let entry = RefreshTokenEntry {
        client_id: client_id.to_string(),
        scope: scope.to_string(),
        expires_at: now_secs() + REFRESH_TOKEN_TTL_SECS,
    };
    state
        .store
        .write()
        .await
        .refresh_tokens
        .insert(refresh.clone(), entry);
    refresh
}

// ---------------------------------------------------------------------------
// Bearer-token middleware
// ---------------------------------------------------------------------------

/// axum middleware that requires a valid RS256 JWT in `Authorization:
/// Bearer …`. Failures return `401` with an RFC 6750-compliant
/// `WWW-Authenticate` challenge.
///
/// # Errors
///
/// Returns the response itself; never propagates a `StatusCode` because
/// the response carries a `WWW-Authenticate` header that the simple
/// status-code return path can't express.
pub async fn validate_oauth_bearer(
    State(state): State<OAuthState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let header_value = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let Some(token) = header_value else {
        return bearer_challenge(
            StatusCode::UNAUTHORIZED,
            "invalid_token",
            "missing bearer token",
        );
    };

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&["mcp"]);
    validation.set_issuer(&[state.issuer_url.as_str()]);
    match decode::<AccessTokenClaims>(token, &state.decoding_key, &validation) {
        Ok(_) => next.run(req).await,
        Err(e) => {
            tracing::debug!("rejected MCP bearer: {e}");
            bearer_challenge(
                StatusCode::UNAUTHORIZED,
                "invalid_token",
                "token failed validation",
            )
        }
    }
}

fn bearer_challenge(status: StatusCode, error: &str, description: &str) -> Response {
    let challenge =
        format!("Bearer realm=\"mcp\", error=\"{error}\", error_description=\"{description}\"",);
    let header_value = HeaderValue::from_str(&challenge)
        .unwrap_or_else(|_| HeaderValue::from_static("Bearer realm=\"mcp\""));
    let mut resp = (
        status,
        Json(json!({"error": error, "error_description": description})),
    )
        .into_response();
    resp.headers_mut()
        .insert(header::WWW_AUTHENTICATE, header_value);
    resp
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Derive an SPKI-PEM-encoded RSA public key from a private-key PEM
/// (accepting either PKCS#8 or PKCS#1 input). `jsonwebtoken`'s
/// `DecodingKey::from_rsa_pem` requires a public-key PEM, so we extract
/// it here so operators only have to manage a single key file.
fn derive_public_key_pem(private_pem: &[u8]) -> Result<String, OAuthError> {
    let pem_str = std::str::from_utf8(private_pem)
        .map_err(|e| OAuthError::PrivateKey(format!("not valid UTF-8: {e}")))?;
    let key = RsaPrivateKey::from_pkcs8_pem(pem_str)
        .or_else(|_| RsaPrivateKey::from_pkcs1_pem(pem_str))
        .map_err(|e| OAuthError::PrivateKey(e.to_string()))?;
    let public = key.to_public_key();
    public
        .to_pkcs1_pem(LineEnding::LF)
        .map_err(|e| OAuthError::PublicKeyEncode(e.to_string()))
}

async fn rotate_refresh_token(state: &OAuthState, client_id: &str, scope: &str) -> String {
    issue_refresh_token(state, client_id, scope).await
}

async fn take_auth_code(state: &OAuthState, code: &str) -> Option<AuthCode> {
    let mut store = state.store.write().await;
    store.auth_codes.remove(code)
}

async fn take_refresh_token(state: &OAuthState, refresh: &str) -> Option<RefreshTokenEntry> {
    let mut store = state.store.write().await;
    store.refresh_tokens.remove(refresh)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn random_string(len: usize) -> String {
    let mut out = String::with_capacity(len);
    while out.len() < len {
        out.push_str(&Uuid::new_v4().simple().to_string());
    }
    out.truncate(len);
    out
}

fn error_json(status: StatusCode, error: &str, description: &str) -> Response {
    (
        status,
        Json(json!({
            "error": error,
            "error_description": description,
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PEM: &[u8] = include_bytes!("../../tests/fixtures/oauth_test_key.pem");

    fn state() -> OAuthState {
        OAuthState::from_pem("http://localhost:8080".to_string(), TEST_PEM, 3600)
            .expect("test PEM parses")
    }

    #[test]
    fn token_ttl_is_clamped_to_24h() {
        let state = OAuthState::from_pem("http://localhost:8080".to_string(), TEST_PEM, 1_000_000)
            .expect("PEM parses");
        assert_eq!(state.token_ttl.as_secs(), MAX_TOKEN_TTL_SECS);
    }

    #[test]
    fn pkce_s256_computation_matches_rfc_test_vector() {
        // RFC 7636 Appendix B: verifier "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        // → challenge "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let expected = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        let digest = Sha256::digest(verifier.as_bytes());
        let computed = URL_SAFE_NO_PAD.encode(digest);
        assert_eq!(computed, expected);
    }

    #[test]
    fn jwt_round_trip_with_state_key() {
        let s = state();
        let token = issue_access_token(&s, "test-client", "mcp").expect("sign");
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&["mcp"]);
        validation.set_issuer(&[s.issuer_url.as_str()]);
        let decoded =
            decode::<AccessTokenClaims>(&token, &s.decoding_key, &validation).expect("verify");
        assert_eq!(decoded.claims.sub, "test-client");
        assert_eq!(decoded.claims.scope, "mcp");
    }

    #[test]
    fn html_escape_handles_special_chars() {
        assert_eq!(
            html_escape("<script>&\"'</script>"),
            "&lt;script&gt;&amp;&quot;&#39;&lt;/script&gt;",
        );
    }

    #[tokio::test]
    async fn refresh_token_round_trip_via_store() {
        let s = state();
        let rt = issue_refresh_token(&s, "rt-client", "mcp").await;
        let snapshot = {
            let g = s.store.read().await;
            g.refresh_tokens.get(&rt).cloned()
        };
        assert!(snapshot.is_some());
    }

    #[test]
    fn render_consent_page_includes_client_name_and_scope() {
        let html = render_consent_page(
            "client-abc",
            "Claude",
            "https://example.com/cb",
            "mcp",
            "state-123",
            "ch-xyz",
        );
        assert!(
            html.contains("<title>Authorize Claude</title>"),
            "title missing: {html}"
        );
        assert!(html.contains("client-abc"), "client_id missing: {html}");
        assert!(
            html.contains("https://example.com/cb"),
            "redirect missing: {html}"
        );
        assert!(html.contains("ch-xyz"), "code_challenge missing: {html}");
        assert!(html.contains("S256"), "S256 marker missing: {html}");
    }

    #[test]
    fn render_consent_page_escapes_malicious_client_name() {
        let html = render_consent_page(
            "client-abc",
            "<script>alert(1)</script>",
            "https://example.com/cb",
            "mcp",
            "s",
            "c",
        );
        assert!(
            !html.contains("<script>alert(1)</script>"),
            "raw script tag not escaped: {html}"
        );
        assert!(
            html.contains("&lt;script&gt;"),
            "expected escaped tag: {html}"
        );
    }

    #[tokio::test]
    async fn take_refresh_token_consumes_entry_so_replay_returns_none() {
        let s = state();
        let rt = issue_refresh_token(&s, "rt-client", "mcp").await;

        let first = take_refresh_token(&s, &rt).await;
        assert!(first.is_some(), "first consumption should succeed");

        let second = take_refresh_token(&s, &rt).await;
        assert!(
            second.is_none(),
            "consumed refresh token must not be reusable"
        );
    }

    #[tokio::test]
    async fn take_auth_code_returns_none_for_unknown_code() {
        let s = state();
        let result = take_auth_code(&s, "never-issued").await;
        assert!(result.is_none());
    }
}
