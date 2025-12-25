use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "tileserver-rs")]
#[command(author, version, about = "A high-performance tile server for PMTiles and MBTiles", long_about = None)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE", env = "TILESERVER_CONFIG")]
    pub config: Option<PathBuf>,

    /// Host to bind to
    #[arg(long, env = "TILESERVER_HOST")]
    pub host: Option<String>,

    /// Port to bind to
    #[arg(short, long, env = "TILESERVER_PORT")]
    pub port: Option<u16>,

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

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Returns whether the UI should be enabled
    pub fn ui_enabled(&self) -> bool {
        !self.no_ui && self.ui
    }
}
