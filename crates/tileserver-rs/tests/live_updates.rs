//! End-to-end tests for live source updates over WebSocket (`GET /ws`,
//! issue #596): a real HTTP server, a real WebSocket client, and reloads
//! driven through the same `SharedState` the server uses.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum_test::{TestServer, TestWebSocket};
use serde_json::{Value, json};
use tileserver_rs::config::Config;
use tileserver_rs::reload::{
    ReloadController, ReloadMeta, SharedState, build_app_state, now_unix_seconds,
};
use tileserver_rs::routes::api_router;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/tiles")
        .join(name)
}

fn write_config(path: &Path, tiles: &Path, name: &str, live: &str) {
    let toml = format!(
        r#"
[[sources]]
id = "florence"
type = "pmtiles"
path = "{}"
name = "{name}"

[[sources]]
id = "zurich"
type = "mbtiles"
path = "{}"

[live]
{live}
"#,
        tiles.display(),
        fixture("zurich_switzerland.mbtiles").display()
    );
    std::fs::write(path, toml).unwrap();
}

struct Harness {
    _dir: tempfile::TempDir,
    config_path: PathBuf,
    tiles: PathBuf,
    shared: SharedState,
    server: TestServer,
}

async fn harness(live: &str) -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let tiles = dir.path().join("florence.pmtiles");
    std::fs::copy(fixture("protomaps-sample.pmtiles"), &tiles).unwrap();
    let config_path = dir.path().join("config.toml");
    write_config(&config_path, &tiles, "Florence", live);

    let load = Config::load_with_metadata(Some(config_path.clone())).unwrap();
    let runtime = common::minimal_runtime();
    let state = build_app_state(&load.config, &runtime).await.unwrap();
    let meta = ReloadMeta {
        config_hash: load.content_hash,
        loaded_at_unix: now_unix_seconds(),
        loaded_sources: state.sources.len(),
        loaded_styles: 0,
        renderer_enabled: false,
        prometheus_listener_active: false,
    };
    let controller = Arc::new(ReloadController::new(
        state,
        meta,
        load.config,
        Some(config_path.clone()),
        runtime,
    ));
    let shared = SharedState::new(controller);
    let server = TestServer::builder()
        .http_transport()
        .build(api_router(shared.clone()));
    Harness {
        _dir: dir,
        config_path,
        tiles,
        shared,
        server,
    }
}

async fn next_json(ws: &mut TestWebSocket) -> Value {
    tokio::time::timeout(Duration::from_secs(5), ws.receive_json::<Value>())
        .await
        .expect("message within 5 s")
}

#[tokio::test]
async fn ws_is_404_when_live_updates_are_off() {
    let h = harness("enabled = false").await;
    h.server
        .get_websocket("/ws")
        .await
        .assert_status_not_found();
}

#[tokio::test]
async fn hello_lists_sources_and_config_change_sends_reload() {
    let h = harness("enabled = true").await;
    let mut ws = h.server.get_websocket("/ws").await.into_websocket().await;
    assert_eq!(
        next_json(&mut ws).await,
        json!({ "type": "hello", "sources": ["florence", "zurich"] })
    );

    // Edit only the florence entry and hot-reload: one message, for it.
    write_config(&h.config_path, &h.tiles, "Firenze", "enabled = true");
    assert!(h.shared.reload(false).await.unwrap().reloaded);
    assert_eq!(
        next_json(&mut ws).await,
        json!({ "type": "reload", "source": "florence" })
    );

    // A data change (as the watcher reports it) is broadcast as well.
    h.shared
        .reload_sources(&["zurich".to_owned()])
        .await
        .unwrap();
    assert_eq!(
        next_json(&mut ws).await,
        json!({ "type": "reload", "source": "zurich" })
    );
}

#[tokio::test]
async fn subscriptions_limit_messages_to_chosen_sources() {
    let h = harness("enabled = true").await;
    let mut ws = h.server.get_websocket("/ws").await.into_websocket().await;
    next_json(&mut ws).await; // hello

    ws.send_json(&json!({ "type": "subscribe", "source": "zurich" }))
        .await;
    // Let the server task apply the subscription before broadcasting.
    tokio::time::sleep(Duration::from_millis(100)).await;

    h.shared
        .reload_sources(&["florence".to_owned(), "zurich".to_owned()])
        .await
        .unwrap();
    assert_eq!(
        next_json(&mut ws).await,
        json!({ "type": "reload", "source": "zurich" })
    );
    // Unknown or malformed messages are ignored; unsubscribing from the
    // only subscription goes back to receiving everything.
    ws.send_text("not json").await;
    ws.send_json(&json!({ "type": "unsubscribe", "source": "zurich" }))
        .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    h.shared
        .reload_sources(&["florence".to_owned()])
        .await
        .unwrap();
    assert_eq!(
        next_json(&mut ws).await,
        json!({ "type": "reload", "source": "florence" })
    );

    // Only zurich again: florence is filtered out, nothing else arrives.
    ws.send_json(&json!({ "type": "subscribe", "source": "zurich" }))
        .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    h.shared
        .reload_sources(&["florence".to_owned()])
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(300), ws.receive_text())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn clients_beyond_max_clients_get_503() {
    let h = harness("enabled = true\nmax_clients = 1").await;
    let mut first = h.server.get_websocket("/ws").await.into_websocket().await;
    next_json(&mut first).await;
    h.server
        .get_websocket("/ws")
        .await
        .assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(h.shared.live().client_count(), 1);

    // Closing frees the slot.
    first.close().await;
    for _ in 0..50 {
        if h.shared.live().client_count() == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(h.shared.live().client_count(), 0);
    let mut second = h.server.get_websocket("/ws").await.into_websocket().await;
    next_json(&mut second).await;
}

#[tokio::test]
async fn forced_reload_sends_every_source() {
    let h = harness("enabled = true").await;
    let mut ws = h.server.get_websocket("/ws").await.into_websocket().await;
    next_json(&mut ws).await;
    h.shared.reload(true).await.unwrap();
    assert_eq!(
        next_json(&mut ws).await,
        json!({ "type": "reload", "source": "florence" })
    );
    assert_eq!(
        next_json(&mut ws).await,
        json!({ "type": "reload", "source": "zurich" })
    );
}

#[tokio::test]
async fn watcher_reloads_and_broadcasts_when_a_watched_file_changes() {
    let h = harness("enabled = true\nwatch = [\"florence\"]\ninterval_secs = 1").await;
    let mut ws = h.server.get_websocket("/ws").await.into_websocket().await;
    next_json(&mut ws).await; // hello

    let watcher = tokio::spawn(tileserver_rs::live::run_watcher(h.shared.clone()));
    // First poll records the fingerprint; change the file after it.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let mut bytes = std::fs::read(&h.tiles).unwrap();
    bytes.extend_from_slice(&[0u8; 16]); // trailing bytes: still a valid archive
    std::fs::write(&h.tiles, bytes).unwrap();

    assert_eq!(
        next_json(&mut ws).await,
        json!({ "type": "reload", "source": "florence" })
    );
    watcher.abort();
}
