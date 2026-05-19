//! Model Context Protocol (MCP) server integration.
//!
//! This module is only compiled when the `mcp` feature is enabled. It
//! exposes:
//!
//! - **Tier A introspection tools** — `tileserver_list_sources`,
//!   `tileserver_get_source_tilejson`, `tileserver_list_styles`,
//!   `tileserver_get_style`, `tileserver_get_tile_metadata`,
//!   `tileserver_get_server_info`.
//! - **Tier B data-access tools** — `tileserver_render_static_map`,
//!   `tileserver_get_tile`, `tileserver_query_features_at_point`, and
//!   (feature-gated) `tileserver_query_features_cql2` /
//!   `tileserver_search_stac_items`.
//! - **Tier D resource templates** — `tileserver://styles/{id}` and
//!   `tileserver://data/{id}.json`.
//!
//! Two transports are supported:
//!
//! 1. Streamable HTTP at `/mcp` (mounted on the main listener).
//! 2. stdio (`tileserver-rs mcp-stdio --config …`).

#![deny(clippy::correctness)]

use std::path::PathBuf;
use std::sync::Arc;

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::reload::{RuntimeSettings, build_app_state};
use crate::startup;

pub mod admin_routes;
pub mod auth;
pub mod auth_store;
#[cfg(feature = "mcp-persistence")]
pub mod auth_store_sqlite;
pub mod error;
pub mod handlers;
pub mod prompts;
pub mod resources;
pub mod transport;

pub use handlers::McpHandler;
pub use transport::{mcp_router, run_stdio};

/// Initialise tracing for stdio mode: subscribers write to stderr so that
/// stdout is reserved for MCP JSON-RPC framing.
///
/// # Errors
///
/// Returns an error when the verbose-mode filter directive fails to parse.
pub fn init_stdio_tracing(verbose: bool) -> anyhow::Result<()> {
    let filter = if verbose {
        EnvFilter::from_default_env().add_directive("tileserver_rs=debug".parse()?)
    } else {
        EnvFilter::from_default_env().add_directive("tileserver_rs=info".parse()?)
    };
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .compact();
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
    Ok(())
}

/// Build an [`AppState`](crate::reload::AppState) suitable for stdio MCP
/// service: forces `ui_enabled = false`, drops any `public_url_override`,
/// and runs the same config-load and state-build pipeline used by the HTTP
/// server.
///
/// # Errors
///
/// Returns an error when the runtime configuration cannot be loaded or when
/// state construction fails.
pub async fn build_stdio_state(
    config_path: Option<PathBuf>,
) -> anyhow::Result<crate::reload::AppState> {
    let (config, _auto_report) = startup::load_runtime_config(config_path, None)?;
    let runtime = RuntimeSettings {
        ui_enabled: false,
        runtime_host: config.server.host.clone(),
        runtime_port: config.server.port,
        public_url_override: None,
    };
    build_app_state(&config, &runtime).await
}

/// Run the MCP server over stdio: init tracing, load config, build state,
/// hand off to [`run_stdio`] which blocks until the client disconnects.
///
/// # Errors
///
/// Returns an error when tracing init, config loading, state construction,
/// or the underlying rmcp service terminates abnormally.
pub async fn run_stdio_from_config(
    config_path: Option<PathBuf>,
    verbose: bool,
) -> anyhow::Result<()> {
    init_stdio_tracing(verbose)?;
    let state = build_stdio_state(config_path).await?;
    tracing::info!(
        sources = state.sources.len(),
        styles = state.styles.len(),
        "starting MCP stdio server"
    );
    run_stdio(Arc::new(state)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn build_stdio_state_loads_explicit_config_file() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg_path = tmp.path().join("config.toml");
        std::fs::write(
            &cfg_path,
            r#"
[server]
host = "127.0.0.1"
port = 9999
"#,
        )
        .unwrap();
        let state = build_stdio_state(Some(cfg_path)).await.unwrap();
        assert_eq!(state.sources.len(), 0);
        assert_eq!(state.styles.len(), 0);
    }

    #[tokio::test]
    async fn build_stdio_state_forces_ui_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg_path = tmp.path().join("config.toml");
        std::fs::write(&cfg_path, "[server]\nport = 8080\n").unwrap();
        let state = build_stdio_state(Some(cfg_path)).await.unwrap();
        assert!(
            !state.ui_enabled,
            "stdio state must always force ui_enabled=false; got {}",
            state.ui_enabled
        );
    }

    #[tokio::test]
    async fn build_stdio_state_errors_for_missing_config_path() {
        let bogus = PathBuf::from("/nonexistent/path/to/nothing.toml");
        match build_stdio_state(Some(bogus)).await {
            Ok(_) => panic!("expected error for nonexistent config path"),
            Err(e) => assert!(
                e.to_string().contains("not found"),
                "expected not-found error, got: {e}"
            ),
        }
    }

    #[test]
    fn init_stdio_tracing_filter_directive_parses() {
        let _ = EnvFilter::from_default_env().add_directive("tileserver_rs=debug".parse().unwrap());
        let _ = EnvFilter::from_default_env().add_directive("tileserver_rs=info".parse().unwrap());
    }
}
