use arc_swap::ArcSwap;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;

use crate::{
    config::Config, render::Renderer, sources::SourceManager, styles::StyleManager, AppState,
};

/// Shared handle for accessing the active application state
#[derive(Clone)]
pub struct SharedState {
    controller: Arc<ReloadController>,
}

impl SharedState {
    pub fn new(controller: Arc<ReloadController>) -> Self {
        Self { controller }
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
}

/// Runtime settings that should remain stable across reloads
#[derive(Clone)]
pub struct RuntimeSettings {
    pub ui_enabled: bool,
    pub runtime_host: String,
    pub runtime_port: u16,
    pub public_url_override: Option<String>,
}

/// Reload metadata exposed in health and admin responses
#[derive(Clone)]
pub struct ReloadMeta {
    pub config_hash: String,
    pub loaded_at_unix: u64,
    pub loaded_sources: usize,
    pub loaded_styles: usize,
    pub renderer_enabled: bool,
}

/// Reload result for admin and signal-triggered reloads
pub struct ReloadResult {
    pub reloaded: bool,
    pub config_hash: String,
    pub loaded_at_unix: u64,
    pub loaded_sources: usize,
    pub loaded_styles: usize,
    pub renderer_enabled: bool,
}

/// Coordinates reloads and swaps in the new state atomically
pub struct ReloadController {
    app: Arc<ArcSwap<AppState>>,
    meta: Arc<ArcSwap<ReloadMeta>>,
    config_path: Option<PathBuf>,
    runtime: RuntimeSettings,
    reload_lock: Mutex<()>,
}

impl ReloadController {
    pub fn new(
        state: AppState,
        meta: ReloadMeta,
        config_path: Option<PathBuf>,
        runtime: RuntimeSettings,
    ) -> Self {
        Self {
            app: Arc::new(ArcSwap::new(Arc::new(state))),
            meta: Arc::new(ArcSwap::new(Arc::new(meta))),
            config_path,
            runtime,
            reload_lock: Mutex::new(()),
        }
    }

    pub async fn reload(&self, flush: bool) -> anyhow::Result<ReloadResult> {
        let _guard = self.reload_lock.lock().await;
        let load = Config::load_with_metadata(self.config_path.clone())?;
        let new_hash = load.content_hash.clone();

        let current_meta = self.meta.load_full();
        if !flush && new_hash == current_meta.config_hash {
            let state = self.app.load_full();
            return Ok(ReloadResult {
                reloaded: false,
                config_hash: current_meta.config_hash.clone(),
                loaded_at_unix: current_meta.loaded_at_unix,
                loaded_sources: state.sources.len(),
                loaded_styles: state.styles.len(),
                renderer_enabled: state.renderer.is_some(),
            });
        }

        let new_state = build_app_state(&load.config, &self.runtime).await?;
        let loaded_at_unix = now_unix_seconds();
        let meta = ReloadMeta {
            config_hash: new_hash.clone(),
            loaded_at_unix,
            loaded_sources: new_state.sources.len(),
            loaded_styles: new_state.styles.len(),
            renderer_enabled: new_state.renderer.is_some(),
        };

        self.app.store(Arc::new(new_state));
        self.meta.store(Arc::new(meta.clone()));

        Ok(ReloadResult {
            reloaded: true,
            config_hash: new_hash,
            loaded_at_unix,
            loaded_sources: meta.loaded_sources,
            loaded_styles: meta.loaded_styles,
            renderer_enabled: meta.renderer_enabled,
        })
    }
}

/// Current Unix timestamp in seconds
pub fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Compute the base URL for tile and style metadata responses
fn build_base_url(config: &Config, runtime: &RuntimeSettings) -> String {
    if let Some(ref public_url) = runtime
        .public_url_override
        .as_ref()
        .or(config.server.public_url.as_ref())
    {
        return public_url.trim_end_matches('/').to_string();
    }

    let host_for_url = if runtime.runtime_host == "0.0.0.0" {
        "localhost"
    } else {
        runtime.runtime_host.as_str()
    };

    format!("http://{}:{}", host_for_url, runtime.runtime_port)
}

/// Build application state from configuration and runtime settings
pub async fn build_app_state(
    config: &Config,
    runtime: &RuntimeSettings,
) -> anyhow::Result<AppState> {
    #[cfg(feature = "postgres")]
    let sources =
        SourceManager::from_configs_with_postgres(&config.sources, config.postgres.as_ref())
            .await?;
    #[cfg(not(feature = "postgres"))]
    let sources = SourceManager::from_configs(&config.sources).await?;
    tracing::info!("Loaded {} tile source(s)", sources.len());

    let styles = StyleManager::from_configs(&config.styles)?;
    tracing::info!("Loaded {} style(s)", styles.len());

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

    let base_url = build_base_url(config, runtime);

    if let Some(ref fonts_path) = config.fonts {
        if fonts_path.exists() {
            tracing::info!("Fonts directory: {}", fonts_path.display());
        } else {
            tracing::warn!("Fonts directory not found: {}", fonts_path.display());
        }
    }

    if let Some(ref files_path) = config.files {
        if files_path.exists() {
            tracing::info!("Files directory: {}", files_path.display());
        } else {
            tracing::warn!("Files directory not found: {}", files_path.display());
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
    })
}

pub async fn reload_signal(controller: Arc<ReloadController>) {
    #[cfg(unix)]
    {
        let mut hup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
            .expect("failed to install SIGHUP handler");
        while hup.recv().await.is_some() {
            tracing::info!("SIGHUP received, reloading configuration");
            if let Err(err) = controller.reload(false).await {
                tracing::error!("Reload failed, keeping existing state: {}", err);
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = controller;
    }
}
