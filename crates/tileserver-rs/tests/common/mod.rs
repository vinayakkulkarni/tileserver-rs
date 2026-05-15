//! Shared test harness for integration tests.
//!
//! Usage: add `mod common;` at top of test file, then call helpers.

use axum_test::TestServer;
use std::sync::Arc;
use tileserver_rs::{
    reload::{
        AppState, ReloadController, ReloadMeta, RuntimeSettings, SharedState, now_unix_seconds,
    },
    routes::api_router,
    sources::SourceManager,
    styles::StyleManager,
};

/// Build a minimal [`RuntimeSettings`] suitable for tests.
pub fn minimal_runtime() -> RuntimeSettings {
    RuntimeSettings {
        ui_enabled: false,
        runtime_host: "127.0.0.1".to_string(),
        runtime_port: 8080,
        public_url_override: None,
    }
}

/// Build a minimal [`ReloadMeta`] with test values.
pub fn minimal_meta() -> ReloadMeta {
    ReloadMeta {
        config_hash: "test-hash-00000000".to_string(),
        loaded_at_unix: now_unix_seconds(),
        loaded_sources: 0,
        loaded_styles: 0,
        renderer_enabled: false,
        prometheus_listener_active: false,
    }
}

/// Build an [`AppState`] with no sources, no styles, no renderer.
///
/// All optional fields are `None`. Base URLs point to localhost:8080.
pub fn minimal_app_state() -> AppState {
    AppState {
        sources: Arc::new(SourceManager::new()),
        styles: Arc::new(StyleManager::new()),
        renderer: None,
        base_url: "http://localhost:8080".to_string(),
        render_base_url: "http://127.0.0.1:8080".to_string(),
        ui_enabled: false,
        fonts_dir: None,
        files_dir: None,
        upload_dir: None,
    }
}

/// Build a [`SharedState`] backed by an empty [`AppState`].
pub fn minimal_shared_state() -> SharedState {
    let state = minimal_app_state();
    let meta = minimal_meta();
    let controller = Arc::new(ReloadController::new(state, meta, None, minimal_runtime()));
    SharedState::new(controller)
}

/// Build a [`TestServer`] with a fully-empty app state (no sources, no styles).
///
/// Use this for testing 404 responses, empty list responses, and routing.
pub fn empty_test_server() -> TestServer {
    let shared = minimal_shared_state();
    let router = api_router(shared);
    TestServer::new(router)
}
