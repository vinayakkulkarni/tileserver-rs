//! CLI argument parsing via `clap` for server configuration and startup options.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "tileserver-rs")]
#[command(author, version, about = "A high-performance tile server for PMTiles and MBTiles", long_about = None)]
pub struct Cli {
    /// Optional subcommand. When omitted the binary runs the HTTP server
    /// (default behavior). Subcommands are used for alternative entry
    /// points like `mcp-stdio` that take control of stdin/stdout.
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to a tile file or directory to auto-detect sources/styles from
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Path to configuration file
    #[arg(short, long, value_name = "FILE", env = "TILESERVER_CONFIG")]
    pub config: Option<PathBuf>,

    /// Host to bind to
    #[arg(long, env = "TILESERVER_HOST")]
    pub host: Option<String>,

    /// Port to bind to
    #[arg(short, long, env = "TILESERVER_PORT")]
    pub port: Option<u16>,

    /// Public URL for tile URLs in TileJSON (e.g., http://localhost:4000)
    #[arg(long, env = "TILESERVER_PUBLIC_URL")]
    pub public_url: Option<String>,

    /// Enable the web UI (enabled by default)
    #[arg(long, env = "TILESERVER_UI", default_value = "true")]
    pub ui: bool,

    /// Disable the web UI
    #[arg(long, conflicts_with = "ui")]
    pub no_ui: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

/// Top-level subcommands.
///
/// New variants here must be additive: omitting `--command` MUST keep the
/// flat HTTP-server invocation (`tileserver-rs --port 8080`) working as it
/// has historically.
#[derive(Subcommand, Debug)]
#[non_exhaustive]
pub enum Commands {
    /// Run the MCP server over stdio (for Claude Desktop and other local
    /// MCP clients). Reads the same config as the HTTP server but ignores
    /// `[server]` host/port — stdin/stdout becomes the transport.
    #[cfg(feature = "mcp")]
    McpStdio {
        /// Path to configuration file. When omitted, the same priority
        /// chain used by the HTTP server applies.
        #[arg(short, long, value_name = "FILE", env = "TILESERVER_CONFIG")]
        config: Option<PathBuf>,

        /// Enable verbose logging on stderr (stdout is reserved for MCP).
        #[arg(short, long)]
        verbose: bool,
    },
}

impl Cli {
    #[must_use]
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Returns whether the UI should be enabled
    #[must_use]
    pub fn ui_enabled(&self) -> bool {
        !self.no_ui && self.ui
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("failed to parse CLI args")
    }

    #[test]
    fn test_cli_no_args() {
        let cli = parse(&["tileserver-rs"]);
        assert!(cli.path.is_none());
        assert!(cli.config.is_none());
        assert!(cli.host.is_none());
        assert!(cli.port.is_none());
        assert!(cli.public_url.is_none());
        assert!(cli.ui);
        assert!(!cli.no_ui);
        assert!(!cli.verbose);
    }

    #[test]
    fn test_cli_positional_path() {
        let cli = parse(&["tileserver-rs", "/data/tiles"]);
        assert_eq!(cli.path.unwrap(), PathBuf::from("/data/tiles"));
    }

    #[test]
    fn test_cli_config_short() {
        let cli = parse(&["tileserver-rs", "-c", "config.toml"]);
        assert_eq!(cli.config.unwrap(), PathBuf::from("config.toml"));
    }

    #[test]
    fn test_cli_config_long() {
        let cli = parse(&["tileserver-rs", "--config", "/etc/ts.toml"]);
        assert_eq!(cli.config.unwrap(), PathBuf::from("/etc/ts.toml"));
    }

    #[test]
    fn test_cli_port_short() {
        let cli = parse(&["tileserver-rs", "-p", "3000"]);
        assert_eq!(cli.port, Some(3000));
    }

    #[test]
    fn test_cli_port_long() {
        let cli = parse(&["tileserver-rs", "--port", "9090"]);
        assert_eq!(cli.port, Some(9090));
    }

    #[test]
    fn test_cli_host_long() {
        let cli = parse(&["tileserver-rs", "--host", "127.0.0.1"]);
        assert_eq!(cli.host.as_deref(), Some("127.0.0.1"));
    }

    #[test]
    fn test_cli_public_url() {
        let cli = parse(&["tileserver-rs", "--public-url", "https://tiles.example.com"]);
        assert_eq!(cli.public_url.as_deref(), Some("https://tiles.example.com"));
    }

    #[test]
    fn test_cli_verbose() {
        let cli = parse(&["tileserver-rs", "-v"]);
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_no_ui_flag() {
        let cli = parse(&["tileserver-rs", "--no-ui"]);
        assert!(cli.no_ui);
    }

    #[test]
    fn test_cli_ui_enabled_default() {
        let cli = parse(&["tileserver-rs"]);
        assert!(cli.ui_enabled());
    }

    #[test]
    fn test_cli_ui_disabled_via_no_ui() {
        let cli = parse(&["tileserver-rs", "--no-ui"]);
        assert!(!cli.ui_enabled());
    }

    #[test]
    fn test_cli_combined_args() {
        let cli = parse(&[
            "tileserver-rs",
            "--host",
            "0.0.0.0",
            "--port",
            "8080",
            "--config",
            "dev.toml",
            "-v",
            "/data",
        ]);
        assert_eq!(cli.host.as_deref(), Some("0.0.0.0"));
        assert_eq!(cli.port, Some(8080));
        assert_eq!(cli.config.unwrap(), PathBuf::from("dev.toml"));
        assert!(cli.verbose);
        assert_eq!(cli.path.unwrap(), PathBuf::from("/data"));
    }

    #[test]
    fn test_cli_invalid_port_rejected() {
        let result = Cli::try_parse_from(["tileserver-rs", "--port", "not-a-number"]);
        assert!(result.is_err());
    }
}
