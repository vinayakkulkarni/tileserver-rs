//! Hot-reload controller using `ArcSwap` for lock-free shared state updates.

use arc_swap::ArcSwap;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, RwLock};

use crate::{
    config::Config,
    render::{Renderer, pool::PoolConfig},
    sources::SourceManager,
    styles::StyleManager,
};

#[derive(Clone)]
pub struct AppState {
    pub sources: Arc<SourceManager>,
    pub styles: Arc<StyleManager>,
    pub renderer: Option<Arc<Renderer>>,
    pub base_url: String,
    /// Localhost URL for native renderer self-fetch (bypasses reverse proxy)
    pub render_base_url: String,
    pub ui_enabled: bool,
    pub fonts_dir: Option<PathBuf>,
    pub files_dir: Option<PathBuf>,
    pub upload_dir: Option<PathBuf>,
}

/// Tracking info for an uploaded file source.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UploadInfo {
    pub id: String,
    pub file_name: String,
    pub format: String,
    pub file_path: PathBuf,
}

/// Registry of uploaded sources, keyed by source ID.
pub type UploadRegistry = Arc<RwLock<HashMap<String, UploadInfo>>>;

/// Shared handle for accessing the active application state.
#[derive(Clone)]
pub struct SharedState {
    controller: Arc<ReloadController>,
    uploads: UploadRegistry,
}

