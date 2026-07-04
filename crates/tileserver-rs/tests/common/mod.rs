//! Shared test harness for integration tests.
//!
//! Usage: add `mod common;` at top of test file, then call helpers.

#![allow(dead_code)] // helpers are referenced across multiple test binaries

use async_trait::async_trait;
use axum_test::TestServer;
use bytes::Bytes;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tileserver_rs::{
    TileCompression, TileData, TileFormat, TileSource,
    config::{Config, StyleConfig},
    reload::{
        AppState, ReloadController, ReloadMeta, RuntimeSettings, SharedState, now_unix_seconds,
    },
    routes::api_router,
    sources::{SourceManager, TileMetadata},
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
    let controller = Arc::new(ReloadController::new(
        state,
        meta,
        Config::default(),
        None,
        minimal_runtime(),
    ));
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

/// Build a [`TestServer`] from an empty [`AppState`] but a caller-supplied
/// [`Config`]. Lets route-gating tests flip `server.disable_render` /
/// `server.disable_ogc` and assert the affected routes are unregistered.
pub fn test_server_with_config(config: Config) -> TestServer {
    let state = minimal_app_state();
    let meta = minimal_meta();
    let controller = Arc::new(ReloadController::new(
        state,
        meta,
        config,
        None,
        minimal_runtime(),
    ));
    let shared = SharedState::new(controller);
    TestServer::new(api_router(shared))
}

// ============================================================
// MockSource — in-memory tile source for integration tests
// ============================================================

/// A configurable in-memory [`TileSource`] suitable for integration tests.
///
/// Lets tests construct sources with arbitrary metadata, tile payloads, and
/// compression without touching the filesystem or pulling in a real
/// PMTiles/MBTiles fixture.
pub struct MockSource {
    meta: TileMetadata,
    tile: Option<TileData>,
}

impl MockSource {
    /// Vector (PBF) source serving a fixed (non-gzipped) tile payload.
    pub fn pbf(id: &str) -> Self {
        Self {
            meta: TileMetadata {
                id: id.to_string(),
                name: id.to_string(),
                description: None,
                attribution: None,
                format: TileFormat::Pbf,
                minzoom: 0,
                maxzoom: 14,
                bounds: Some([-180.0, -85.0, 180.0, 85.0]),
                center: Some([0.0, 0.0, 2.0]),
                vector_layers: Some(serde_json::json!([
                    {
                        "id": "buildings",
                        "description": "building footprints",
                        "minzoom": 0,
                        "maxzoom": 14,
                        "fields": {
                            "height": "number",
                            "name": "string",
                        },
                    },
                    {
                        "id": "roads",
                    },
                ])),
            },
            tile: Some(TileData {
                data: Bytes::from_static(b"mock-pbf-bytes"),
                format: TileFormat::Pbf,
                compression: TileCompression::None,
            }),
        }
    }

    /// Vector (PBF) source whose tiles carry the gzip [`TileCompression`]
    /// marker. The payload itself is *not* a real gzip stream; this only
    /// drives callers that branch on `tile.compression`.
    pub fn pbf_gzip(id: &str) -> Self {
        let mut src = Self::pbf(id);
        if let Some(ref mut t) = src.tile {
            t.compression = TileCompression::Gzip;
        }
        src
    }

    /// Raster (PNG) source — feature queries should reject this.
    pub fn png(id: &str) -> Self {
        let mut src = Self::pbf(id);
        src.meta.format = TileFormat::Png;
        src.meta.vector_layers = None;
        if let Some(ref mut t) = src.tile {
            t.format = TileFormat::Png;
        }
        src
    }

    /// PBF source that returns `None` from `get_tile` (i.e. nothing cached at
    /// any coordinate).  Useful for hitting `TileNotFound` branches.
    pub fn empty(id: &str) -> Self {
        let mut src = Self::pbf(id);
        src.tile = None;
        src
    }

    /// PBF source with no `center` and no `bounds` — exercises default-tile
    /// fallback branches in the spatial query handler.
    pub fn no_center(id: &str) -> Self {
        let mut src = Self::pbf(id);
        src.meta.center = None;
        src.meta.bounds = None;
        src
    }

    /// MLT (vector) source — used by style auto-gen viewer-compat tests.
    pub fn mlt(id: &str) -> Self {
        let mut src = Self::pbf(id);
        src.meta.format = TileFormat::Mlt;
        if let Some(ref mut t) = src.tile {
            t.format = TileFormat::Mlt;
        }
        src
    }

    /// Override the source's `vector_layers` metadata.
    #[must_use]
    pub fn with_vector_layers(mut self, vl: serde_json::Value) -> Self {
        self.meta.vector_layers = Some(vl);
        self
    }

    /// Override zoom range.
    #[must_use]
    pub fn with_zoom(mut self, minzoom: u8, maxzoom: u8) -> Self {
        self.meta.minzoom = minzoom;
        self.meta.maxzoom = maxzoom;
        self
    }

    /// Serve exact `bytes` (uncompressed PBF) for every tile request.
    #[must_use]
    pub fn with_tile_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.tile = Some(TileData {
            data: Bytes::from(bytes),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        });
        self
    }

    /// Serve exact `bytes` marked as gzip-compressed for every tile request.
    #[must_use]
    pub fn with_gzip_tile_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.tile = Some(TileData {
            data: Bytes::from(bytes),
            format: TileFormat::Pbf,
            compression: TileCompression::Gzip,
        });
        self
    }
}

