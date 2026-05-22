//! Disk-backed SQLite implementation of [`super::auth_store::OAuthBackend`].
//!
//! Compiled only when the `mcp-persistence` Cargo feature is on. Survives
//! server restarts (every issued refresh token remains valid until its
//! `expires_at`); without this feature the store is purely in-memory and
//! restart invalidates every connected app + device session.
//!
//! # Schema
//!
//! ```sql
//! CREATE TABLE clients (
//!     client_id           TEXT PRIMARY KEY NOT NULL,
//!     client_secret       TEXT,
//!     redirect_uris       TEXT NOT NULL,  -- JSON array
//!     client_name         TEXT
//! );
//! CREATE TABLE auth_codes (
//!     code                TEXT PRIMARY KEY NOT NULL,
//!     client_id           TEXT NOT NULL REFERENCES clients(client_id)
//!                                ON DELETE CASCADE,
//!     redirect_uri        TEXT NOT NULL,
//!     code_challenge      TEXT NOT NULL,
//!     scope               TEXT NOT NULL,
//!     expires_at          INTEGER NOT NULL
//! );
//! CREATE TABLE refresh_tokens (
//!     token               TEXT PRIMARY KEY NOT NULL,
//!     client_id           TEXT NOT NULL REFERENCES clients(client_id)
//!                                ON DELETE CASCADE,
//!     scope               TEXT NOT NULL,
//!     expires_at          INTEGER NOT NULL
//! );
//! CREATE INDEX refresh_tokens_client_id ON refresh_tokens(client_id);
//! CREATE INDEX auth_codes_client_id ON auth_codes(client_id);
//! ```
//!
//! `clients.redirect_uris` is JSON-encoded because the OAuth flow allows
//! multiple URIs per client and SQLite has no array type. The two foreign
//! keys cascade so `delete_client()` atomically removes the client and
//! every refresh token issued to it — the same invariant the in-memory
//! implementation enforces by hand.
//!
//! # Concurrency
//!
//! rusqlite is synchronous; SQLite serialises writes at the database
//! level. To avoid blocking the Tokio runtime, every operation runs on
//! `tokio::task::spawn_blocking`. We hold a single `Arc<Mutex<Connection>>`
//! (rather than r2d2): connection-establish cost is one-time, and writes
//! must serialise anyway because of SQLite's `BEGIN IMMEDIATE` semantics.
//! For the OAuth flow this is fast — token issuance is bounded by JWT
//! signing, not database writes.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use rusqlite::{Connection, OptionalExtension, params};
use tokio::sync::Mutex;

use super::auth::{AuthCode, RefreshTokenEntry, RegisteredClient};
use super::auth_store::{BackendError, BackendResult, OAuthBackend};

/// Disk-backed SQLite backend.
///
/// Construct with [`SqliteStore::open`]. The schema is applied
/// idempotently on every open so first-launch and existing-database
/// startups follow the same code path.
#[derive(Debug)]
pub struct SqliteStore {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl SqliteStore {
    /// Open (or create) a SQLite database at `path` and apply the schema.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Storage`] if the file cannot be opened
    /// (parent directory missing, disk full, permission denied) or if the
    /// schema migration fails (rare; usually indicates an in-place file
    /// format change between SQLite versions).
    pub fn open(path: impl Into<PathBuf>) -> BackendResult<Self> {
        let path = path.into();
        let conn = Connection::open(&path).map_err(|e| {
            BackendError::Storage(format!("open SQLite at {}: {e}", path.display()))
        })?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| BackendError::Storage(format!("enable foreign_keys: {e}")))?;
        Self::migrate(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
        })
    }

    /// Path to the underlying SQLite file. Useful for diagnostics and the
    /// admin `/__admin/oauth/diagnostics` endpoint.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn migrate(conn: &Connection) -> BackendResult<()> {
        conn.execute_batch(
            r"
            CREATE TABLE IF NOT EXISTS clients (
                client_id     TEXT PRIMARY KEY NOT NULL,
                client_secret TEXT,
                redirect_uris TEXT NOT NULL,
                client_name   TEXT
            );
            CREATE TABLE IF NOT EXISTS auth_codes (
                code           TEXT PRIMARY KEY NOT NULL,
                client_id      TEXT NOT NULL REFERENCES clients(client_id) ON DELETE CASCADE,
                redirect_uri   TEXT NOT NULL,
                code_challenge TEXT NOT NULL,
                scope          TEXT NOT NULL,
                expires_at     INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS auth_codes_client_id ON auth_codes(client_id);
            CREATE TABLE IF NOT EXISTS refresh_tokens (
                token      TEXT PRIMARY KEY NOT NULL,
                client_id  TEXT NOT NULL REFERENCES clients(client_id) ON DELETE CASCADE,
                scope      TEXT NOT NULL,
                expires_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS refresh_tokens_client_id ON refresh_tokens(client_id);
            ",
        )
        .map_err(|e| BackendError::Storage(format!("schema migration: {e}")))?;
        Ok(())
    }
}

