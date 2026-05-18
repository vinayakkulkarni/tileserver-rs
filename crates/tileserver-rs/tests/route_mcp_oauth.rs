//! Integration tests for the MCP OAuth 2.0 authorization server.
//!
//! Exercises:
//! - Discovery: `/.well-known/oauth-authorization-server` + protected-resource.
//! - DCR (RFC 7591) registration with happy + error paths.
//! - Authorization-code flow with PKCE S256 (and rejection of `plain`).
//! - Token exchange + refresh-token grant.
//! - Bearer-token middleware acceptance / rejection.
//! - Config validation: static bearer + OAuth are mutually exclusive.

#![cfg(feature = "mcp")]

use std::sync::OnceLock;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::routing::post;
use axum_test::TestServer;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde_json::{Value, json};
use tileserver_rs::mcp::auth::{
    AccessTokenClaims, OAuthState, oauth_router, validate_oauth_bearer,
};

const TEST_ISSUER: &str = "http://oauth.test";
const TEST_REDIRECT: &str = "https://claude.ai/api/mcp/auth_callback";
const TEST_TTL_SECS: u64 = 3600;

/// 2048-bit RSA PKCS#8 PEM used by every OAuth test. Generated once with
/// `openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048` and baked
/// in so tests don't shell out at runtime. Test-only — DO NOT reuse this
/// key for anything other than localhost test runs.
const TEST_RSA_PEM: &str = include_str!("fixtures/oauth_test_key.pem");

fn test_state() -> OAuthState {
    static CACHED: OnceLock<OAuthState> = OnceLock::new();
    CACHED
        .get_or_init(|| {
            OAuthState::from_pem(
                TEST_ISSUER.to_string(),
                TEST_RSA_PEM.as_bytes(),
                TEST_TTL_SECS,
            )
            .expect("test RSA PEM is well-formed")
        })
        .clone()
}

/// Builds an axum app exposing OAuth routes + a stub `/mcp` POST that
/// short-circuits with 200 if `validate_oauth_bearer` lets the request
/// through.
fn oauth_app(state: OAuthState) -> Router {
    let protected = Router::new().route("/mcp", post(|| async { "ok" })).layer(
        axum::middleware::from_fn_with_state(state.clone(), validate_oauth_bearer),
    );

    oauth_router(state).merge(protected)
}

fn test_server(state: OAuthState) -> TestServer {
    TestServer::new(oauth_app(state))
}

/// PKCE helper — compute `BASE64URL(SHA256(verifier))` with no padding.
fn pkce_challenge(verifier: &str) -> String {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

/// Register a fresh client and return its `client_id`.
async fn register_client(server: &TestServer) -> String {
    let resp = server
        .post("/register")
        .json(&json!({
            "client_name": "claude.ai",
            "redirect_uris": [TEST_REDIRECT],
            "grant_types": ["authorization_code", "refresh_token"]
        }))
        .await;
    resp.assert_status(StatusCode::CREATED);
    let body: Value = resp.json();
    body["client_id"]
        .as_str()
        .expect("client_id in registration response")
        .to_string()
}

/// Drive `/authorize` + `/approve` and return the redirect Location with the
/// freshly-issued auth code.
async fn approve_and_get_code(
    server: &TestServer,
    client_id: &str,
    code_challenge: &str,
    state_param: &str,
) -> String {
    let approve_resp = server
        .post("/approve")
        .form(&[
            ("client_id", client_id),
            ("redirect_uri", TEST_REDIRECT),
            ("scope", "mcp"),
            ("state", state_param),
            ("code_challenge", code_challenge),
            ("code_challenge_method", "S256"),
            ("approved", "true"),
        ])
        .await;

    let status = approve_resp.status_code();
    assert!(
        status.is_redirection(),
        "expected redirect after approve, got {status}"
    );
    let location = approve_resp
        .headers()
        .get(header::LOCATION)
        .expect("Location header on redirect")
        .to_str()
        .expect("ascii Location")
        .to_string();
    let url = url::Url::parse(&location).expect("valid Location URL");
    url.query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned())
        .expect("code= in redirect query")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn oauth_discovery_well_known_returns_required_fields() {
    let server = test_server(test_state());

    let resp = server.get("/.well-known/oauth-authorization-server").await;
    resp.assert_status_ok();
    let body: Value = resp.json();

    assert_eq!(body["issuer"], TEST_ISSUER);
    assert_eq!(
        body["authorization_endpoint"],
        format!("{TEST_ISSUER}/authorize")
    );
    assert_eq!(body["token_endpoint"], format!("{TEST_ISSUER}/token"));
    assert_eq!(
        body["registration_endpoint"],
        format!("{TEST_ISSUER}/register")
    );
    assert_eq!(body["code_challenge_methods_supported"], json!(["S256"]));
    assert!(
        body["grant_types_supported"]
            .as_array()
            .expect("grant_types_supported array")
            .iter()
            .any(|v| v == "authorization_code"),
    );
    assert!(
        body["grant_types_supported"]
            .as_array()
            .expect("grant_types_supported array")
            .iter()
            .any(|v| v == "refresh_token"),
    );
    assert_eq!(body["response_types_supported"], json!(["code"]));
    assert!(
        body["token_endpoint_auth_methods_supported"]
            .as_array()
            .expect("token_endpoint_auth_methods_supported array")
            .iter()
            .any(|v| v == "none"),
    );
    assert!(
        body["token_endpoint_auth_methods_supported"]
            .as_array()
            .expect("token_endpoint_auth_methods_supported array")
            .iter()
            .any(|v| v == "client_secret_post"),
    );
}

#[tokio::test]
async fn oauth_protected_resource_metadata_returns_resource() {
    let server = test_server(test_state());

    let resp = server.get("/.well-known/oauth-protected-resource").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["resource"], TEST_ISSUER);
    let servers = body["authorization_servers"]
        .as_array()
        .expect("authorization_servers array");
    assert!(
        servers.iter().any(|v| v == TEST_ISSUER),
        "authorization_servers includes issuer URL: {servers:?}",
    );
}

