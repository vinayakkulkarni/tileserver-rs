//! Integration tests for the MCP admin OAuth routes.
//!
//! These tests exercise the full HTTP boundary of:
//! - `GET    /__admin/oauth/clients`
//! - `DELETE /__admin/oauth/clients/{client_id}`
//! - `GET    /__admin/oauth/sessions`
//! - `DELETE /__admin/oauth/sessions/{token}`
//!
//! Two backends are exercised through the same admin router:
//! 1. `InMemoryStore`             — the default `mcp` feature backend.
//! 2. `SqliteStore` (`:memory:`)  — only compiled in when the
//!    `mcp-persistence` Cargo feature is on.
//!
//! The unit tests in `src/mcp/admin_routes.rs` already cover handler
//! logic (aggregation, idempotency, cascade-delete). This file's job
//! is the wiring: confirming the routes are reachable through
//! `axum-test::TestServer` exactly the way `main.rs` mounts them on
//! the admin listener.

#![cfg(feature = "mcp")]

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum_test::TestServer;
use tileserver_rs::mcp::admin_routes::{AdminClient, AdminSession, DeleteResponse, admin_router};
use tileserver_rs::mcp::auth::{AuthCode, RefreshTokenEntry, RegisteredClient};
use tileserver_rs::mcp::auth_store::{InMemoryStore, OAuthBackend};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be past 1970")
        .as_secs()
}

async fn seed_two_clients_one_session(store: &Arc<dyn OAuthBackend>) {
    let now = now_secs();
    store
        .insert_client(
            "client-a".into(),
            RegisteredClient {
                client_id: "client-a".into(),
                client_secret: None,
                redirect_uris: vec!["https://a.example/cb".into()],
                client_name: Some("Client A".into()),
            },
        )
        .await
        .expect("insert client-a");
    store
        .insert_client(
            "client-b".into(),
            RegisteredClient {
                client_id: "client-b".into(),
                client_secret: None,
                redirect_uris: vec!["https://b.example/cb".into()],
                client_name: Some("Client B".into()),
            },
        )
        .await
        .expect("insert client-b");
    store
        .insert_refresh_token(
            "rt-a-1".into(),
            RefreshTokenEntry {
                client_id: "client-a".into(),
                scope: "mcp".into(),
                expires_at: now + 3600,
            },
        )
        .await
        .expect("insert refresh token rt-a-1");
}

fn server_for(store: Arc<dyn OAuthBackend>) -> TestServer {
    TestServer::new(admin_router(store))
}

// ============================================================
// InMemoryStore backend
// ============================================================

#[tokio::test]
async fn in_memory_list_clients_returns_both_clients() {
    let store: Arc<dyn OAuthBackend> = Arc::new(InMemoryStore::new());
    seed_two_clients_one_session(&store).await;

    let server = server_for(store);
    let body: Vec<AdminClient> = server.get("/__admin/oauth/clients").await.json();

    assert_eq!(body.len(), 2, "both clients must be listed");
    let a = body.iter().find(|c| c.client_id == "client-a").unwrap();
    assert_eq!(a.active_sessions, 1, "client-a has one refresh token");
    let b = body.iter().find(|c| c.client_id == "client-b").unwrap();
    assert_eq!(b.active_sessions, 0, "client-b has no sessions");
}

#[tokio::test]
async fn in_memory_list_sessions_returns_only_outstanding_token() {
    let store: Arc<dyn OAuthBackend> = Arc::new(InMemoryStore::new());
    seed_two_clients_one_session(&store).await;

    let server = server_for(store);
    let body: Vec<AdminSession> = server.get("/__admin/oauth/sessions").await.json();

    assert_eq!(body.len(), 1);
    assert_eq!(body[0].client_id, "client-a");
    assert_eq!(body[0].scope, "mcp");
    assert_eq!(body[0].token_id, "rt-a-1");
}