#[allow(clippy::cognitive_complexity)]
fn map_sqlite_err(op: &'static str, e: rusqlite::Error) -> BackendError {
    BackendError::Storage(format!("{op}: {e}"))
}

async fn with_conn<F, R>(conn: &Arc<Mutex<Connection>>, f: F) -> BackendResult<R>
where
    F: FnOnce(&mut Connection) -> BackendResult<R> + Send + 'static,
    R: Send + 'static,
{
    let conn = Arc::clone(conn);
    tokio::task::spawn_blocking(move || {
        let mut guard = conn.blocking_lock();
        f(&mut guard)
    })
    .await
    .map_err(|e| BackendError::Storage(format!("blocking task join: {e}")))?
}

#[async_trait]
impl OAuthBackend for SqliteStore {
    async fn insert_client(
        &self,
        client_id: String,
        client: RegisteredClient,
    ) -> BackendResult<()> {
        let redirect_uris = serde_json::to_string(&client.redirect_uris)
            .map_err(|e| BackendError::Storage(format!("encode redirect_uris: {e}")))?;
        with_conn(&self.conn, move |c| {
            c.execute(
                "INSERT OR REPLACE INTO clients
                   (client_id, client_secret, redirect_uris, client_name)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    client_id,
                    client.client_secret,
                    redirect_uris,
                    client.client_name,
                ],
            )
            .map(|_| ())
            .map_err(|e| map_sqlite_err("insert_client", e))
        })
        .await
    }

    async fn get_client(&self, client_id: &str) -> BackendResult<Option<RegisteredClient>> {
        let id = client_id.to_string();
        with_conn(&self.conn, move |c| {
            c.query_row(
                "SELECT client_id, client_secret, redirect_uris, client_name
                   FROM clients WHERE client_id = ?1",
                params![id],
                row_to_client,
            )
            .optional()
            .map_err(|e| map_sqlite_err("get_client", e))
        })
        .await
    }

    async fn list_clients(&self) -> BackendResult<Vec<RegisteredClient>> {
        with_conn(&self.conn, move |c| {
            let mut stmt = c
                .prepare(
                    "SELECT client_id, client_secret, redirect_uris, client_name
                       FROM clients ORDER BY client_id",
                )
                .map_err(|e| map_sqlite_err("list_clients prepare", e))?;
            let rows = stmt
                .query_map([], row_to_client)
                .map_err(|e| map_sqlite_err("list_clients query", e))?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| map_sqlite_err("list_clients row", e))?);
            }
            Ok(out)
        })
        .await
    }

    async fn delete_client(&self, client_id: &str) -> BackendResult<()> {
        let id = client_id.to_string();
        with_conn(&self.conn, move |c| {
            c.execute("DELETE FROM clients WHERE client_id = ?1", params![id])
                .map(|_| ())
                .map_err(|e| map_sqlite_err("delete_client", e))
        })
        .await
    }

    async fn insert_auth_code(&self, code: String, entry: AuthCode) -> BackendResult<()> {
        let expires = i64::try_from(entry.expires_at).unwrap_or(i64::MAX);
        with_conn(&self.conn, move |c| {
            c.execute(
                "INSERT OR REPLACE INTO auth_codes
                   (code, client_id, redirect_uri, code_challenge, scope, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    code,
                    entry.client_id,
                    entry.redirect_uri,
                    entry.code_challenge,
                    entry.scope,
                    expires,
                ],
            )
            .map(|_| ())
            .map_err(|e| map_sqlite_err("insert_auth_code", e))
        })
        .await
    }

    async fn take_auth_code(&self, code: &str) -> BackendResult<Option<AuthCode>> {
        let code = code.to_string();
        with_conn(&self.conn, move |c| {
            let tx = c
                .transaction()
                .map_err(|e| map_sqlite_err("take_auth_code begin", e))?;
            let row: Option<AuthCode> = tx
                .query_row(
                    "SELECT client_id, redirect_uri, code_challenge, scope, expires_at
                       FROM auth_codes WHERE code = ?1",
                    params![code],
                    row_to_auth_code,
                )
                .optional()
                .map_err(|e| map_sqlite_err("take_auth_code select", e))?;
            if row.is_some() {
                tx.execute("DELETE FROM auth_codes WHERE code = ?1", params![code])
                    .map_err(|e| map_sqlite_err("take_auth_code delete", e))?;
            }
            tx.commit()
                .map_err(|e| map_sqlite_err("take_auth_code commit", e))?;
            Ok(row)
        })
        .await
    }

    async fn insert_refresh_token(
        &self,
        token: String,
        entry: RefreshTokenEntry,
    ) -> BackendResult<()> {
        let expires = i64::try_from(entry.expires_at).unwrap_or(i64::MAX);
        with_conn(&self.conn, move |c| {
            c.execute(
                "INSERT OR REPLACE INTO refresh_tokens
                   (token, client_id, scope, expires_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![token, entry.client_id, entry.scope, expires],
            )
            .map(|_| ())
            .map_err(|e| map_sqlite_err("insert_refresh_token", e))
        })
        .await
    }

    async fn take_refresh_token(&self, token: &str) -> BackendResult<Option<RefreshTokenEntry>> {
        let token = token.to_string();
        with_conn(&self.conn, move |c| {
            let tx = c
                .transaction()
                .map_err(|e| map_sqlite_err("take_refresh_token begin", e))?;
            let row: Option<RefreshTokenEntry> = tx
                .query_row(
                    "SELECT client_id, scope, expires_at FROM refresh_tokens WHERE token = ?1",
                    params![token],
                    row_to_refresh,
                )
                .optional()
                .map_err(|e| map_sqlite_err("take_refresh_token select", e))?;
            if row.is_some() {
                tx.execute(
                    "DELETE FROM refresh_tokens WHERE token = ?1",
                    params![token],
                )
                .map_err(|e| map_sqlite_err("take_refresh_token delete", e))?;
            }
            tx.commit()
                .map_err(|e| map_sqlite_err("take_refresh_token commit", e))?;
            Ok(row)
        })
        .await
    }

    async fn list_refresh_tokens(&self) -> BackendResult<Vec<(String, RefreshTokenEntry)>> {
        with_conn(&self.conn, move |c| {
            let mut stmt = c
                .prepare(
                    "SELECT token, client_id, scope, expires_at
                       FROM refresh_tokens ORDER BY expires_at DESC",
                )
                .map_err(|e| map_sqlite_err("list_refresh_tokens prepare", e))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        RefreshTokenEntry {
                            client_id: row.get(1)?,
                            scope: row.get(2)?,
                            expires_at: u64::try_from(row.get::<_, i64>(3)?).unwrap_or(0),
                        },
                    ))
                })
                .map_err(|e| map_sqlite_err("list_refresh_tokens query", e))?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row.map_err(|e| map_sqlite_err("list_refresh_tokens row", e))?);
            }
            Ok(out)
        })
        .await
    }

    async fn delete_refresh_token(&self, token: &str) -> BackendResult<()> {
        let token = token.to_string();
        with_conn(&self.conn, move |c| {
            c.execute(
                "DELETE FROM refresh_tokens WHERE token = ?1",
                params![token],
            )
            .map(|_| ())
            .map_err(|e| map_sqlite_err("delete_refresh_token", e))
        })
        .await
    }
}

