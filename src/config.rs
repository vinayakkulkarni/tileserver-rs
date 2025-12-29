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
    /// Path to static files directory for /files/{filename} endpoint
    #[serde(default)]
    pub files: Option<PathBuf>,
    /// PostgreSQL configuration (optional, requires `postgres` feature)
    #[serde(default)]
    #[cfg(feature = "postgres")]
    pub postgres: Option<PostgresConfig>,
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
    #[cfg(feature = "postgres")]
    Postgres,
}

/// PostgreSQL connection configuration
#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Database connection string (e.g., "postgresql://user:pass@host:5432/db")
    pub connection_string: String,
    /// Maximum number of connections in the pool (default: 20)
    #[serde(default = "default_pool_size")]
    pub pool_size: usize,
    /// Timeout waiting for a connection from the pool in milliseconds (default: 30000)
    #[serde(default = "default_pool_wait_timeout_ms")]
    pub pool_wait_timeout_ms: u64,
    /// Timeout for creating a new connection in milliseconds (default: 10000)
    #[serde(default = "default_pool_create_timeout_ms")]
    pub pool_create_timeout_ms: u64,
    /// Timeout for recycling a connection in milliseconds (default: 5000)
    #[serde(default = "default_pool_recycle_timeout_ms")]
    pub pool_recycle_timeout_ms: u64,
    /// SSL certificate file path (optional, same as PGSSLCERT)
    pub ssl_cert: Option<PathBuf>,
    /// SSL key file path (optional, same as PGSSLKEY)
    pub ssl_key: Option<PathBuf>,
    /// SSL root certificate file path (optional, same as PGSSLROOTCERT)
    pub ssl_root_cert: Option<PathBuf>,
    /// Function sources to publish
    #[serde(default)]
    pub functions: Vec<PostgresFunctionConfig>,
    /// Table sources to publish (generates optimized SQL with spatial filtering)
    #[serde(default)]
    pub tables: Vec<PostgresTableConfig>,
    /// Tile cache configuration (optional, disabled by default)
    #[serde(default)]
    pub cache: Option<PostgresCacheConfig>,
}

/// Tile cache configuration for PostgreSQL sources
#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresCacheConfig {
    /// Maximum cache size in megabytes (default: 256)
    #[serde(default = "default_cache_size_mb")]
    pub size_mb: u64,
    /// Time-to-live for cache entries in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_cache_ttl_seconds")]
    pub ttl_seconds: u64,
}

#[cfg(feature = "postgres")]
fn default_cache_size_mb() -> u64 {
    256
}

#[cfg(feature = "postgres")]
fn default_cache_ttl_seconds() -> u64 {
    3600
}

#[cfg(feature = "postgres")]
fn default_pool_size() -> usize {
    20
}

#[cfg(feature = "postgres")]
fn default_pool_wait_timeout_ms() -> u64 {
    30000
}

#[cfg(feature = "postgres")]
fn default_pool_create_timeout_ms() -> u64 {
    10000
}

#[cfg(feature = "postgres")]
fn default_pool_recycle_timeout_ms() -> u64 {
    5000
}

/// PostgreSQL function source configuration
#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresFunctionConfig {
    /// Unique identifier for this source
    pub id: String,
    /// Schema name (default: public)
    #[serde(default = "default_schema")]
    pub schema: String,
    /// Function name (required)
    pub function: String,
    /// Optional display name
    pub name: Option<String>,
    /// Optional attribution text
    pub attribution: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Minimum zoom level (default: 0)
    #[serde(default)]
    pub minzoom: u8,
    /// Maximum zoom level (default: 22)
    #[serde(default = "default_maxzoom")]
    pub maxzoom: u8,
    /// Bounds [west, south, east, north] in WGS84
    pub bounds: Option<[f64; 4]>,
}

#[cfg(feature = "postgres")]
fn default_schema() -> String {
    "public".to_string()
}

#[cfg(feature = "postgres")]
fn default_maxzoom() -> u8 {
    22
}

#[cfg(feature = "postgres")]
fn default_extent() -> u32 {
    4096
}

#[cfg(feature = "postgres")]
fn default_buffer() -> u32 {
    64
}