#[tokio::test]
async fn in_memory_delete_client_cascades_sessions_and_is_idempotent() {
    let store: Arc<dyn OAuthBackend> = Arc::new(InMemoryStore::new());
    seed_two_clients_one_session(&store).await;

    let server = server_for(store.clone());

    let body: DeleteResponse = server
        .delete("/__admin/oauth/clients/client-a")
        .await
        .json();
    assert!(body.deleted, "first delete must succeed");
    assert_eq!(
        body.revoked_sessions,
        Some(1),
        "client-a had one refresh token that must cascade-delete"
    );

    let post = store.list_refresh_tokens().await.expect("list rts");
    assert!(
        post.is_empty(),
        "all refresh tokens for deleted client must be gone"
    );

    let body: DeleteResponse = server
        .delete("/__admin/oauth/clients/client-a")
        .await
        .json();
    assert!(!body.deleted, "second delete must be idempotent");
}

#[tokio::test]
async fn in_memory_delete_session_does_not_touch_client() {
    let store: Arc<dyn OAuthBackend> = Arc::new(InMemoryStore::new());
    seed_two_clients_one_session(&store).await;

    let server = server_for(store.clone());
    let body: DeleteResponse = server.delete("/__admin/oauth/sessions/rt-a-1").await.json();
    assert!(body.deleted);
    assert!(
        body.revoked_sessions.is_none(),
        "session DELETE must never set revoked_sessions"
    );

    let clients = store.list_clients().await.expect("list clients");
    assert_eq!(clients.len(), 2, "deleting a session must not drop clients");
}

#[tokio::test]
async fn in_memory_auth_codes_never_leak_into_sessions_listing() {
    // Security invariant: /__admin/oauth/sessions reports refresh tokens
    // only. Pending auth codes (consent-page-issued, single-use) must NOT
    // appear there, even when the store holds both.
    let store: Arc<dyn OAuthBackend> = Arc::new(InMemoryStore::new());
    seed_two_clients_one_session(&store).await;
    store
        .insert_auth_code(
            "ac-1".into(),
            AuthCode {
                client_id: "client-b".into(),
                redirect_uri: "https://b.example/cb".into(),
                code_challenge: "challenge".into(),
                scope: "mcp".into(),
                expires_at: now_secs() + 60,
            },
        )
        .await
        .expect("insert auth code");

    let server = server_for(store);
    let body: Vec<AdminSession> = server.get("/__admin/oauth/sessions").await.json();
    assert_eq!(
        body.len(),
        1,
        "auth codes must NOT be returned by /__admin/oauth/sessions"
    );
    assert_eq!(body[0].token_id, "rt-a-1");
}

// ============================================================
// SqliteStore backend (only when `mcp-persistence` is on)
// ============================================================

#[cfg(feature = "mcp-persistence")]
mod sqlite {
    use super::{
        AdminClient, DeleteResponse, OAuthBackend, seed_two_clients_one_session, server_for,
    };
    use std::sync::Arc;
    use tileserver_rs::mcp::auth_store_sqlite::SqliteStore;

    // SQLite recognises the special path ":memory:" as an ephemeral
    // in-process database that lives for the lifetime of the connection
    // — exactly what we want for an integration test.
    fn sqlite_store() -> Arc<dyn OAuthBackend> {
        Arc::new(SqliteStore::open(":memory:").expect("open sqlite :memory: store"))
    }

    #[tokio::test]
    async fn sqlite_list_clients_matches_in_memory_shape() {
        let store = sqlite_store();
        seed_two_clients_one_session(&store).await;

        let server = server_for(store);
        let body: Vec<AdminClient> = server.get("/__admin/oauth/clients").await.json();

        assert_eq!(body.len(), 2);
        let a = body.iter().find(|c| c.client_id == "client-a").unwrap();
        assert_eq!(a.active_sessions, 1);
    }

    #[tokio::test]
    async fn sqlite_delete_client_cascades_via_fk_constraint() {
        let store = sqlite_store();
        seed_two_clients_one_session(&store).await;

        let server = server_for(store.clone());
        let body: DeleteResponse = server
            .delete("/__admin/oauth/clients/client-a")
            .await
            .json();
        assert!(body.deleted);
        assert_eq!(body.revoked_sessions, Some(1));

        let post = store.list_refresh_tokens().await.expect("list rts");
        assert!(post.is_empty(), "FK ON DELETE CASCADE must fire");
    }
}
