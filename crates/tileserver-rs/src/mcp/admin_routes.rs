//! Operator-facing OAuth admin endpoints mounted on the admin listener.
//!
//! These routes surface the MCP OAuth backend ([`OAuthBackend`]) to the
//! operator UI under `/__admin/oauth/*`. They live on the admin
//! `127.0.0.1:N` listener (same convention as `/__admin/reload`) so they
//! are **not** exposed on the public port — there is no per-route auth,
//! the admin bind itself is the security boundary.
//!
//! # Endpoints
//!
//! | Method | Path | Purpose |
//! |---|---|---|
//! | `GET`    | `/__admin/oauth/clients`             | List every registered DCR client with derived stats (active sessions, granted scopes, first/last sign-in timestamps). |
//! | `DELETE` | `/__admin/oauth/clients/{client_id}` | Revoke a client. Cascades to every refresh token issued to that client (the backend trait guarantees this atomically). |
//! | `GET`    | `/__admin/oauth/sessions`            | List every outstanding refresh token (= "device session") with the joined client name + granted/expires timestamps. |
//! | `DELETE` | `/__admin/oauth/sessions/{token}`    | Revoke a single device session. Other sessions for the same client are unaffected. |
//!
//! # Response shapes
//!
//! Plain JSON; no pagination (the operator UI shows everything on one
//! page — a single tile server is unlikely to have more than a few
//! hundred clients). All timestamps are unsigned Unix epoch seconds.

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get};
use serde::{Deserialize, Serialize};

use super::auth_store::{BackendError, OAuthBackend};

/// Refresh-token lifetime in seconds; kept in sync with
/// [`super::auth::REFRESH_TOKEN_TTL_SECS`]. Used to derive the
/// `granted_at` timestamp for sessions and clients (we don't persist
/// granted_at directly).
const REFRESH_TOKEN_TTL_SECS: u64 = 60 * 60 * 24 * 90;

/// Operator-facing view of a registered DCR client.
///
/// The fields here are deliberately enumerated (rather than
/// `#[serde(flatten)]`-ing the storage type [`super::auth::RegisteredClient`])
/// because `RegisteredClient.client_secret` MUST NOT be exposed on this
/// admin endpoint. Flattening would silently leak the secret on a future
/// add-a-field change to the storage struct — the explicit enumeration is
/// the security boundary.
#[derive(Debug, Serialize, Deserialize)]
pub struct AdminClient {
    /// Public OAuth `client_id`.
    pub client_id: String,
    /// Human-readable name supplied at registration (`null` if the
    /// client did not provide one).
    pub client_name: Option<String>,
    /// Every redirect URI registered for this client.
    pub redirect_uris: Vec<String>,
    /// Number of refresh tokens currently outstanding for this client.
    pub active_sessions: u32,
    /// Distinct OAuth scopes the client has been granted across every
    /// outstanding refresh token. Empty when the client has no
    /// outstanding sessions.
    pub scopes: Vec<String>,
    /// Approximate Unix-epoch seconds of the earliest outstanding
    /// session for this client (derived as `min(expires_at) -
    /// REFRESH_TOKEN_TTL_SECS`). `null` when no sessions exist.
    pub first_granted_at: Option<u64>,
    /// Approximate Unix-epoch seconds of the latest outstanding session
    /// (derived as `max(expires_at) - REFRESH_TOKEN_TTL_SECS`). `null`
    /// when no sessions exist.
    pub last_seen_at: Option<u64>,
}

/// Operator-facing view of a single outstanding refresh token. From the
/// UI's perspective each refresh token is a "device session".
#[derive(Debug, Serialize, Deserialize)]
pub struct AdminSession {
    /// Opaque token identifier. The DELETE endpoint accepts this value
    /// in the path segment.
    pub token_id: String,
    /// Owning client.
    pub client_id: String,
    /// Optional human-readable client name (joined from clients table).
    pub client_name: Option<String>,
    /// Granted OAuth scope at the time of issuance.
    pub scope: String,
    /// Approximate Unix-epoch seconds of issuance.
    pub granted_at: u64,
    /// Unix-epoch seconds at which the refresh token expires.
    pub expires_at: u64,
}