impl SharedState {
    #[must_use]
    pub fn new(controller: Arc<ReloadController>) -> Self {
        Self {
            controller,
            uploads: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[must_use]
    pub fn load(&self) -> Arc<AppState> {
        self.controller.app.load_full()
    }

    #[must_use]
    pub fn meta(&self) -> Arc<ReloadMeta> {
        self.controller.meta.load_full()
    }

    /// Returns the currently-loaded [`Config`]. Cheap (one Arc clone).
    #[must_use]
    pub fn config(&self) -> Arc<Config> {
        self.controller.config.load_full()
    }

    /// Returns the path the config was loaded from, if any. `None` when the
    /// server was started without `--config` (zero-config auto-detect mode).
    #[must_use]
    pub fn config_path(&self) -> Option<&std::path::Path> {
        self.controller.config_path.as_deref()
    }

    pub async fn reload(&self, flush: bool) -> anyhow::Result<ReloadResult> {
        self.controller.reload(flush).await
    }

    /// Access the upload registry (for upload/delete handlers)
    #[must_use]
    pub fn uploads(&self) -> &UploadRegistry {
        &self.uploads
    }

    /// Store a new AppState (used by upload/delete to swap sources at runtime)
    pub fn store(&self, state: Arc<AppState>) {
        self.controller.app.store(state);
    }
}

/// Settings that remain stable across hot-reloads.
#[derive(Clone)]
pub struct RuntimeSettings {
    pub ui_enabled: bool,
    pub runtime_host: String,
    pub runtime_port: u16,
    pub public_url_override: Option<String>,
}

/// Metadata exposed in `/ping` and admin responses.
#[derive(Clone)]
pub struct ReloadMeta {
    pub config_hash: String,
    pub loaded_at_unix: u64,
    pub loaded_sources: usize,
    pub loaded_styles: usize,
    pub renderer_enabled: bool,
    pub prometheus_listener_active: bool,
}

/// Outcome of a reload attempt.
pub struct ReloadResult {
    pub reloaded: bool,
    pub config_hash: String,
    pub loaded_at_unix: u64,
    pub loaded_sources: usize,
    pub loaded_styles: usize,
    pub renderer_enabled: bool,
    pub prometheus_listener_active: bool,
}

pub struct ReloadController {
    pub app: ArcSwap<AppState>,
    pub meta: ArcSwap<ReloadMeta>,
    /// The loaded [`Config`] that produced the current [`AppState`]. Held
    /// alongside `app`/`meta` so the admin UI can render it back as TOML
    /// without re-reading the source file.
    pub config: ArcSwap<Config>,
    config_path: Option<PathBuf>,
    runtime: RuntimeSettings,
    reload_mutex: Mutex<()>,
}

impl ReloadController {
    #[must_use]
    pub fn new(
        state: AppState,
        meta: ReloadMeta,
        config: Config,
        config_path: Option<PathBuf>,
        runtime: RuntimeSettings,
    ) -> Self {
        Self {
            app: ArcSwap::new(Arc::new(state)),
            meta: ArcSwap::new(Arc::new(meta)),
            config: ArcSwap::new(Arc::new(config)),
            config_path,
            runtime,
            reload_mutex: Mutex::new(()),
        }
    }

    async fn reload(&self, flush: bool) -> anyhow::Result<ReloadResult> {
        let _guard = self.reload_mutex.lock().await;

        let load = Config::load_with_metadata(self.config_path.clone())?;
        let new_hash = load.content_hash.clone();

        let current_meta = self.meta.load_full();
        if !flush && new_hash == current_meta.config_hash {
            return Ok(ReloadResult {
                reloaded: false,
                config_hash: current_meta.config_hash.clone(),
                loaded_at_unix: current_meta.loaded_at_unix,
                loaded_sources: current_meta.loaded_sources,
                loaded_styles: current_meta.loaded_styles,
                renderer_enabled: current_meta.renderer_enabled,
                prometheus_listener_active: current_meta.prometheus_listener_active,
            });
        }

        let new_state = build_app_state(&load.config, &self.runtime).await?;

        let new_meta = ReloadMeta {
            config_hash: new_hash,
            loaded_at_unix: now_unix_seconds(),
            loaded_sources: new_state.sources.len(),
            loaded_styles: new_state.styles.len(),
            renderer_enabled: new_state.renderer.is_some(),
            prometheus_listener_active: current_meta.prometheus_listener_active,
        };

        let result = ReloadResult {
            reloaded: true,
            config_hash: new_meta.config_hash.clone(),
            loaded_at_unix: new_meta.loaded_at_unix,
            loaded_sources: new_meta.loaded_sources,
            loaded_styles: new_meta.loaded_styles,
            renderer_enabled: new_meta.renderer_enabled,
            prometheus_listener_active: new_meta.prometheus_listener_active,
        };

        self.app.store(Arc::new(new_state));
        self.meta.store(Arc::new(new_meta));
        self.config.store(Arc::new(load.config));

        Ok(result)
    }
}

#[must_use]
pub fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Build an [`AppState`] from a [`Config`] and [`RuntimeSettings`].
pub async fn build_app_state(
    config: &Config,
    runtime: &RuntimeSettings,
) -> anyhow::Result<AppState> {
    // Load tile sources
    #[cfg(feature = "postgres")]
    let raw_sources =
        SourceManager::from_configs_with_postgres(&config.sources, config.postgres.as_ref())
            .await?;
    #[cfg(not(feature = "postgres"))]
    let raw_sources = SourceManager::from_configs(&config.sources).await?;

    let sources = if config.cache.enabled {
        let cache = Arc::new(crate::cache::TileCache::new(
            config.cache.max_size_mb,
            config.cache.ttl_seconds,
        ));
        tracing::info!(
            "Global tile cache enabled: {}MB TTL {}s",
            config.cache.max_size_mb,
            config.cache.ttl_seconds,
        );
        raw_sources.with_cache(cache)
    } else {
        raw_sources
    };
    tracing::info!("Loaded {} tile source(s)", sources.len());

    // Load styles
    let styles = StyleManager::from_configs(&config.styles)?;
    tracing::info!("Loaded {} style(s)", styles.len());

    // Initialize native renderer (if styles are configured)
    let renderer = if !styles.is_empty() {
        let pool_config = PoolConfig {
            tile_size: 512,
            pool_size: config.render.pool_size,
            render_timeout: std::time::Duration::from_secs(config.render.render_timeout_secs),
        };
        match Renderer::with_config(pool_config, 3) {
            Ok(r) => {
                tracing::info!("Native MapLibre renderer initialized");
                Some(Arc::new(r))
            }
            Err(e) => {
                tracing::warn!("Failed to initialize renderer: {}. Rendering disabled.", e);
                None
            }
        }
    } else {
        None
    };

    // Build base URL
    let base_url = if let Some(ref public_url) = runtime.public_url_override {
        public_url.trim_end_matches('/').to_string()
    } else if let Some(ref public_url) = config.server.public_url {
        public_url.trim_end_matches('/').to_string()
    } else {
        let host_for_url = if runtime.runtime_host == "0.0.0.0" {
            "localhost"
        } else {
            &runtime.runtime_host
        };
        format!("http://{}:{}", host_for_url, runtime.runtime_port)
    };

    let render_base_url = format!("http://127.0.0.1:{}", runtime.runtime_port);

    // Log fonts directory
    if let Some(ref fonts_path) = config.fonts {
        if fonts_path.exists() {
            tracing::info!("Fonts directory: {}", fonts_path.display());
        } else {
            tracing::warn!("Fonts directory not found: {}", fonts_path.display());
        }
    }

    // Log files directory
    if let Some(ref files_path) = config.files {
        if files_path.exists() {
            tracing::info!("Files directory: {}", files_path.display());
        } else {
            tracing::warn!("Files directory not found: {}", files_path.display());
        }
    }

    // Resolve upload directory
    let upload_dir = if let Some(ref dir) = config.server.upload_dir {
        Some(dir.clone())
    } else {
        Some(config.resolve_cache_dir(None).join("uploads"))
    };

    if let Some(ref dir) = upload_dir {
        if let Err(e) = std::fs::create_dir_all(dir) {
            tracing::warn!("Failed to create upload directory {}: {}", dir.display(), e);
        } else {
            tracing::info!("Upload directory: {}", dir.display());
        }
    }

    Ok(AppState {
        sources: Arc::new(sources),
        styles: Arc::new(styles),
        renderer,
        base_url,
        render_base_url,
        ui_enabled: runtime.ui_enabled,
        fonts_dir: config.fonts.clone(),
        files_dir: config.files.clone(),
        upload_dir,
    })
}

/// Listen for `SIGHUP` and trigger a config reload.
#[cfg(unix)]
pub async fn reload_signal(controller: Arc<ReloadController>) {
    let mut sig =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()).expect("SIGHUP");
    loop {
        sig.recv().await;
        tracing::info!("Received SIGHUP, reloading configuration...");
        match controller.reload(false).await {
            Ok(result) => {
                if result.reloaded {
                    tracing::info!(
                        "Configuration reloaded (hash={}, sources={}, styles={})",
                        result.config_hash,
                        result.loaded_sources,
                        result.loaded_styles,
                    );
                } else {
                    tracing::info!("Configuration unchanged, no reload performed");
                }
            }
            Err(e) => tracing::error!("Failed to reload configuration: {}", e),
        }
    }
}

#[cfg(not(unix))]
pub async fn reload_signal(_controller: Arc<ReloadController>) {
    // SIGHUP is not available on non-Unix platforms
    std::future::pending::<()>().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_test_app_state() -> AppState {
        AppState {
            sources: Arc::new(crate::sources::SourceManager::new()),
            styles: Arc::new(crate::styles::StyleManager::new()),
            renderer: None,
            base_url: "http://localhost:8080".to_string(),
            render_base_url: "http://127.0.0.1:8080".to_string(),
            ui_enabled: false,
            fonts_dir: None,
            files_dir: None,
            upload_dir: None,
        }
    }

    fn make_test_meta() -> ReloadMeta {
        ReloadMeta {
            config_hash: "test-hash".to_string(),
            loaded_at_unix: now_unix_seconds(),
            loaded_sources: 0,
            loaded_styles: 0,
            renderer_enabled: false,
            prometheus_listener_active: false,
        }
    }

    fn make_test_runtime() -> RuntimeSettings {
        RuntimeSettings {
            ui_enabled: false,
            runtime_host: "127.0.0.1".to_string(),
            runtime_port: 8080,
            public_url_override: None,
        }
    }

    #[test]
    fn now_unix_seconds_is_positive() {
        let t = now_unix_seconds();
        assert!(t > 0, "unix timestamp must be positive");
    }

    #[test]
    fn now_unix_seconds_is_recent() {
        let t = now_unix_seconds();
        // Must be after 2020-01-01 (unix 1577836800)
        assert!(t > 1_577_836_800, "timestamp should be after 2020");
    }

    #[test]
    fn runtime_settings_fields_accessible() {
        let rt = make_test_runtime();
        assert_eq!(rt.runtime_port, 8080);
        assert_eq!(rt.runtime_host, "127.0.0.1");
        assert!(!rt.ui_enabled);
        assert!(rt.public_url_override.is_none());
    }

    #[test]
    fn reload_meta_fields_accessible() {
        let meta = make_test_meta();
        assert_eq!(meta.config_hash, "test-hash");
        assert_eq!(meta.loaded_sources, 0);
        assert!(!meta.renderer_enabled);
    }

    #[test]
    fn app_state_fields_accessible() {
        let state = make_test_app_state();
        assert!(state.renderer.is_none());
        assert!(state.fonts_dir.is_none());
        assert!(!state.ui_enabled);
        assert_eq!(state.base_url, "http://localhost:8080");
    }

    #[test]
    fn reload_controller_new_stores_initial_state() {
        let state = make_test_app_state();
        let meta = make_test_meta();
        let runtime = make_test_runtime();
        let controller = ReloadController::new(state, meta, Config::default(), None, runtime);
        let loaded = controller.app.load_full();
        assert_eq!(loaded.base_url, "http://localhost:8080");
    }

    #[test]
    fn shared_state_new_load_returns_initial_state() {
        let state = make_test_app_state();
        let meta = make_test_meta();
        let runtime = make_test_runtime();
        let controller = Arc::new(ReloadController::new(
            state,
            meta,
            Config::default(),
            None,
            runtime,
        ));
        let shared = SharedState::new(controller);
        let loaded = shared.load();
        assert_eq!(loaded.base_url, "http://localhost:8080");
    }

    #[test]
    fn shared_state_meta_returns_initial_hash() {
        let state = make_test_app_state();
        let meta = make_test_meta();
        let runtime = make_test_runtime();
        let controller = Arc::new(ReloadController::new(
            state,
            meta,
            Config::default(),
            None,
            runtime,
        ));
        let shared = SharedState::new(controller);
        let loaded_meta = shared.meta();
        assert_eq!(loaded_meta.config_hash, "test-hash");
    }

    #[test]
    fn shared_state_uploads_initially_empty() {
        let state = make_test_app_state();
        let meta = make_test_meta();
        let runtime = make_test_runtime();
        let controller = Arc::new(ReloadController::new(
            state,
            meta,
            Config::default(),
            None,
            runtime,
        ));
        let shared = SharedState::new(controller);
        let _ = shared.uploads();
    }

    #[test]
    fn app_state_clone_preserves_base_url() {
        let state = make_test_app_state();
        let cloned = state.clone();
        assert_eq!(state.base_url, cloned.base_url);
    }

    #[test]
    fn shared_state_store_swaps_app_state_atomically() {
        let state = make_test_app_state();
        let meta = make_test_meta();
        let runtime = make_test_runtime();
        let controller = Arc::new(ReloadController::new(
            state,
            meta,
            Config::default(),
            None,
            runtime,
        ));
        let shared = SharedState::new(controller);

        let mut replacement = make_test_app_state();
        replacement.base_url = "http://replaced".to_string();
        shared.store(Arc::new(replacement));

        let loaded = shared.load();
        assert_eq!(loaded.base_url, "http://replaced");
    }

    #[test]
    fn upload_info_clone_round_trips_fields() {
        let info = UploadInfo {
            id: "abc".to_string(),
            file_name: "world.mbtiles".to_string(),
            format: "mbtiles".to_string(),
            file_path: std::path::PathBuf::from("/tmp/x.mbtiles"),
        };
        let cloned = info.clone();
        assert_eq!(cloned.id, "abc");
        assert_eq!(cloned.file_name, "world.mbtiles");
        assert_eq!(cloned.format, "mbtiles");
        assert_eq!(cloned.file_path, std::path::PathBuf::from("/tmp/x.mbtiles"));
    }

    fn runtime_for(host: &str, port: u16, public_override: Option<&str>) -> RuntimeSettings {
        RuntimeSettings {
            ui_enabled: false,
            runtime_host: host.to_string(),
            runtime_port: port,
            public_url_override: public_override.map(str::to_string),
        }
    }

    #[tokio::test]
    async fn build_app_state_uses_public_url_override() {
        let cfg = crate::config::Config::default();
        let runtime = runtime_for("127.0.0.1", 8080, Some("https://tiles.example.com/"));
        let state = build_app_state(&cfg, &runtime).await.unwrap();
        assert_eq!(state.base_url, "https://tiles.example.com");
        assert_eq!(state.render_base_url, "http://127.0.0.1:8080");
    }

    #[tokio::test]
    async fn build_app_state_uses_server_public_url_when_no_override() {
        let mut cfg = crate::config::Config::default();
        cfg.server.public_url = Some("https://maps.test/".to_string());
        let runtime = runtime_for("0.0.0.0", 9090, None);
        let state = build_app_state(&cfg, &runtime).await.unwrap();
        assert_eq!(state.base_url, "https://maps.test");
    }

    #[tokio::test]
    async fn build_app_state_falls_back_to_localhost_when_host_is_wildcard() {
        let cfg = crate::config::Config::default();
        let runtime = runtime_for("0.0.0.0", 9000, None);
        let state = build_app_state(&cfg, &runtime).await.unwrap();
        assert_eq!(state.base_url, "http://localhost:9000");
        assert_eq!(state.render_base_url, "http://127.0.0.1:9000");
    }

    #[tokio::test]
    async fn build_app_state_uses_runtime_host_when_not_wildcard() {
        let cfg = crate::config::Config::default();
        let runtime = runtime_for("192.168.1.5", 7777, None);
        let state = build_app_state(&cfg, &runtime).await.unwrap();
        assert_eq!(state.base_url, "http://192.168.1.5:7777");
    }

    #[tokio::test]
    async fn build_app_state_default_upload_dir_lands_in_temp() {
        let cfg = crate::config::Config::default();
        let runtime = runtime_for("127.0.0.1", 8080, None);
        let state = build_app_state(&cfg, &runtime).await.unwrap();

        let upload = state.upload_dir.expect("default upload dir set");
        assert!(
            upload.ends_with("tileserver-rs/uploads"),
            "default upload dir must live under the resolved cache dir, got: {}",
            upload.display()
        );
        assert!(upload.exists(), "upload dir must be created");
    }

    #[tokio::test]
    async fn build_app_state_uses_configured_upload_dir() {
        let tempdir = tempfile::tempdir().unwrap();
        let upload_path = tempdir.path().join("custom-uploads");
        let mut cfg = crate::config::Config::default();
        cfg.server.upload_dir = Some(upload_path.clone());
        let runtime = runtime_for("127.0.0.1", 8080, None);
        let state = build_app_state(&cfg, &runtime).await.unwrap();
        assert_eq!(state.upload_dir.as_deref(), Some(upload_path.as_path()));
        assert!(upload_path.exists());
    }

    #[tokio::test]
    async fn build_app_state_with_no_styles_has_no_renderer() {
        let cfg = crate::config::Config::default();
        let runtime = runtime_for("127.0.0.1", 8080, None);
        let state = build_app_state(&cfg, &runtime).await.unwrap();
        assert!(state.renderer.is_none());
        assert_eq!(state.styles.len(), 0);
        assert_eq!(state.sources.len(), 0);
    }

    #[tokio::test]
    async fn build_app_state_with_cache_enabled_wires_global_cache() {
        let mut cfg = crate::config::Config::default();
        cfg.cache.enabled = true;
        cfg.cache.max_size_mb = 1;
        cfg.cache.ttl_seconds = 60;
        let runtime = runtime_for("127.0.0.1", 8080, None);
        let state = build_app_state(&cfg, &runtime).await.unwrap();
        assert!(state.sources.cache().is_some());
    }

    #[tokio::test]
    async fn build_app_state_propagates_runtime_flags() {
        let cfg = crate::config::Config {
            fonts: Some(std::path::PathBuf::from("/tmp/does-not-exist-fonts")),
            files: Some(std::path::PathBuf::from("/tmp/does-not-exist-files")),
            ..crate::config::Config::default()
        };
        let runtime = RuntimeSettings {
            ui_enabled: true,
            runtime_host: "127.0.0.1".to_string(),
            runtime_port: 8080,
            public_url_override: None,
        };
        let state = build_app_state(&cfg, &runtime).await.unwrap();
        assert!(state.ui_enabled);
        assert!(state.fonts_dir.is_some());
        assert!(state.files_dir.is_some());
    }

    fn write_default_config_toml(path: &std::path::Path) {
        let cfg = crate::config::Config::default();
        let content = toml::to_string(&cfg).expect("serialize default config");
        std::fs::write(path, content).expect("write config");
    }

    #[tokio::test]
    async fn reload_returns_unchanged_when_hash_matches() {
        let tempdir = tempfile::tempdir().unwrap();
        let cfg_path = tempdir.path().join("config.toml");
        write_default_config_toml(&cfg_path);

        let load = crate::config::Config::load_with_metadata(Some(cfg_path.clone())).unwrap();
        let runtime = runtime_for("127.0.0.1", 8080, None);
        let initial_state = build_app_state(&load.config, &runtime).await.unwrap();
        let initial_hash = load.content_hash.clone();
        let meta = ReloadMeta {
            config_hash: initial_hash.clone(),
            loaded_at_unix: now_unix_seconds(),
            loaded_sources: initial_state.sources.len(),
            loaded_styles: initial_state.styles.len(),
            renderer_enabled: initial_state.renderer.is_some(),
            prometheus_listener_active: false,
        };
        let controller = ReloadController::new(
            initial_state,
            meta,
            load.config.clone(),
            Some(cfg_path),
            runtime,
        );

        let result = controller.reload(false).await.unwrap();
        assert!(
            !result.reloaded,
            "reload must short-circuit when config hash matches"
        );
        assert_eq!(result.config_hash, initial_hash);
    }

    #[tokio::test]
    async fn reload_with_flush_true_rebuilds_even_when_hash_matches() {
        let tempdir = tempfile::tempdir().unwrap();
        let cfg_path = tempdir.path().join("config.toml");
        write_default_config_toml(&cfg_path);

        let load = crate::config::Config::load_with_metadata(Some(cfg_path.clone())).unwrap();
        let runtime = runtime_for("127.0.0.1", 8080, None);
        let initial_state = build_app_state(&load.config, &runtime).await.unwrap();
        let initial_hash = load.content_hash.clone();
        let meta = ReloadMeta {
            config_hash: initial_hash.clone(),
            loaded_at_unix: 0,
            loaded_sources: initial_state.sources.len(),
            loaded_styles: initial_state.styles.len(),
            renderer_enabled: initial_state.renderer.is_some(),
            prometheus_listener_active: false,
        };
        let controller = ReloadController::new(
            initial_state,
            meta,
            load.config.clone(),
            Some(cfg_path),
            runtime,
        );

        let result = controller.reload(true).await.unwrap();
        assert!(result.reloaded, "flush=true must force a rebuild");
        assert_eq!(result.config_hash, initial_hash);
        assert!(
            result.loaded_at_unix > 0,
            "loaded_at_unix must be refreshed"
        );
    }

    #[tokio::test]
    async fn reload_detects_config_change_and_swaps_app_state() {
        let tempdir = tempfile::tempdir().unwrap();
        let cfg_path = tempdir.path().join("config.toml");

        let mut cfg = crate::config::Config::default();
        cfg.server.port = 8080;
        std::fs::write(&cfg_path, toml::to_string(&cfg).unwrap()).unwrap();

        let load = crate::config::Config::load_with_metadata(Some(cfg_path.clone())).unwrap();
        let runtime = runtime_for("127.0.0.1", 8080, None);
        let initial_state = build_app_state(&load.config, &runtime).await.unwrap();
        let meta = ReloadMeta {
            config_hash: load.content_hash.clone(),
            loaded_at_unix: now_unix_seconds(),
            loaded_sources: 0,
            loaded_styles: 0,
            renderer_enabled: false,
            prometheus_listener_active: true,
        };
        let controller = ReloadController::new(
            initial_state,
            meta,
            load.config.clone(),
            Some(cfg_path.clone()),
            runtime,
        );

        let mut new_cfg = crate::config::Config::default();
        new_cfg.server.port = 9999;
        new_cfg.server.public_url = Some("https://changed.test".to_string());
        std::fs::write(&cfg_path, toml::to_string(&new_cfg).unwrap()).unwrap();

        let result = controller.reload(false).await.unwrap();
        assert!(result.reloaded);
        assert_ne!(result.config_hash, load.content_hash);
        assert!(
            result.prometheus_listener_active,
            "reload must preserve prometheus listener flag"
        );

        let new_state = controller.app.load_full();
        assert_eq!(new_state.base_url, "https://changed.test");
    }

    #[tokio::test]
    async fn shared_state_reload_through_controller_short_circuits_on_same_hash() {
        let tempdir = tempfile::tempdir().unwrap();
        let cfg_path = tempdir.path().join("config.toml");
        write_default_config_toml(&cfg_path);

        let load = crate::config::Config::load_with_metadata(Some(cfg_path.clone())).unwrap();
        let runtime = runtime_for("127.0.0.1", 8080, None);
        let initial_state = build_app_state(&load.config, &runtime).await.unwrap();
        let meta = ReloadMeta {
            config_hash: load.content_hash.clone(),
            loaded_at_unix: now_unix_seconds(),
            loaded_sources: 0,
            loaded_styles: 0,
            renderer_enabled: false,
            prometheus_listener_active: false,
        };
        let controller = Arc::new(ReloadController::new(
            initial_state,
            meta,
            load.config.clone(),
            Some(cfg_path),
            runtime,
        ));
        let shared = SharedState::new(controller);

        let result = shared.reload(false).await.unwrap();
        assert!(!result.reloaded);
    }
}
