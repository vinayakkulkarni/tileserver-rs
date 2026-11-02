//! Live source updates over WebSocket (issue #596).
//!
//! When `[live] enabled = true`, the server mounts `GET /ws`. Clients
//! receive a `reload` message for each source whose data changed, and
//! re-fetch its tiles (MapLibre: `map.getSource(id).reload()`).
//!
//! Changes come from two places:
//!
//! - a hot reload (`SIGHUP`, `POST /__admin/reload`) that changed a
//!   source's config entry, added it, or removed it;
//! - the watcher: sources listed in `[live].watch` are polled every
//!   `interval_secs`. Local files are compared by modification time and
//!   size, `http(s)` sources by `ETag` / `Last-Modified`. A change reloads
//!   the server state (like `SIGHUP`, with a fresh tile cache) and is then
//!   broadcast.
//!
//! Protocol (JSON text frames):
//!
//! ```text
//! server → client  {"type":"hello","sources":["a","b"]}
//!                  {"type":"reload","source":"a"}
//! client → server  {"type":"subscribe","source":"a"}
//!                  {"type":"unsubscribe","source":"a"}
//! ```
//!
//! A client with no subscriptions receives every source's messages. After
//! the first `subscribe` it receives only the sources it subscribed to.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::config::{Config, LiveConfig, SourceConfig};
use crate::reload::SharedState;

/// Buffered messages per client before a slow client starts to lag.
const CHANNEL_CAPACITY: usize = 256;

/// Message sent to clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ServerMessage {
    /// Sent once on connect: the sources the server serves.
    Hello { sources: Vec<String> },
    /// The source's data or config changed; re-fetch its tiles.
    Reload { source: String },
}

/// Message received from clients.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ClientMessage {
    Subscribe { source: String },
    Unsubscribe { source: String },
}

/// Broadcast hub shared by the WebSocket handler, the reload path and
/// the watcher.
#[derive(Debug, Clone)]
pub struct LiveHub {
    sender: broadcast::Sender<ServerMessage>,
    clients: Arc<AtomicUsize>,
}

impl Default for LiveHub {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveHub {
    #[must_use]
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            sender,
            clients: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Broadcast a `reload` message for each source (no-op without clients).
    pub fn publish_reloads<I: IntoIterator<Item = String>>(&self, sources: I) {
        for source in sources {
            // `send` only fails when no client is connected.
            let _ = self.sender.send(ServerMessage::Reload { source });
        }
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        self.sender.subscribe()
    }

    /// Connected WebSocket clients.
    #[must_use]
    pub fn client_count(&self) -> usize {
        self.clients.load(Ordering::Relaxed)
    }
}

/// Source ids whose config entry differs between `old` and `new`
/// (changed, added or removed), sorted.
#[must_use]
pub fn changed_sources(old: &Config, new: &Config) -> Vec<String> {
    let index = |config: &Config| -> BTreeMap<String, serde_json::Value> {
        config
            .sources
            .iter()
            .map(|s| {
                (
                    s.id.clone(),
                    serde_json::to_value(s).unwrap_or(serde_json::Value::Null),
                )
            })
            .collect()
    };
    let (old, new) = (index(old), index(new));
    let ids: BTreeSet<&String> = old.keys().chain(new.keys()).collect();
    ids.into_iter()
        .filter(|id| old.get(*id) != new.get(*id))
        .cloned()
        .collect()
}

/// Whether `client` should receive `message` given its subscriptions.
fn wants(subscriptions: &BTreeSet<String>, message: &ServerMessage) -> bool {
    match message {
        ServerMessage::Hello { .. } => true,
        ServerMessage::Reload { source } => {
            subscriptions.is_empty() || subscriptions.contains(source)
        }
    }
}

/// `GET /ws`: upgrade to a WebSocket that streams [`ServerMessage`]s.
pub async fn ws_handler(State(shared): State<SharedState>, upgrade: WebSocketUpgrade) -> Response {
    let config = shared.config();
    let hub = shared.live().clone();
    let reserved = hub
        .clients
        .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
            (n < config.live.max_clients).then_some(n + 1)
        })
        .is_ok();
    if !reserved {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "too many WebSocket clients",
        )
            .into_response();
    }
    let mut sources: Vec<String> = shared
        .load()
        .sources
        .ids()
        .into_iter()
        .map(str::to_owned)
        .collect();
    sources.sort();
    upgrade.on_upgrade(move |socket| async move {
        serve_client(socket, &hub, sources).await;
        hub.clients.fetch_sub(1, Ordering::AcqRel);
    })
}

