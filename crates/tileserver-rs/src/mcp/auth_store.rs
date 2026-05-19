//! Pluggable backing store for the MCP authorization server.
//!
//! Defines [`OAuthBackend`] — the async trait every storage backend must
//! satisfy — and the default [`InMemoryStore`] implementation. A
//! disk-backed SQLite implementation lives in [`super::auth_store_sqlite`]
//! and is gated behind the `mcp-persistence` Cargo feature.
//!
//! # Why a trait
//!
//! `src/mcp/auth.rs` previously owned a concrete `OAuthStore` struct made
//! of three `HashMap`s wrapped in `Arc<RwLock<…>>`. That works fine in
//! memory but loses every issued token on restart, which is unacceptable
//! once operators start managing connected apps from the admin UI ("I
//! revoked it yesterday and it's back"). Abstracting the store behind a
//! small async trait keeps the in-memory path as the zero-config default
//! and lets `mcp-persistence` swap in a SQLite-backed implementation
//! without touching any of the OAuth flow code in `auth.rs`.
//!
//! # Surface
//!
//! Six core operations cover the OAuth flow (`auth.rs`):
//!
//! - [`OAuthBackend::insert_client`] / [`OAuthBackend::get_client`] — DCR
//!   registration + lookup during `/authorize` and `/approve`.
//! - [`OAuthBackend::insert_auth_code`] / [`OAuthBackend::take_auth_code`]
//!   — consent-page issued codes consumed exactly once at `/token`.
//! - [`OAuthBackend::insert_refresh_token`] /
//!   [`OAuthBackend::take_refresh_token`] — refresh-token issue + rotation.
//!
//! Four additional operations support the admin UI (`/__admin/oauth/*`):
//!
//! - [`OAuthBackend::list_clients`] — render the connected-apps page.
//! - [`OAuthBackend::delete_client`] — revoke. Cascades to refresh tokens.
//! - [`OAuthBackend::list_refresh_tokens`] — render the device-sessions
//!   page.
//! - [`OAuthBackend::delete_refresh_token`] — revoke a single session.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::auth::{AuthCode, RefreshTokenEntry, RegisteredClient};

/// Errors that any backend can surface.
///
/// Distinct from [`super::auth::OAuthError`] because storage failures (a
/// SQLite write returning `SQLITE_BUSY`, for example) are operationally
/// different from PKCE / JWT verification failures and the OAuth flow
/// should map them to a generic `server_error` response rather than leak
/// implementation details.
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    /// Underlying storage rejected the operation (disk full, locked, etc.).
    #[error("OAuth backend storage error: {0}")]
    Storage(String),
}

/// Result alias for backend operations.
pub type BackendResult<T> = Result<T, BackendError>;