#[tokio::test]
async fn oauth_register_minimal_request_returns_client_credentials() {
    let server = test_server(test_state());

    let resp = server
        .post("/register")
        .json(&json!({
            "client_name": "claude.ai",
            "redirect_uris": [TEST_REDIRECT],
            "grant_types": ["authorization_code", "refresh_token"]
        }))
        .await;
    resp.assert_status(StatusCode::CREATED);

    let body: Value = resp.json();
    assert!(
        body["client_id"].is_string(),
        "client_id is a string: {body}",
    );
    assert!(
        body["client_id_issued_at"].is_number(),
        "client_id_issued_at is numeric: {body}",
    );
}

#[tokio::test]
async fn oauth_register_missing_redirect_uris_returns_400() {
    let server = test_server(test_state());

    let resp = server
        .post("/register")
        .json(&json!({ "client_name": "claude.ai" }))
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    let body: Value = resp.json();
    assert_eq!(body["error"], "invalid_redirect_uri");
}

#[tokio::test]
async fn oauth_authorize_redirects_with_code_when_approved() {
    let server = test_server(test_state());
    let client_id = register_client(&server).await;
    let verifier = "verifier-must-be-43-or-more-chars-aaaaaaaaaaaaaaaaaaa";
    let challenge = pkce_challenge(verifier);

    let code = approve_and_get_code(&server, &client_id, &challenge, "xyz").await;
    assert!(!code.is_empty(), "auth code is non-empty");
}

#[tokio::test]
async fn oauth_authorize_rejects_plain_pkce() {
    let server = test_server(test_state());
    let client_id = register_client(&server).await;

    let resp = server
        .get("/authorize")
        .add_query_param("response_type", "code")
        .add_query_param("client_id", &client_id)
        .add_query_param("redirect_uri", TEST_REDIRECT)
        .add_query_param("scope", "mcp")
        .add_query_param("state", "abc")
        .add_query_param("code_challenge", "some-challenge")
        .add_query_param("code_challenge_method", "plain")
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    let body: Value = resp.json();
    assert_eq!(body["error"], "invalid_request");
}

#[tokio::test]
async fn oauth_token_exchanges_code_for_access_token() {
    let server = test_server(test_state());
    let client_id = register_client(&server).await;
    let verifier = "verifier-must-be-43-or-more-chars-aaaaaaaaaaaaaaaaaaa";
    let challenge = pkce_challenge(verifier);
    let code = approve_and_get_code(&server, &client_id, &challenge, "s").await;

    let form = serde_urlencoded::to_string([
        ("grant_type", "authorization_code"),
        ("code", &code),
        ("redirect_uri", TEST_REDIRECT),
        ("client_id", &client_id),
        ("code_verifier", verifier),
    ])
    .expect("encode form");

    let resp = server
        .post("/token")
        .content_type("application/x-www-form-urlencoded")
        .bytes(form.into_bytes().into())
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();

    assert!(body["access_token"].is_string());
    assert_eq!(body["token_type"], "Bearer");
    assert!(body["expires_in"].as_u64().unwrap_or(0) > 0);
    assert!(body["refresh_token"].is_string(), "{body}");
}

#[tokio::test]
async fn oauth_token_rejects_wrong_pkce_verifier() {
    let server = test_server(test_state());
    let client_id = register_client(&server).await;
    let challenge = pkce_challenge("verifier-must-be-43-or-more-chars-aaaaaaaaaaaaaaaaaaa");
    let code = approve_and_get_code(&server, &client_id, &challenge, "s").await;

    let form = serde_urlencoded::to_string([
        ("grant_type", "authorization_code"),
        ("code", &code),
        ("redirect_uri", TEST_REDIRECT),
        ("client_id", &client_id),
        (
            "code_verifier",
            "this-is-not-the-verifier-we-bound-the-code-to",
        ),
    ])
    .expect("encode form");

    let resp = server
        .post("/token")
        .content_type("application/x-www-form-urlencoded")
        .bytes(form.into_bytes().into())
        .await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    let body: Value = resp.json();
    assert_eq!(body["error"], "invalid_grant");
}