async fn serve_client(mut socket: WebSocket, hub: &LiveHub, sources: Vec<String>) {
    let mut receiver = hub.subscribe();
    let mut subscriptions: BTreeSet<String> = BTreeSet::new();
    if send(&mut socket, &ServerMessage::Hello { sources })
        .await
        .is_err()
    {
        return;
    }
    loop {
        tokio::select! {
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(ClientMessage::Subscribe { source }) => {
                            subscriptions.insert(source);
                        }
                        Ok(ClientMessage::Unsubscribe { source }) => {
                            subscriptions.remove(&source);
                        }
                        Err(e) => tracing::debug!("ignoring WebSocket message: {e}"),
                    }
                }
                Some(Ok(Message::Close(_)) | Err(_)) | None => return,
                // Ping/pong are answered by axum; binary frames are ignored.
                Some(Ok(_)) => {}
            },
            outgoing = receiver.recv() => match outgoing {
                Ok(message) => {
                    if wants(&subscriptions, &message) && send(&mut socket, &message).await.is_err() {
                        return;
                    }
                }
                // The client fell behind; it may have missed reloads, so
                // tell it to reload everything it subscribed to.
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    tracing::warn!("WebSocket client lagged; sending reloads for all subscriptions");
                    for source in subscriptions.clone() {
                        if send(&mut socket, &ServerMessage::Reload { source }).await.is_err() {
                            return;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => return,
            },
        }
    }
}

async fn send(socket: &mut WebSocket, message: &ServerMessage) -> Result<(), axum::Error> {
    let text = serde_json::to_string(message).unwrap_or_default();
    socket.send(Message::Text(text.into())).await
}

/// A source's change fingerprint (modification time + size, or the HTTP
/// validators). `None` when it cannot be read; that is not a change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fingerprint {
    File {
        modified: Option<SystemTime>,
        len: u64,
    },
    Http {
        etag: Option<String>,
        last_modified: Option<String>,
        len: Option<u64>,
    },
}

fn file_fingerprint(path: &Path) -> Option<Fingerprint> {
    let meta = std::fs::metadata(path).ok()?;
    Some(Fingerprint::File {
        modified: meta.modified().ok(),
        len: meta.len(),
    })
}

async fn http_fingerprint(client: &reqwest::Client, url: &str) -> Option<Fingerprint> {
    let response = client.head(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let header = |name: reqwest::header::HeaderName| {
        response
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    Some(Fingerprint::Http {
        etag: header(reqwest::header::ETAG),
        last_modified: header(reqwest::header::LAST_MODIFIED),
        len: header(reqwest::header::CONTENT_LENGTH).and_then(|v| v.parse().ok()),
    })
}

async fn fingerprint(client: &reqwest::Client, source: &SourceConfig) -> Option<Fingerprint> {
    let path = source.path.as_str();
    if path.starts_with("http://") || path.starts_with("https://") {
        http_fingerprint(client, path).await
    } else {
        file_fingerprint(Path::new(path))
    }
}

/// Sources the watcher polls: every source when `watch` is `["*"]`,
/// otherwise the listed ids that exist.
#[must_use]
pub fn watched_sources<'a>(
    live: &LiveConfig,
    sources: &'a [SourceConfig],
) -> Vec<&'a SourceConfig> {
    let all = live.watch.iter().any(|w| w == "*");
    sources
        .iter()
        .filter(|s| all || live.watch.contains(&s.id))
        .filter(|s| !s.path.is_empty())
        .collect()
}

