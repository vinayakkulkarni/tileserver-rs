//! `--serve` integration: boot the existing HTTP server against a freshly
//! converted PMTiles archive using a synthetic single-source config.

use crate::config::Config;
use crate::error::{Result, TileServerError};
use crate::reload::{
    ReloadController, ReloadMeta, RuntimeSettings, SharedState, build_app_state, now_unix_seconds,
};
use crate::routes;
use std::path::Path;
use std::sync::Arc;

/// Default port used by `--serve` when `--port` is omitted.
pub const DEFAULT_SERVE_PORT: u16 = 8080;

/// Build a synthetic single-source [`Config`] pointing at `pmtiles_path`.
///
/// The config is expressed as TOML and parsed through the normal deserializer
/// so every `#[serde(default)]` field is populated without hand-constructing
/// the large [`crate::config::SourceConfig`] across every cargo feature.
///
/// # Errors
///
/// Returns [`TileServerError::ConvertError`] when the synthetic config fails to
/// parse (which would indicate an internal bug, not user error).
pub fn synthetic_config(
    pmtiles_path: &Path,
    source_id: &str,
    host: &str,
    port: u16,
) -> Result<Config> {
    let toml = format!(
        "[server]\nhost = \"{host}\"\nport = {port}\n\n[[sources]]\nid = \"{id}\"\ntype = \"pmtiles\"\npath = \"{path}\"\n",
        host = host,
        port = port,
        id = source_id,
        path = pmtiles_path.display(),
    );
    toml::from_str::<Config>(&toml)
        .map_err(|e| TileServerError::ConvertError(format!("synthetic config: {e}")))
}

/// Serve the converted archive over HTTP, reusing the standard app-state build
/// and API router. Blocks until the server shuts down.
///
/// # Errors
///
/// Returns an error if the source fails to load or the listener cannot bind.
pub async fn serve_pmtiles(
    pmtiles_path: &Path,
    source_id: &str,
    host: &str,
    port: u16,
) -> anyhow::Result<()> {
    let config = synthetic_config(pmtiles_path, source_id, host, port)?;
    let runtime = RuntimeSettings {
        ui_enabled: false,
        runtime_host: config.server.host.clone(),
        runtime_port: config.server.port,
        public_url_override: None,
    };

    let state = build_app_state(&config, &runtime).await?;
    let meta = ReloadMeta {
        config_hash: String::new(),
        loaded_at_unix: now_unix_seconds(),
        loaded_sources: state.sources.len(),
        loaded_styles: state.styles.len(),
        renderer_enabled: state.renderer.is_some(),
        prometheus_listener_active: false,
    };
    let controller = Arc::new(ReloadController::new(
        state,
        meta,
        config.clone(),
        None,
        runtime,
    ));
    let shared = SharedState::new(Arc::clone(&controller));

    let router = routes::api_router(shared);
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Serving converted tiles on http://{addr}/data/{source_id}.json");
    axum::serve(listener, router).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_config_has_single_source() {
        let cfg =
            synthetic_config(Path::new("/tmp/out.pmtiles"), "cities", "127.0.0.1", 8080).unwrap();
        assert_eq!(cfg.sources.len(), 1);
        assert_eq!(cfg.sources[0].id, "cities");
        assert_eq!(cfg.server.port, 8080);
    }

    #[test]
    fn synthetic_config_uses_given_host_and_port() {
        let cfg = synthetic_config(Path::new("/data/x.pmtiles"), "x", "0.0.0.0", 9000).unwrap();
        assert_eq!(cfg.server.host, "0.0.0.0");
        assert_eq!(cfg.server.port, 9000);
        assert_eq!(cfg.sources[0].path, "/data/x.pmtiles");
    }
}