/// Response to a successful DELETE.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteResponse {
    /// Always `true`.
    pub ok: bool,
    /// `true` when something was actually removed; `false` when the
    /// target did not exist (still 200, idempotent semantics).
    pub deleted: bool,
    /// For client deletes: number of refresh tokens revoked as a result
    /// of the cascade. `null` for session deletes.
    pub revoked_sessions: Option<u32>,
}

/// JSON body emitted on a backend storage failure.
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: &'static str,
    error_description: String,
}

/// Mount the admin OAuth routes onto the admin listener.
///
/// `store` must be the SAME `Arc<dyn OAuthBackend>` the public OAuth
/// router uses, or the admin UI will display a stale view that does not
/// reflect actual server state.
pub fn admin_router(store: Arc<dyn OAuthBackend>) -> Router {
    Router::new()
        .route("/__admin/oauth/clients", get(list_clients))
        .route("/__admin/oauth/clients/{client_id}", delete(delete_client))
        .route("/__admin/oauth/sessions", get(list_sessions))
        .route("/__admin/oauth/sessions/{token}", delete(delete_session))
        .with_state(store)
}

fn backend_to_response(err: &BackendError) -> (StatusCode, Json<ErrorResponse>) {
    tracing::error!("MCP admin OAuth backend error: {err}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "server_error",
            error_description: format!("{err}"),
        }),
    )
}

async fn list_clients(
    State(store): State<Arc<dyn OAuthBackend>>,
) -> Result<Json<Vec<AdminClient>>, (StatusCode, Json<ErrorResponse>)> {
    let clients = store
        .list_clients()
        .await
        .map_err(|e| backend_to_response(&e))?;
    let refresh_tokens = store
        .list_refresh_tokens()
        .await
        .map_err(|e| backend_to_response(&e))?;

    let mut out = Vec::with_capacity(clients.len());
    for c in clients {
        let mut sessions_for_client = refresh_tokens
            .iter()
            .filter(|(_, e)| e.client_id == c.client_id)
            .peekable();

        let mut scopes: Vec<String> = Vec::new();
        let mut first_expiry: Option<u64> = None;
        let mut last_expiry: Option<u64> = None;
        let mut count: u32 = 0;
        for (_, entry) in sessions_for_client.by_ref() {
            count = count.saturating_add(1);
            for tok in entry.scope.split_whitespace() {
                if !scopes.iter().any(|s| s == tok) {
                    scopes.push(tok.to_string());
                }
            }
            first_expiry = match first_expiry {
                None => Some(entry.expires_at),
                Some(prev) => Some(prev.min(entry.expires_at)),
            };
            last_expiry = match last_expiry {
                None => Some(entry.expires_at),
                Some(prev) => Some(prev.max(entry.expires_at)),
            };
        }

        out.push(AdminClient {
            client_id: c.client_id,
            client_name: c.client_name,
            redirect_uris: c.redirect_uris,
            active_sessions: count,
            scopes,
            first_granted_at: first_expiry.map(|e| e.saturating_sub(REFRESH_TOKEN_TTL_SECS)),
            last_seen_at: last_expiry.map(|e| e.saturating_sub(REFRESH_TOKEN_TTL_SECS)),
        });
    }
    Ok(Json(out))
}

async fn delete_client(
    State(store): State<Arc<dyn OAuthBackend>>,
    Path(client_id): Path<String>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let existing = store
        .get_client(&client_id)
        .await
        .map_err(|e| backend_to_response(&e))?;
    if existing.is_none() {
        return Ok(Json(DeleteResponse {
            ok: true,
            deleted: false,
            revoked_sessions: Some(0),
        }));
    }
    let prior_sessions = store
        .list_refresh_tokens()
        .await
        .map_err(|e| backend_to_response(&e))?
        .into_iter()
        .filter(|(_, e)| e.client_id == client_id)
        .count();
    store
        .delete_client(&client_id)
        .await
        .map_err(|e| backend_to_response(&e))?;
    Ok(Json(DeleteResponse {
        ok: true,
        deleted: true,
        revoked_sessions: Some(u32::try_from(prior_sessions).unwrap_or(u32::MAX)),
    }))
}