/// Ids whose fingerprint changed since `previous`; updates `previous`.
/// A source seen for the first time is recorded, not reported.
pub fn diff_fingerprints(
    previous: &mut HashMap<String, Fingerprint>,
    current: HashMap<String, Fingerprint>,
) -> Vec<String> {
    let mut changed: Vec<String> = current
        .iter()
        .filter(|(id, fp)| previous.get(*id).is_some_and(|old| old != *fp))
        .map(|(id, _)| id.clone())
        .collect();
    changed.sort();
    for (id, fp) in current {
        previous.insert(id, fp);
    }
    changed
}

/// Poll the `[live].watch` sources and reload + broadcast on change.
/// Runs until the process exits; reads the current config each round so
/// a hot reload can change the watch list.
pub async fn run_watcher(shared: SharedState) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();
    let mut previous: HashMap<String, Fingerprint> = HashMap::new();
    loop {
        let config = shared.config();
        let interval = Duration::from_secs(config.live.interval_secs.max(1));
        if config.live.enabled && !config.live.watch.is_empty() {
            let mut current = HashMap::new();
            for source in watched_sources(&config.live, &config.sources) {
                if let Some(fp) = fingerprint(&client, source).await {
                    current.insert(source.id.clone(), fp);
                }
            }
            let changed = diff_fingerprints(&mut previous, current);
            if !changed.is_empty() {
                tracing::info!(sources = ?changed, "watched sources changed; reloading");
                match shared.reload_sources(&changed).await {
                    Ok(()) => {}
                    Err(e) => tracing::error!("reload after source change failed: {e:#}"),
                }
            }
        }
        tokio::time::sleep(interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str, path: &str) -> SourceConfig {
        toml::from_str(&format!(
            "id = \"{id}\"\ntype = \"pmtiles\"\npath = \"{path}\"\n"
        ))
        .expect("valid source")
    }

    fn config(sources: Vec<SourceConfig>) -> Config {
        Config {
            sources,
            ..Config::default()
        }
    }

    #[test]
    fn messages_use_the_issue_wire_format() {
        assert_eq!(
            serde_json::to_string(&ServerMessage::Reload {
                source: "osm".into()
            })
            .unwrap(),
            r#"{"type":"reload","source":"osm"}"#
        );
        assert_eq!(
            serde_json::to_string(&ServerMessage::Hello {
                sources: vec!["a".into()]
            })
            .unwrap(),
            r#"{"type":"hello","sources":["a"]}"#
        );
        assert_eq!(
            serde_json::from_str::<ClientMessage>(r#"{"type":"subscribe","source":"osm"}"#)
                .unwrap(),
            ClientMessage::Subscribe {
                source: "osm".into()
            }
        );
        assert!(serde_json::from_str::<ClientMessage>(r#"{"type":"nope"}"#).is_err());
    }

    #[test]
    fn subscriptions_filter_reloads() {
        let reload = |s: &str| ServerMessage::Reload { source: s.into() };
        let none = BTreeSet::new();
        assert!(wants(&none, &reload("a")));
        let only_a = BTreeSet::from(["a".to_owned()]);
        assert!(wants(&only_a, &reload("a")));
        assert!(!wants(&only_a, &reload("b")));
        assert!(wants(&only_a, &ServerMessage::Hello { sources: vec![] }));
    }

    #[test]
    fn changed_sources_finds_edits_additions_and_removals() {
        let old = config(vec![
            source("a", "a.pmtiles"),
            source("b", "b.pmtiles"),
            source("c", "c.pmtiles"),
        ]);
        let new = config(vec![
            source("a", "a.pmtiles"),
            source("b", "b2.pmtiles"),
            source("d", "d.pmtiles"),
        ]);
        assert_eq!(changed_sources(&old, &new), vec!["b", "c", "d"]);
        assert!(changed_sources(&old, &old).is_empty());
    }

    #[test]
    fn watch_list_selects_sources() {
        let sources = vec![
            source("a", "a.pmtiles"),
            source("b", "b.pmtiles"),
            source("c", ""),
        ];
        let live = |watch: &[&str]| LiveConfig {
            watch: watch.iter().map(|s| (*s).to_owned()).collect(),
            ..LiveConfig::default()
        };
        let ids = |l: &LiveConfig| -> Vec<String> {
            watched_sources(l, &sources)
                .iter()
                .map(|s| s.id.clone())
                .collect()
        };
        assert_eq!(ids(&live(&["b", "zzz"])), vec!["b"]);
        assert_eq!(
            ids(&live(&["*"])),
            vec!["a", "b"],
            "sources without a path are skipped"
        );
        assert!(ids(&live(&[])).is_empty());
    }

    #[test]
    fn fingerprint_diff_reports_only_changes_after_first_sight() {
        let fp = |len| Fingerprint::File {
            modified: None,
            len,
        };
        let mut seen = HashMap::new();
        assert!(diff_fingerprints(&mut seen, HashMap::from([("a".into(), fp(1))])).is_empty());
        assert!(diff_fingerprints(&mut seen, HashMap::from([("a".into(), fp(1))])).is_empty());
        assert_eq!(
            diff_fingerprints(
                &mut seen,
                HashMap::from([("a".into(), fp(2)), ("b".into(), fp(1))])
            ),
            vec!["a"]
        );
    }

    #[test]
    fn file_fingerprint_tracks_size_and_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.pmtiles");
        std::fs::write(&path, b"one").unwrap();
        let first = file_fingerprint(&path).unwrap();
        std::fs::write(&path, b"three").unwrap();
        assert_ne!(file_fingerprint(&path).unwrap(), first);
        assert!(file_fingerprint(&dir.path().join("missing")).is_none());
    }

    /// HEAD validators from a real HTTP server: a changed body changes the
    /// fingerprint, a 404 or an unreachable host gives `None`.
    #[tokio::test]
    async fn http_fingerprint_reads_validators() {
        use axum::http::header;
        let body = std::sync::Arc::new(std::sync::Mutex::new("v1"));
        let shared = std::sync::Arc::clone(&body);
        let app = axum::Router::new().route(
            "/a.pmtiles",
            axum::routing::get(move || {
                let v = *shared.lock().unwrap();
                async move { ([(header::ETAG, format!("\"{v}\""))], v) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await });

        let client = reqwest::Client::new();
        let url = format!("http://{addr}/a.pmtiles");
        let first = http_fingerprint(&client, &url).await.unwrap();
        assert_eq!(
            first,
            Fingerprint::Http {
                etag: Some("\"v1\"".into()),
                last_modified: None,
                len: Some(2)
            }
        );
        *body.lock().unwrap() = "v22";
        assert_ne!(http_fingerprint(&client, &url).await.unwrap(), first);
        assert_eq!(
            fingerprint(&client, &source("a", &url)).await,
            http_fingerprint(&client, &url).await
        );
        assert!(
            http_fingerprint(&client, &format!("http://{addr}/missing"))
                .await
                .is_none()
        );
        assert!(
            http_fingerprint(&client, "http://127.0.0.1:9/x")
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn hub_broadcasts_and_counts_nothing_without_clients() {
        let hub = LiveHub::default();
        hub.publish_reloads(["lost".to_owned()]); // no receivers: dropped
        let mut rx = hub.subscribe();
        hub.publish_reloads(["a".to_owned(), "b".to_owned()]);
        assert_eq!(
            rx.recv().await.unwrap(),
            ServerMessage::Reload { source: "a".into() }
        );
        assert_eq!(
            rx.recv().await.unwrap(),
            ServerMessage::Reload { source: "b".into() }
        );
        assert_eq!(hub.client_count(), 0);
    }
}