#[tokio::test]
async fn oauth_token_refresh_grant_returns_new_access_token() {
    let server = test_server(test_state());
    let client_id = register_client(&server).await;
    let verifier = "verifier-must-be-43-or-more-chars-aaaaaaaaaaaaaaaaaaa";
    let challenge = pkce_challenge(verifier);
    let code = approve_and_get_code(&server, &client_id, &challenge, "s").await;

    let initial = {
        let form = serde_urlencoded::to_string([
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", TEST_REDIRECT),
            ("client_id", &client_id),
            ("code_verifier", verifier),
        ])
        .expect("encode form");
        let resp = server
            .post("/token")
            .content_type("application/x-www-form-urlencoded")
            .bytes(form.into_bytes().into())
            .await;
        resp.assert_status_ok();
        resp.json::<Value>()
    };

    let refresh_token = initial["refresh_token"]
        .as_str()
        .expect("refresh_token present");

    let refresh_form = serde_urlencoded::to_string([
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", &client_id),
    ])
    .expect("encode form");
    let refreshed = server
        .post("/token")
        .content_type("application/x-www-form-urlencoded")
        .bytes(refresh_form.into_bytes().into())
        .await;
    refreshed.assert_status_ok();
    let body: Value = refreshed.json();
    assert!(body["access_token"].is_string());
    assert!(
        body["access_token"] != initial["access_token"],
        "refresh issues a different access token",
    );
}

#[tokio::test]
async fn mcp_call_with_valid_oauth_bearer_succeeds() {
    let server = test_server(test_state());
    let client_id = register_client(&server).await;
    let verifier = "verifier-must-be-43-or-more-chars-aaaaaaaaaaaaaaaaaaa";
    let challenge = pkce_challenge(verifier);
    let code = approve_and_get_code(&server, &client_id, &challenge, "s").await;

    let form = serde_urlencoded::to_string([
        ("grant_type", "authorization_code"),
        ("code", &code),
        ("redirect_uri", TEST_REDIRECT),
        ("client_id", &client_id),
        ("code_verifier", verifier),
    ])
    .expect("encode form");
    let resp = server
        .post("/token")
        .content_type("application/x-www-form-urlencoded")
        .bytes(form.into_bytes().into())
        .await;
    resp.assert_status_ok();
    let token: Value = resp.json();
    let access = token["access_token"].as_str().expect("access_token");

    let mcp_resp = server
        .post("/mcp")
        .add_header("authorization", format!("Bearer {access}"))
        .await;
    mcp_resp.assert_status_ok();
}

#[tokio::test]
async fn mcp_call_with_invalid_oauth_bearer_returns_401() {
    let server = test_server(test_state());
    let resp = server
        .post("/mcp")
        .add_header("authorization", "Bearer not-a-jwt")
        .await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
    let www_auth = resp
        .headers()
        .get(header::WWW_AUTHENTICATE)
        .expect("WWW-Authenticate header on 401")
        .to_str()
        .expect("ascii WWW-Authenticate")
        .to_string();
    assert!(
        www_auth.contains("Bearer"),
        "WWW-Authenticate is a Bearer challenge: {www_auth}",
    );
    assert!(
        www_auth.contains("invalid_token"),
        "challenge includes invalid_token error: {www_auth}",
    );
}

#[tokio::test]
async fn mcp_call_with_expired_oauth_bearer_returns_401() {
    let state = test_state();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_secs();
    let claims = AccessTokenClaims {
        sub: "expired-client".to_string(),
        iss: TEST_ISSUER.to_string(),
        aud: "mcp".to_string(),
        iat: now - 7200,
        exp: now - 3600,
        scope: "mcp".to_string(),
        jti: None,
    };
    let header = Header::new(jsonwebtoken::Algorithm::RS256);
    let signing_key =
        EncodingKey::from_rsa_pem(TEST_RSA_PEM.as_bytes()).expect("EncodingKey from PEM");
    let expired_jwt = encode(&header, &claims, &signing_key).expect("encode JWT");

    let app = oauth_app(state);
    let req = Request::builder()
        .method(Method::POST)
        .uri("/mcp")
        .header(header::AUTHORIZATION, format!("Bearer {expired_jwt}"))
        .body(Body::empty())
        .expect("build request");

    let resp = tower::ServiceExt::oneshot(app, req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn static_bearer_and_oauth_mutually_exclusive_config_rejected() {
    let toml = r#"
[mcp]
enabled = true
auth_token = "shhh"

[mcp.oauth]
enabled = true
issuer_url = "http://oauth.test"
signing_key_path = "/dev/null"
"#;
    let config: tileserver_rs::config::Config = toml::from_str(toml).expect("parse toml");
    let err = config.validate().expect_err("validate must reject both");
    let msg = format!("{err}");
    assert!(
        msg.contains("cannot enable both") || msg.contains("auth_token"),
        "error mentions mutual exclusion: {msg}",
    );
}