/// PostgreSQL table source configuration
#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresTableConfig {
    /// Unique identifier for this source
    pub id: String,
    /// Schema name (default: public)
    #[serde(default = "default_schema")]
    pub schema: String,
    /// Table name (required)
    pub table: String,
    /// Geometry column name (default: auto-detect)
    pub geometry_column: Option<String>,
    /// ID column name for feature IDs (optional)
    pub id_column: Option<String>,
    /// Columns to include in tile properties (default: all non-geometry columns)
    pub properties: Option<Vec<String>>,
    /// Optional display name
    pub name: Option<String>,
    /// Optional attribution text
    pub attribution: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Minimum zoom level (default: 0)
    #[serde(default)]
    pub minzoom: u8,
    /// Maximum zoom level (default: 22)
    #[serde(default = "default_maxzoom")]
    pub maxzoom: u8,
    /// Bounds [west, south, east, north] in WGS84 (default: auto-detect from data)
    pub bounds: Option<[f64; 4]>,
    /// MVT extent (default: 4096)
    #[serde(default = "default_extent")]
    pub extent: u32,
    /// Buffer around tiles in pixels (default: 64)
    #[serde(default = "default_buffer")]
    pub buffer: u32,
    /// Maximum features per tile (default: unlimited)
    pub max_features: Option<u32>,
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

    #[test]
    fn test_source_type_serialization() {
        assert_eq!(
            serde_json::to_string(&SourceType::PMTiles).unwrap(),
            "\"pmtiles\""
        );
        assert_eq!(
            serde_json::to_string(&SourceType::MBTiles).unwrap(),
            "\"mbtiles\""
        );
    }

    #[cfg(feature = "postgres")]
    mod postgres_tests {
        use super::*;

        #[test]
        fn test_parse_postgres_config() {
            let toml = r#"
                [server]
                host = "127.0.0.1"
                port = 3000

                [postgres]
                connection_string = "postgresql://user:pass@localhost:5432/mydb"
                pool_size = 10

                [[postgres.functions]]
                id = "my_tiles"
                schema = "public"
                function = "tile_function"
                minzoom = 0
                maxzoom = 14
                bounds = [-180.0, -85.0, 180.0, 85.0]

                [[postgres.functions]]
                id = "other_tiles"
                function = "other_function"
                name = "Other Tiles"
                attribution = "© My Company"
            "#;

            let config: Config = toml::from_str(toml).unwrap();

            let pg = config.postgres.expect("postgres config should be present");
            assert_eq!(
                pg.connection_string,
                "postgresql://user:pass@localhost:5432/mydb"
            );
            assert_eq!(pg.pool_size, 10);
            assert_eq!(pg.functions.len(), 2);

            // First function
            let func1 = &pg.functions[0];
            assert_eq!(func1.id, "my_tiles");
            assert_eq!(func1.schema, "public");
            assert_eq!(func1.function, "tile_function");
            assert_eq!(func1.minzoom, 0);
            assert_eq!(func1.maxzoom, 14);
            assert!(func1.bounds.is_some());
            assert_eq!(func1.bounds.unwrap(), [-180.0, -85.0, 180.0, 85.0]);

            // Second function with defaults
            let func2 = &pg.functions[1];
            assert_eq!(func2.id, "other_tiles");
            assert_eq!(func2.schema, "public"); // default
            assert_eq!(func2.function, "other_function");
            assert_eq!(func2.name, Some("Other Tiles".to_string()));
            assert_eq!(func2.attribution, Some("© My Company".to_string()));
            assert_eq!(func2.minzoom, 0); // default
            assert_eq!(func2.maxzoom, 22); // default
            assert!(func2.bounds.is_none());
        }

        #[test]
        fn test_postgres_config_defaults() {
            let toml = r#"
                [postgres]
                connection_string = "postgresql://localhost/db"

                [[postgres.functions]]
                id = "tiles"
                function = "get_tiles"
            "#;

            let config: Config = toml::from_str(toml).unwrap();
            let pg = config.postgres.unwrap();

            assert_eq!(pg.pool_size, 20); // default
            assert!(pg.ssl_cert.is_none());
            assert!(pg.ssl_key.is_none());
            assert!(pg.ssl_root_cert.is_none());

            let func = &pg.functions[0];
            assert_eq!(func.schema, "public"); // default
            assert_eq!(func.minzoom, 0); // default
            assert_eq!(func.maxzoom, 22); // default
        }

        #[test]
        fn test_postgres_function_config_serialization() {
            let func = PostgresFunctionConfig {
                id: "test".to_string(),
                schema: "myschema".to_string(),
                function: "myfunc".to_string(),
                name: Some("Test Function".to_string()),
                attribution: None,
                description: Some("A test function".to_string()),
                minzoom: 0,
                maxzoom: 16,
                bounds: Some([-10.0, -10.0, 10.0, 10.0]),
            };

            let json = serde_json::to_string(&func).unwrap();
            let parsed: PostgresFunctionConfig = serde_json::from_str(&json).unwrap();

            assert_eq!(parsed.id, "test");
            assert_eq!(parsed.schema, "myschema");
            assert_eq!(parsed.function, "myfunc");
            assert_eq!(parsed.name, Some("Test Function".to_string()));
            assert_eq!(parsed.maxzoom, 16);
        }

        #[test]
        fn test_source_type_postgres() {
            assert_eq!(
                serde_json::to_string(&SourceType::Postgres).unwrap(),
                "\"postgres\""
            );

            let parsed: SourceType = serde_json::from_str("\"postgres\"").unwrap();
            assert_eq!(parsed, SourceType::Postgres);
        }

        #[test]
        fn test_parse_postgres_table_config() {
            let toml = r#"
                [postgres]
                connection_string = "postgresql://user:pass@localhost:5432/mydb"

                [[postgres.tables]]
                id = "points"
                table = "my_points"
                geometry_column = "geom"
                id_column = "id"
                properties = ["name", "category"]
                minzoom = 0
                maxzoom = 14
                extent = 4096
                buffer = 64
                max_features = 10000

                [[postgres.tables]]
                id = "polygons"
                schema = "public"
                table = "my_polygons"
            "#;

            let config: Config = toml::from_str(toml).unwrap();
            let pg = config.postgres.expect("postgres config should be present");
            assert_eq!(pg.tables.len(), 2);

            let table1 = &pg.tables[0];
            assert_eq!(table1.id, "points");
            assert_eq!(table1.table, "my_points");
            assert_eq!(table1.geometry_column, Some("geom".to_string()));
            assert_eq!(table1.id_column, Some("id".to_string()));
            assert_eq!(
                table1.properties,
                Some(vec!["name".to_string(), "category".to_string()])
            );
            assert_eq!(table1.extent, 4096);
            assert_eq!(table1.buffer, 64);
            assert_eq!(table1.max_features, Some(10000));

            let table2 = &pg.tables[1];
            assert_eq!(table2.id, "polygons");
            assert_eq!(table2.schema, "public");
            assert_eq!(table2.table, "my_polygons");
            assert_eq!(table2.extent, 4096);
            assert_eq!(table2.buffer, 64);
            assert!(table2.geometry_column.is_none());
            assert!(table2.max_features.is_none());
        }
    }
}
