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

    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
