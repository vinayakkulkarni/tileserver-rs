use arc_swap::ArcSwap;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, RwLock};

use crate::{config::Config, render::Renderer, sources::SourceManager, styles::StyleManager};

#[derive(Clone)]
pub struct AppState {
    pub sources: Arc<SourceManager>,
    pub styles: Arc<StyleManager>,
    pub renderer: Option<Arc<Renderer>>,
    pub base_url: String,
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
    pub fn new(controller: Arc<ReloadController>) -> Self {
        Self {
            controller,
            uploads: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn load(&self) -> Arc<AppState> {
        self.controller.app.load_full()
    }

    pub fn meta(&self) -> Arc<ReloadMeta> {
        self.controller.meta.load_full()
    }

    pub async fn reload(&self, flush: bool) -> anyhow::Result<ReloadResult> {
        self.controller.reload(flush).await
    }

    /// Access the upload registry (for upload/delete handlers)
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
}

/// Outcome of a reload attempt.
pub struct ReloadResult {
    pub reloaded: bool,
    pub config_hash: String,
    pub loaded_at_unix: u64,
    pub loaded_sources: usize,
    pub loaded_styles: usize,
    pub renderer_enabled: bool,
}

pub struct ReloadController {
    pub app: ArcSwap<AppState>,
    pub meta: ArcSwap<ReloadMeta>,
    config_path: Option<PathBuf>,
    runtime: RuntimeSettings,
    reload_mutex: Mutex<()>,
}

impl ReloadController {
    pub fn new(
        state: AppState,
        meta: ReloadMeta,
        config_path: Option<PathBuf>,
        runtime: RuntimeSettings,
    ) -> Self {
        Self {
            app: ArcSwap::new(Arc::new(state)),
            meta: ArcSwap::new(Arc::new(meta)),
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
            });
        }

        let new_state = build_app_state(&load.config, &self.runtime).await?;

        let new_meta = ReloadMeta {
            config_hash: new_hash,
            loaded_at_unix: now_unix_seconds(),
            loaded_sources: new_state.sources.len(),
            loaded_styles: new_state.styles.len(),
            renderer_enabled: new_state.renderer.is_some(),
        };

        let result = ReloadResult {
            reloaded: true,
            config_hash: new_meta.config_hash.clone(),
            loaded_at_unix: new_meta.loaded_at_unix,
            loaded_sources: new_meta.loaded_sources,
            loaded_styles: new_meta.loaded_styles,
            renderer_enabled: new_meta.renderer_enabled,
        };

        self.app.store(Arc::new(new_state));
        self.meta.store(Arc::new(new_meta));

        Ok(result)
    }
}

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
    let sources =
        SourceManager::from_configs_with_postgres(&config.sources, config.postgres.as_ref())
            .await?;
    #[cfg(not(feature = "postgres"))]
    let sources = SourceManager::from_configs(&config.sources).await?;
    tracing::info!("Loaded {} tile source(s)", sources.len());

    // Load styles
    let styles = StyleManager::from_configs(&config.styles)?;
    tracing::info!("Loaded {} style(s)", styles.len());

    // Initialize native renderer (if styles are configured)
    let renderer = if !styles.is_empty() {
        match Renderer::new() {
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
        Some(std::env::temp_dir().join("tileserver-uploads"))
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
