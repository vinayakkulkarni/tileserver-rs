//! End-to-end test for the analytics dashboard (`GET /__admin/stats`, #602).
//!
//! Lives in its own test binary because dashboard collection is process
//! global: other test binaries build app state with the default
//! (collection off), which would reset the store mid-test.

mod common;

use std::collections::HashMap;
use std::sync::Arc;

use axum_test::TestServer;
use serde_json::Value;
use tileserver_rs::admin::admin_router;
use tileserver_rs::config::Config;
use tileserver_rs::metrics::dashboard::{self, DashboardSettings};
use tileserver_rs::reload::{ReloadController, SharedState};
use tileserver_rs::routes::api_router;
use tileserver_rs::sources::{SourceManager, TileSource};

use common::{MockSource, minimal_app_state, minimal_meta, minimal_runtime};

#[tokio::test]
async fn tile_requests_appear_in_the_stats_snapshot() {
    dashboard::configure(DashboardSettings {
        enabled: true,
        recent_requests: 10,
        heatmap_zoom: 4,
    });

    let mut map: HashMap<String, Arc<dyn TileSource>> = HashMap::new();
    map.insert("osm".to_owned(), Arc::new(MockSource::pbf("osm")));
    let mut state = minimal_app_state();
    state.sources = Arc::new(SourceManager::from_sources(map));
    let controller = Arc::new(ReloadController::new(
        state,
        minimal_meta(),
        Config::default(),
        None,
        minimal_runtime(),
    ));
    let shared = SharedState::new(controller);
    let api = TestServer::new(api_router(shared.clone()));
    let admin = TestServer::new(admin_router(shared));

    for _ in 0..3 {
        api.get("/data/osm/14/8192/5461.pbf")
            .await
            .assert_status_ok();
    }
    api.get("/data/osm/2/1/1.pbf").await.assert_status_ok();

    let resp = admin.get("/__admin/stats").await;
    resp.assert_status_ok();
    let body: Value = resp.json();

    assert_eq!(body["enabled"], true);
    assert_eq!(body["totals"]["requests"], 4);
    assert_eq!(body["sources"][0]["id"], "osm");
    assert_eq!(body["sources"][0]["requests"], 4);

    let zooms: Vec<(u64, u64)> = body["zooms"]
        .as_array()
        .expect("zooms")
        .iter()
        .map(|z| (z["z"].as_u64().unwrap(), z["requests"].as_u64().unwrap()))
        .collect();
    assert_eq!(zooms, vec![(2, 1), (14, 3)]);

    assert_eq!(body["top_tiles"][0]["z"], 14);
    assert_eq!(body["top_tiles"][0]["x"], 8192);
    assert_eq!(body["top_tiles"][0]["requests"], 3);

    let recent = body["recent"].as_array().expect("recent");
    assert_eq!(recent.len(), 4);
    assert_eq!(recent[0]["z"], 2, "newest first");

    let heat_total: u64 = body["heatmap"]
        .as_array()
        .expect("heatmap")
        .iter()
        .map(|h| h["requests"].as_u64().unwrap())
        .sum();
    assert_eq!(heat_total, 4);
    assert_eq!(body["heatmap_zoom"], 4);
}