async fn list_sessions(
    State(store): State<Arc<dyn OAuthBackend>>,
) -> Result<Json<Vec<AdminSession>>, (StatusCode, Json<ErrorResponse>)> {
    let tokens = store
        .list_refresh_tokens()
        .await
        .map_err(|e| backend_to_response(&e))?;
    let clients = store
        .list_clients()
        .await
        .map_err(|e| backend_to_response(&e))?;

    let mut out = Vec::with_capacity(tokens.len());
    for (token_id, entry) in tokens {
        let client_name = clients
            .iter()
            .find(|c| c.client_id == entry.client_id)
            .and_then(|c| c.client_name.clone());
        out.push(AdminSession {
            token_id,
            client_name,
            client_id: entry.client_id,
            scope: entry.scope,
            granted_at: entry.expires_at.saturating_sub(REFRESH_TOKEN_TTL_SECS),
            expires_at: entry.expires_at,
        });
    }
    Ok(Json(out))
}

async fn delete_session(
    State(store): State<Arc<dyn OAuthBackend>>,
    Path(token): Path<String>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let existed = store
        .list_refresh_tokens()
        .await
        .map_err(|e| backend_to_response(&e))?
        .into_iter()
        .any(|(t, _)| t == token);
    store
        .delete_refresh_token(&token)
        .await
        .map_err(|e| backend_to_response(&e))?;
    Ok(Json(DeleteResponse {
        ok: true,
        deleted: existed,
        revoked_sessions: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::auth::{RefreshTokenEntry, RegisteredClient};
    use crate::mcp::auth_store::InMemoryStore;
    use axum_test::TestServer;

    fn sample_client(id: &str, name: Option<&str>) -> RegisteredClient {
        RegisteredClient {
            client_id: id.to_string(),
            client_secret: None,
            redirect_uris: vec!["http://localhost/cb".to_string()],
            client_name: name.map(str::to_string),
        }
    }

    fn sample_refresh(client_id: &str, scope: &str, expires_at: u64) -> RefreshTokenEntry {
        RefreshTokenEntry {
            client_id: client_id.to_string(),
            scope: scope.to_string(),
            expires_at,
        }
    }

    async fn seed_store() -> Arc<dyn OAuthBackend> {
        let store: Arc<dyn OAuthBackend> = Arc::new(InMemoryStore::new());
        store
            .insert_client("c-1".to_string(), sample_client("c-1", Some("Claude.ai")))
            .await
            .unwrap();
        store
            .insert_client("c-2".to_string(), sample_client("c-2", None))
            .await
            .unwrap();
        store
            .insert_refresh_token(
                "rt-c1-a".to_string(),
                sample_refresh("c-1", "identify read render", 9_999_999_900),
            )
            .await
            .unwrap();
        store
            .insert_refresh_token(
                "rt-c1-b".to_string(),
                sample_refresh("c-1", "identify query", 9_999_999_950),
            )
            .await
            .unwrap();
        store
            .insert_refresh_token(
                "rt-c2-a".to_string(),
                sample_refresh("c-2", "read query", 9_999_999_800),
            )
            .await
            .unwrap();
        store
    }

    fn server(store: Arc<dyn OAuthBackend>) -> TestServer {
        TestServer::new(admin_router(store))
    }

    #[tokio::test]
    async fn list_clients_aggregates_sessions_and_distinct_scopes() {
        let store = seed_store().await;
        let parsed: Vec<AdminClient> = server(store).get("/__admin/oauth/clients").await.json();

        let c1 = parsed.iter().find(|c| c.client_id == "c-1").unwrap();
        assert_eq!(c1.active_sessions, 2);
        let mut scopes = c1.scopes.clone();
        scopes.sort();
        assert_eq!(
            scopes,
            vec![
                "identify".to_string(),
                "query".to_string(),
                "read".to_string(),
                "render".to_string(),
            ]
        );
        assert_eq!(c1.client_name.as_deref(), Some("Claude.ai"));
        assert!(c1.first_granted_at.is_some());
        assert!(c1.last_seen_at.is_some());

        let c2 = parsed.iter().find(|c| c.client_id == "c-2").unwrap();
        assert_eq!(c2.active_sessions, 1);
        assert!(c2.client_name.is_none());
    }

    #[tokio::test]
    async fn delete_client_cascades_and_reports_revoked_sessions_count() {
        let store = seed_store().await;
        let parsed: DeleteResponse = server(Arc::clone(&store))
            .delete("/__admin/oauth/clients/c-1")
            .await
            .json();
        assert!(parsed.ok);
        assert!(parsed.deleted);
        assert_eq!(parsed.revoked_sessions, Some(2));

        assert!(store.get_client("c-1").await.unwrap().is_none());
        let remaining: Vec<_> = store
            .list_refresh_tokens()
            .await
            .unwrap()
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        assert_eq!(remaining, vec!["rt-c2-a".to_string()]);
    }

    #[tokio::test]
    async fn delete_client_returns_deleted_false_for_unknown_id() {
        let store = seed_store().await;
        let parsed: DeleteResponse = server(store)
            .delete("/__admin/oauth/clients/never-existed")
            .await
            .json();
        assert!(parsed.ok);
        assert!(!parsed.deleted);
        assert_eq!(parsed.revoked_sessions, Some(0));
    }

    #[tokio::test]
    async fn list_sessions_joins_client_name() {
        let store = seed_store().await;
        let parsed: Vec<AdminSession> = server(store).get("/__admin/oauth/sessions").await.json();

        assert_eq!(parsed.len(), 3);
        let one_for_c1 = parsed.iter().find(|s| s.client_id == "c-1").unwrap();
        assert_eq!(one_for_c1.client_name.as_deref(), Some("Claude.ai"));
        let one_for_c2 = parsed.iter().find(|s| s.client_id == "c-2").unwrap();
        assert!(one_for_c2.client_name.is_none());
    }

    #[tokio::test]
    async fn empty_store_returns_empty_lists() {
        // The admin UI mounts these routes even when OAuth is disabled
        // (static bearer / no auth). An empty in-memory store must answer
        // with empty lists, never 404 — the connected-apps and devices
        // pages render their empty states instead of erroring.
        let store: Arc<dyn OAuthBackend> = Arc::new(InMemoryStore::new());
        let parsed: Vec<AdminClient> = server(Arc::clone(&store))
            .get("/__admin/oauth/clients")
            .await
            .json();
        assert!(parsed.is_empty());

        let sessions: Vec<AdminSession> = server(Arc::clone(&store))
            .get("/__admin/oauth/sessions")
            .await
            .json();
        assert!(sessions.is_empty());

        // Idempotent deletes on an empty store still answer 200.
        let del: DeleteResponse = server(Arc::clone(&store))
            .delete("/__admin/oauth/clients/never-existed")
            .await
            .json();
        assert!(del.ok);
        assert!(!del.deleted);
    }

    #[tokio::test]
    async fn delete_session_reports_deleted_true_when_exists() {
        let store = seed_store().await;
        let parsed: DeleteResponse = server(Arc::clone(&store))
            .delete("/__admin/oauth/sessions/rt-c2-a")
            .await
            .json();
        assert!(parsed.ok);
        assert!(parsed.deleted);
        assert_eq!(parsed.revoked_sessions, None);

        let remaining: Vec<_> = store
            .list_refresh_tokens()
            .await
            .unwrap()
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        assert!(!remaining.contains(&"rt-c2-a".to_string()));
        assert_eq!(remaining.len(), 2);
    }

    #[tokio::test]
    async fn delete_session_reports_deleted_false_for_unknown_token() {
        let store = seed_store().await;
        let parsed: DeleteResponse = server(store)
            .delete("/__admin/oauth/sessions/rt-never-issued")
            .await
            .json();
        assert!(parsed.ok);
        assert!(!parsed.deleted);
    }
}
