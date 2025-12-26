use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration for the tileserver
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub telemetry: TelemetryConfig,
    #[serde(default)]
    pub sources: Vec<SourceConfig>,
    #[serde(default)]
    pub styles: Vec<StyleConfig>,
    /// Path to fonts directory containing PBF glyph files
    #[serde(default)]
    pub fonts: Option<PathBuf>,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub cors_origins: Vec<String>,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            cors_origins: vec!["*".to_string()],
        }
    }
}

/// OpenTelemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Enable OpenTelemetry tracing
    #[serde(default)]
    pub enabled: bool,
    /// OTLP endpoint (e.g., "http://localhost:4317")
    #[serde(default = "default_otlp_endpoint")]
    pub endpoint: String,
    /// Service name for traces
    #[serde(default = "default_service_name")]
    pub service_name: String,
    /// Sampling rate (0.0 to 1.0, where 1.0 = 100% of traces)
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
}

fn default_otlp_endpoint() -> String {
    "http://localhost:4317".to_string()
}

fn default_service_name() -> String {
    "tileserver-rs".to_string()
}

fn default_sample_rate() -> f64 {
    1.0
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: default_otlp_endpoint(),
            service_name: default_service_name(),
            sample_rate: default_sample_rate(),
        }
    }
}

/// Configuration for a tile source (PMTiles or MBTiles)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Unique identifier for this source
    pub id: String,
    /// Type of source: "pmtiles" or "mbtiles"
    #[serde(rename = "type")]
    pub source_type: SourceType,
    /// Path to the file (local path, HTTP URL, or S3 URL)
    pub path: String,
    /// Optional display name
    pub name: Option<String>,
    /// Optional attribution text
    pub attribution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    PMTiles,
    MBTiles,
}

/// Configuration for a map style
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleConfig {
    /// Unique identifier for this style
    pub id: String,
    /// Path to the style.json file
    pub path: PathBuf,
    /// Optional display name
    pub name: Option<String>,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from environment or file
    pub fn load(config_path: Option<PathBuf>) -> anyhow::Result<Self> {
        // Try loading from provided path
        if let Some(path) = config_path {
            if path.exists() {
                return Self::from_file(&path);
            }
        }

        // Try loading from default locations
        let default_paths = vec![
            PathBuf::from("config.toml"),
            PathBuf::from("/etc/tileserver-rs/config.toml"),
        ];

        for path in default_paths {
            if path.exists() {
                return Self::from_file(&path);
            }
        }

        // Return default config if no file found
        Ok(Config::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
            [server]
            host = "127.0.0.1"
            port = 3000

            [[sources]]
            id = "osm"
            type = "pmtiles"
            path = "/data/osm.pmtiles"
            name = "OpenStreetMap"

            [[styles]]
            id = "bright"
            path = "/data/styles/bright/style.json"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].id, "osm");
        assert_eq!(config.sources[0].source_type, SourceType::PMTiles);
    }
}