/// The contract every OAuth backing store must satisfy.
///
/// All operations are `async` because disk-backed implementations spawn
/// blocking SQLite work onto Tokio's blocking pool. The in-memory
/// implementation [`InMemoryStore`] satisfies the same signature with
/// trivial `async` wrappers — the runtime cost is a single `await` point
/// per call, well under a microsecond.
#[async_trait]
pub trait OAuthBackend: Send + Sync + std::fmt::Debug + 'static {
    /// Persist a freshly registered DCR client.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Storage`] if the underlying store cannot
    /// commit the write (e.g. SQLite `database is locked`).
    async fn insert_client(&self, client_id: String, client: RegisteredClient)
    -> BackendResult<()>;

    /// Look up a registered client by id.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Storage`] if the read fails. A *missing*
    /// client is `Ok(None)`, not an error.
    async fn get_client(&self, client_id: &str) -> BackendResult<Option<RegisteredClient>>;

    /// Return every registered client. Used only by the admin
    /// connected-apps page; not on any hot path.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Storage`] if the read fails.
    async fn list_clients(&self) -> BackendResult<Vec<RegisteredClient>>;

    /// Revoke a client. Implementations MUST cascade-delete the client's
    /// outstanding refresh tokens in the same transaction (or equivalent
    /// atomic step) so the admin operator never observes a half-revoked
    /// client.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Storage`] if the write fails.
    async fn delete_client(&self, client_id: &str) -> BackendResult<()>;

    /// Persist a freshly-issued authorization code awaiting exchange.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Storage`] if the write fails.
    async fn insert_auth_code(&self, code: String, entry: AuthCode) -> BackendResult<()>;

    /// Consume an authorization code (single-use). Removes the code from
    /// the store and returns its contents, or `Ok(None)` if it was already
    /// consumed or never existed.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Storage`] if the underlying delete fails.
    async fn take_auth_code(&self, code: &str) -> BackendResult<Option<AuthCode>>;

    /// Persist a refresh token issued during code exchange or rotation.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Storage`] if the write fails.
    async fn insert_refresh_token(
        &self,
        token: String,
        entry: RefreshTokenEntry,
    ) -> BackendResult<()>;

    /// Consume a refresh token (single-use after OAuth 2.1 rotation).
    /// Removes the token from the store and returns its contents.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Storage`] if the underlying delete fails.
    async fn take_refresh_token(&self, token: &str) -> BackendResult<Option<RefreshTokenEntry>>;

    /// List every outstanding refresh token. The first tuple element is
    /// the token identifier (the value `take_refresh_token` would
    /// consume). Used only by the admin device-sessions page.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Storage`] if the read fails.
    async fn list_refresh_tokens(&self) -> BackendResult<Vec<(String, RefreshTokenEntry)>>;

    /// Revoke a single refresh token (admin-initiated device sign-out).
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Storage`] if the delete fails.
    async fn delete_refresh_token(&self, token: &str) -> BackendResult<()>;
}

/// Zero-config in-memory backend. Three `HashMap`s behind an
/// `Arc<RwLock<…>>`. Every issued token is invalidated on restart, which
/// is the documented historical behaviour of the MCP OAuth server.
#[derive(Debug, Default)]
pub struct InMemoryStore {
    inner: Arc<RwLock<InMemoryInner>>,
}

#[derive(Debug, Default)]
struct InMemoryInner {
    clients: HashMap<String, RegisteredClient>,
    auth_codes: HashMap<String, AuthCode>,
    refresh_tokens: HashMap<String, RefreshTokenEntry>,
}

impl InMemoryStore {
    /// Construct an empty in-memory store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl OAuthBackend for InMemoryStore {
    async fn insert_client(
        &self,
        client_id: String,
        client: RegisteredClient,
    ) -> BackendResult<()> {
        self.inner.write().await.clients.insert(client_id, client);
        Ok(())
    }

    async fn get_client(&self, client_id: &str) -> BackendResult<Option<RegisteredClient>> {
        Ok(self.inner.read().await.clients.get(client_id).cloned())
    }

    async fn list_clients(&self) -> BackendResult<Vec<RegisteredClient>> {
        Ok(self.inner.read().await.clients.values().cloned().collect())
    }

    async fn delete_client(&self, client_id: &str) -> BackendResult<()> {
        let mut guard = self.inner.write().await;
        guard.clients.remove(client_id);
        guard.refresh_tokens.retain(|_, e| e.client_id != client_id);
        Ok(())
    }

    async fn insert_auth_code(&self, code: String, entry: AuthCode) -> BackendResult<()> {
        self.inner.write().await.auth_codes.insert(code, entry);
        Ok(())
    }

    async fn take_auth_code(&self, code: &str) -> BackendResult<Option<AuthCode>> {
        Ok(self.inner.write().await.auth_codes.remove(code))
    }

    async fn insert_refresh_token(
        &self,
        token: String,
        entry: RefreshTokenEntry,
    ) -> BackendResult<()> {
        self.inner.write().await.refresh_tokens.insert(token, entry);
        Ok(())
    }

    async fn take_refresh_token(&self, token: &str) -> BackendResult<Option<RefreshTokenEntry>> {
        Ok(self.inner.write().await.refresh_tokens.remove(token))
    }

    async fn list_refresh_tokens(&self) -> BackendResult<Vec<(String, RefreshTokenEntry)>> {
        Ok(self
            .inner
            .read()
            .await
            .refresh_tokens
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }

    async fn delete_refresh_token(&self, token: &str) -> BackendResult<()> {
        self.inner.write().await.refresh_tokens.remove(token);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_client(id: &str) -> RegisteredClient {
        RegisteredClient {
            client_id: id.to_string(),
            client_secret: None,
            redirect_uris: vec!["http://localhost/cb".to_string()],
            client_name: Some("test".to_string()),
        }
    }

    fn sample_code(client_id: &str) -> AuthCode {
        AuthCode {
            client_id: client_id.to_string(),
            redirect_uri: "http://localhost/cb".to_string(),
            code_challenge: "challenge".to_string(),
            scope: "mcp".to_string(),
            expires_at: 9_999_999_999,
        }
    }

    fn sample_refresh(client_id: &str) -> RefreshTokenEntry {
        RefreshTokenEntry {
            client_id: client_id.to_string(),
            scope: "mcp".to_string(),
            expires_at: 9_999_999_999,
        }
    }

    #[tokio::test]
    async fn insert_and_get_client_roundtrips() {
        let store = InMemoryStore::new();
        store
            .insert_client("c-1".to_string(), sample_client("c-1"))
            .await
            .unwrap();
        let got = store.get_client("c-1").await.unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().client_id, "c-1");
    }

    #[tokio::test]
    async fn get_client_returns_none_when_missing() {
        let store = InMemoryStore::new();
        let got = store.get_client("nope").await.unwrap();
        assert!(got.is_none());
    }

    #[tokio::test]
    async fn list_clients_returns_every_registered_client() {
        let store = InMemoryStore::new();
        for id in ["a", "b", "c"] {
            store
                .insert_client(id.to_string(), sample_client(id))
                .await
                .unwrap();
        }
        let mut ids: Vec<_> = store
            .list_clients()
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.client_id)
            .collect();
        ids.sort();
        assert_eq!(ids, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[tokio::test]
    async fn delete_client_cascades_refresh_tokens() {
        let store = InMemoryStore::new();
        store
            .insert_client("c-1".to_string(), sample_client("c-1"))
            .await
            .unwrap();
        store
            .insert_client("c-2".to_string(), sample_client("c-2"))
            .await
            .unwrap();
        store
            .insert_refresh_token("rt-c1-a".to_string(), sample_refresh("c-1"))
            .await
            .unwrap();
        store
            .insert_refresh_token("rt-c1-b".to_string(), sample_refresh("c-1"))
            .await
            .unwrap();
        store
            .insert_refresh_token("rt-c2".to_string(), sample_refresh("c-2"))
            .await
            .unwrap();

        store.delete_client("c-1").await.unwrap();

        assert!(store.get_client("c-1").await.unwrap().is_none());
        assert!(store.get_client("c-2").await.unwrap().is_some());
        let remaining: Vec<_> = store
            .list_refresh_tokens()
            .await
            .unwrap()
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        assert_eq!(remaining, vec!["rt-c2".to_string()]);
    }

    #[tokio::test]
    async fn take_auth_code_consumes_exactly_once() {
        let store = InMemoryStore::new();
        store
            .insert_auth_code("code-1".to_string(), sample_code("c-1"))
            .await
            .unwrap();
        let first = store.take_auth_code("code-1").await.unwrap();
        assert!(first.is_some());
        let second = store.take_auth_code("code-1").await.unwrap();
        assert!(second.is_none());
    }

    #[tokio::test]
    async fn take_refresh_token_consumes_exactly_once() {
        let store = InMemoryStore::new();
        store
            .insert_refresh_token("rt-1".to_string(), sample_refresh("c-1"))
            .await
            .unwrap();
        let first = store.take_refresh_token("rt-1").await.unwrap();
        assert!(first.is_some());
        let second = store.take_refresh_token("rt-1").await.unwrap();
        assert!(second.is_none());
    }

    #[tokio::test]
    async fn delete_refresh_token_is_idempotent() {
        let store = InMemoryStore::new();
        store
            .insert_refresh_token("rt-1".to_string(), sample_refresh("c-1"))
            .await
            .unwrap();
        store.delete_refresh_token("rt-1").await.unwrap();
        store.delete_refresh_token("rt-1").await.unwrap();
        assert!(store.list_refresh_tokens().await.unwrap().is_empty());
    }
}