#[async_trait]
impl TileSource for MockSource {
    async fn get_tile(&self, _z: u8, _x: u32, _y: u32) -> tileserver_rs::Result<Option<TileData>> {
        Ok(self.tile.clone())
    }

    fn metadata(&self) -> &TileMetadata {
        &self.meta
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Build a [`TestServer`] populated with the supplied mock sources.
///
/// Each entry becomes a routable source under its `id`.
pub fn server_with_sources(sources: Vec<Arc<dyn TileSource>>) -> TestServer {
    server_with_sources_and_config(sources, Config::default())
}

/// Like [`server_with_sources`] but with a caller-supplied [`Config`] so
/// composite / named-source tests can register `[[composites]]` entries.
pub fn server_with_sources_and_config(
    sources: Vec<Arc<dyn TileSource>>,
    config: Config,
) -> TestServer {
    let mut map: HashMap<String, Arc<dyn TileSource>> = HashMap::new();
    for s in sources {
        map.insert(s.metadata().id.clone(), s);
    }
    let mut state = minimal_app_state();
    state.sources = Arc::new(SourceManager::from_sources(map));
    let meta = minimal_meta();
    let controller = Arc::new(ReloadController::new(
        state,
        meta,
        config,
        None,
        minimal_runtime(),
    ));
    let shared = SharedState::new(controller);
    let router = api_router(shared);
    TestServer::new(router)
}

// ============================================================
// Populated state builders for MCP behavioral tests
// ============================================================

/// Build a `StyleManager` containing the on-disk `protomaps-light` style.
///
/// The style fixture is resolved relative to `CARGO_MANIFEST_DIR` so the
/// builder works regardless of the test's CWD.
#[must_use]
pub fn protomaps_light_style_manager() -> StyleManager {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let style_path = manifest_dir.join("../../data/styles/protomaps-light/style.json");
    let style_config = StyleConfig {
        id: "protomaps-light".to_string(),
        path: style_path,
        name: Some("Protomaps Light".to_string()),
    };
    StyleManager::from_configs(&[style_config]).expect("load protomaps-light style")
}

/// Build a [`SharedState`] populated with two mock vector sources
/// (`alpha-source`, `beta-source`) but no styles or renderer.
///
/// Used by MCP handler tests that need source listing, get-by-id, and tile
/// metadata fixtures without a real PMTiles/MBTiles file on disk.
#[must_use]
pub fn shared_state_with_two_sources() -> SharedState {
    let mut map: HashMap<String, Arc<dyn TileSource>> = HashMap::with_capacity(2);
    map.insert(
        "alpha-source".to_string(),
        Arc::new(MockSource::pbf("alpha-source")),
    );
    map.insert(
        "beta-source".to_string(),
        Arc::new(MockSource::pbf("beta-source")),
    );

    let mut state = minimal_app_state();
    state.sources = Arc::new(SourceManager::from_sources(map));

    let meta = minimal_meta();
    let controller = Arc::new(ReloadController::new(
        state,
        meta,
        Config::default(),
        None,
        minimal_runtime(),
    ));
    SharedState::new(controller)
}

/// Build a [`SharedState`] populated with one mock source plus the
/// `protomaps-light` style — gives MCP tests a single end-to-end fixture
/// covering both source and style code paths.
#[must_use]
pub fn shared_state_populated() -> SharedState {
    let mut map: HashMap<String, Arc<dyn TileSource>> = HashMap::with_capacity(2);
    map.insert(
        "alpha-source".to_string(),
        Arc::new(MockSource::pbf("alpha-source")),
    );
    map.insert(
        "beta-source".to_string(),
        Arc::new(MockSource::pbf("beta-source")),
    );

    let mut state = minimal_app_state();
    state.sources = Arc::new(SourceManager::from_sources(map));
    state.styles = Arc::new(protomaps_light_style_manager());

    let meta = minimal_meta();
    let controller = Arc::new(ReloadController::new(
        state,
        meta,
        Config::default(),
        None,
        minimal_runtime(),
    ));
    SharedState::new(controller)
}

/// Build a [`SharedState`] containing one source whose `get_tile` always
/// returns `Ok(None)` — exercises the `TileNotFound` branch in
/// `tileserver_get_tile` without depending on a real tile store.
#[must_use]
pub fn shared_state_with_empty_source() -> SharedState {
    let mut map: HashMap<String, Arc<dyn TileSource>> = HashMap::with_capacity(1);
    map.insert(
        "empty-source".to_string(),
        Arc::new(MockSource::empty("empty-source")),
    );

    let mut state = minimal_app_state();
    state.sources = Arc::new(SourceManager::from_sources(map));

    let meta = minimal_meta();
    let controller = Arc::new(ReloadController::new(
        state,
        meta,
        Config::default(),
        None,
        minimal_runtime(),
    ));
    SharedState::new(controller)
}