fn row_to_client(row: &rusqlite::Row<'_>) -> rusqlite::Result<RegisteredClient> {
    let redirect_uris_json: String = row.get(2)?;
    let redirect_uris: Vec<String> = serde_json::from_str(&redirect_uris_json).unwrap_or_default();
    Ok(RegisteredClient {
        client_id: row.get(0)?,
        client_secret: row.get(1)?,
        redirect_uris,
        client_name: row.get(3)?,
    })
}

fn row_to_auth_code(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuthCode> {
    Ok(AuthCode {
        client_id: row.get(0)?,
        redirect_uri: row.get(1)?,
        code_challenge: row.get(2)?,
        scope: row.get(3)?,
        expires_at: u64::try_from(row.get::<_, i64>(4)?).unwrap_or(0),
    })
}

fn row_to_refresh(row: &rusqlite::Row<'_>) -> rusqlite::Result<RefreshTokenEntry> {
    Ok(RefreshTokenEntry {
        client_id: row.get(0)?,
        scope: row.get(1)?,
        expires_at: u64::try_from(row.get::<_, i64>(2)?).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_client(id: &str) -> RegisteredClient {
        RegisteredClient {
            client_id: id.to_string(),
            client_secret: Some("sec".to_string()),
            redirect_uris: vec![
                "http://localhost/cb".to_string(),
                "http://localhost/cb2".to_string(),
            ],
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

    fn tmp_store() -> SqliteStore {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oauth.sqlite");
        std::mem::forget(dir);
        SqliteStore::open(path).unwrap()
    }

    #[tokio::test]
    async fn insert_and_get_client_roundtrips() {
        let store = tmp_store();
        store
            .insert_client("c-1".to_string(), sample_client("c-1"))
            .await
            .unwrap();
        let got = store.get_client("c-1").await.unwrap();
        let c = got.expect("client should exist");
        assert_eq!(c.client_id, "c-1");
        assert_eq!(c.redirect_uris.len(), 2);
        assert_eq!(c.client_secret.as_deref(), Some("sec"));
    }

    #[tokio::test]
    async fn list_clients_orders_by_id_and_returns_all() {
        let store = tmp_store();
        for id in ["b", "a", "c"] {
            store
                .insert_client(id.to_string(), sample_client(id))
                .await
                .unwrap();
        }
        let ids: Vec<_> = store
            .list_clients()
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.client_id)
            .collect();
        assert_eq!(ids, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[tokio::test]
    async fn delete_client_cascades_refresh_tokens_via_foreign_key() {
        let store = tmp_store();
        store
            .insert_client("c-1".to_string(), sample_client("c-1"))
            .await
            .unwrap();
        store
            .insert_client("c-2".to_string(), sample_client("c-2"))
            .await
            .unwrap();
        store
            .insert_refresh_token("rt-a".to_string(), sample_refresh("c-1"))
            .await
            .unwrap();
        store
            .insert_refresh_token("rt-b".to_string(), sample_refresh("c-1"))
            .await
            .unwrap();
        store
            .insert_refresh_token("rt-c".to_string(), sample_refresh("c-2"))
            .await
            .unwrap();
        store.delete_client("c-1").await.unwrap();
        assert!(store.get_client("c-1").await.unwrap().is_none());
        let remaining: Vec<_> = store
            .list_refresh_tokens()
            .await
            .unwrap()
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        assert_eq!(remaining, vec!["rt-c".to_string()]);
    }

    #[tokio::test]
    async fn take_auth_code_consumes_exactly_once() {
        let store = tmp_store();
        store
            .insert_client("c-1".to_string(), sample_client("c-1"))
            .await
            .unwrap();
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
        let store = tmp_store();
        store
            .insert_client("c-1".to_string(), sample_client("c-1"))
            .await
            .unwrap();
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
    async fn schema_is_idempotent_across_reopens() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("oauth.sqlite");
        {
            let store = SqliteStore::open(&path).unwrap();
            store
                .insert_client("c-1".to_string(), sample_client("c-1"))
                .await
                .unwrap();
        }
        let store = SqliteStore::open(&path).unwrap();
        let got = store.get_client("c-1").await.unwrap();
        assert!(got.is_some(), "client must persist across reopen");
    }
}
