//! MBTiles tile source backed by SQLite via `rusqlite`.

use async_trait::async_trait;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::config::SourceConfig;
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

/// MBTiles tile source
///
/// Implements the MBTiles 1.3 specification for serving tiles from SQLite databases.
/// See: https://github.com/mapbox/mbtiles-spec/blob/master/1.3/spec.md
pub struct MbTilesSource {
    /// SQLite connection (wrapped in Arc<Mutex> for thread-safety)
    conn: Arc<Mutex<Connection>>,
    /// Cached metadata
    metadata: TileMetadata,
    /// The native format of the underlying tile data (before any `serve_as` override).
    native_format: TileFormat,
}

impl MbTilesSource {
    /// Create a new MBTiles source from a local file
    pub async fn from_file(config: &SourceConfig) -> Result<Self> {
        let path = Path::new(&config.path);

        // Check if file exists
        if !path.exists() {
            return Err(TileServerError::FileError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("MBTiles file not found: {}", config.path),
            )));
        }

        // Open SQLite connection in read-only mode
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| TileServerError::MbTilesError(e.to_string()))?;

        // Read metadata from the database
        let mut metadata = Self::read_metadata(&conn, config)?;

        // Apply `serve_as` override: the metadata format controls TileJSON URLs
        // and encoding, while native_format tracks the actual on-disk format.
        let native_format = metadata.format;
        if let Some(target_format) = config.serve_as {
            metadata.format = target_format;
            tracing::info!(
                "Source '{}': native format {:?}, serving as {:?} (serve_as override)",
                config.id,
                native_format,
                target_format
            );
        }

        // Use config description if provided, otherwise fall back to database metadata
        if config.description.is_some() {
            metadata.description = config.description.clone();
        }

        tracing::info!(
            "Loaded MBTiles source '{}': {} (zoom {}-{})",
            config.id,
            metadata.name,
            metadata.minzoom,
            metadata.maxzoom
        );

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            metadata,
            native_format,
        })
    }
    /// Read metadata from the MBTiles metadata table
    fn read_metadata(conn: &Connection, config: &SourceConfig) -> Result<TileMetadata> {
        let mut stmt = conn
            .prepare("SELECT name, value FROM metadata")
            .map_err(|e| TileServerError::MbTilesError(e.to_string()))?;

        let mut name = config.name.clone().unwrap_or_else(|| config.id.clone());
        let mut description = None;
        let mut attribution = config.attribution.clone();
        let mut format = TileFormat::Pbf;
        let mut minzoom: u8 = 0;
        let mut maxzoom: u8 = 22;
        let mut bounds: Option<[f64; 4]> = None;
        let mut center: Option<[f64; 3]> = None;
        let mut vector_layers: Option<serde_json::Value> = None;

        let rows = stmt
            .query_map([], |row| {
                let key: String = row.get(0)?;
                let value: String = row.get(1)?;
                Ok((key, value))
            })
            .map_err(|e| TileServerError::MbTilesError(e.to_string()))?;

        for row in rows {
            let (key, value) = row.map_err(|e| TileServerError::MbTilesError(e.to_string()))?;

            match key.as_str() {
                "name" => name = value,
                "description" => description = Some(value),
                "attribution" => {
                    if attribution.is_none() {
                        attribution = Some(value);
                    }
                }
                "format" => {
                    format = match value.to_lowercase().as_str() {
                        "pbf" => TileFormat::Pbf,
                        "png" => TileFormat::Png,
                        "jpg" | "jpeg" => TileFormat::Jpeg,
                        "webp" => TileFormat::Webp,
                        "avif" => TileFormat::Avif,
                        "mlt" | "application/vnd.maplibre-vector-tile" => TileFormat::Mlt,
                        _ => TileFormat::Pbf,
                    };
                }
                "minzoom" => {
                    minzoom = value.parse().unwrap_or(0);
                }
                "maxzoom" => {
                    maxzoom = value.parse().unwrap_or(22);
                }
                "bounds" => {
                    let parts: Vec<f64> = value
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                    if parts.len() == 4 {
                        bounds = Some([parts[0], parts[1], parts[2], parts[3]]);
                    }
                }
                "center" => {
                    let parts: Vec<f64> = value
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                    if parts.len() >= 2 {
                        let zoom = if parts.len() >= 3 {
                            parts[2]
                        } else {
                            ((minzoom as f64 + maxzoom as f64) / 2.0).floor()
                        };
                        center = Some([parts[0], parts[1], zoom]);
                    }
                }
                "json" => {
                    // Parse vector_layers from the JSON metadata
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&value)
                        && let Some(layers) = json.get("vector_layers")
                    {
                        vector_layers = Some(layers.clone());
                    }
                }
                _ => {}
            }
        }

        // If no center specified, calculate from bounds
        if center.is_none()
            && let Some(b) = bounds
        {
            let center_lon = (b[0] + b[2]) / 2.0;
            let center_lat = (b[1] + b[3]) / 2.0;
            let center_zoom = ((minzoom as f64 + maxzoom as f64) / 2.0).floor();
            center = Some([center_lon, center_lat, center_zoom]);
        }

        Ok(TileMetadata {
            id: config.id.clone(),
            name,
            description,
            attribution,
            format,
            minzoom,
            maxzoom,
            bounds,
            center,
            vector_layers,
        })
    }
    /// Flip Y coordinate for TMS scheme (MBTiles uses TMS, most clients use XYZ)
    fn flip_y(z: u8, y: u32) -> u32 {
        (1u32 << z) - 1 - y
    }
}

#[async_trait]
impl TileSource for MbTilesSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        // Validate coordinates
        let max_tile = 1u32 << z;
        if x >= max_tile || y >= max_tile {
            return Err(TileServerError::InvalidCoordinates { z, x, y });
        }

        // Check zoom bounds
        if z < self.metadata.minzoom || z > self.metadata.maxzoom {
            return Ok(None);
        }

        // MBTiles uses TMS scheme (Y is flipped)
        let tms_y = Self::flip_y(z, y);

        // Clone the connection Arc for use in the blocking task
        let conn = self.conn.clone();
        let format = self.native_format;

        // Run the SQLite query in a blocking task to avoid blocking the async runtime
        let result = tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| {
                TileServerError::MbTilesError(format!("Failed to acquire connection lock: {}", e))
            })?;

            let mut stmt = conn
                .prepare_cached("SELECT tile_data FROM tiles WHERE zoom_level = ?1 AND tile_column = ?2 AND tile_row = ?3")
                .map_err(|e| TileServerError::MbTilesError(e.to_string()))?;

            let tile_data: Option<Vec<u8>> = stmt
                .query_row([z as i32, x as i32, tms_y as i32], |row| row.get(0))
                .ok();

            Ok::<_, TileServerError>(tile_data.map(|data| {
                // Detect gzip compression by checking magic bytes
                let compression = if data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b {
                    TileCompression::Gzip
                } else {
                    TileCompression::None
                };

                TileData {
                    data: data.into(),
                    format,
                    compression,
                }
            }))
        })
        .await
        .map_err(|e| TileServerError::MbTilesError(format!("Task join error: {}", e)))??;

        Ok(result)
    }

    fn metadata(&self) -> &TileMetadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flip_y() {
        // At zoom 0, there's only one tile (0, 0)
        assert_eq!(MbTilesSource::flip_y(0, 0), 0);

        // At zoom 1, y=0 -> tms_y=1, y=1 -> tms_y=0
        assert_eq!(MbTilesSource::flip_y(1, 0), 1);
        assert_eq!(MbTilesSource::flip_y(1, 1), 0);

        // At zoom 2, there are 4 rows (0-3)
        assert_eq!(MbTilesSource::flip_y(2, 0), 3);
        assert_eq!(MbTilesSource::flip_y(2, 1), 2);
        assert_eq!(MbTilesSource::flip_y(2, 2), 1);
        assert_eq!(MbTilesSource::flip_y(2, 3), 0);
    }

    #[test]
    fn test_flip_y_high_zoom() {
        assert_eq!(MbTilesSource::flip_y(10, 0), 1023);
        assert_eq!(MbTilesSource::flip_y(10, 1023), 0);
    }

    fn create_in_memory_mbtiles() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE metadata (name TEXT, value TEXT);
             CREATE TABLE tiles (zoom_level INTEGER, tile_column INTEGER, tile_row INTEGER, tile_data BLOB);",
        ).unwrap();
        conn
    }

    #[test]
    fn test_read_metadata_format_pbf() {
        let conn = create_in_memory_mbtiles();
        conn.execute("INSERT INTO metadata VALUES ('format', 'pbf')", [])
            .unwrap();
        conn.execute("INSERT INTO metadata VALUES ('name', 'Test')", [])
            .unwrap();

        let config = SourceConfig {
            id: "test".to_string(),
            source_type: crate::config::SourceType::MBTiles,
            path: "memory".to_string(),
            name: None,
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
        };
        let meta = MbTilesSource::read_metadata(&conn, &config).unwrap();
        assert_eq!(meta.format, TileFormat::Pbf);
        assert_eq!(meta.name, "Test");
    }

    #[test]
    fn test_read_metadata_format_png() {
        let conn = create_in_memory_mbtiles();
        conn.execute("INSERT INTO metadata VALUES ('format', 'png')", [])
            .unwrap();

        let config = SourceConfig {
            id: "raster".to_string(),
            source_type: crate::config::SourceType::MBTiles,
            path: "memory".to_string(),
            name: None,
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
        };
        let meta = MbTilesSource::read_metadata(&conn, &config).unwrap();
        assert_eq!(meta.format, TileFormat::Png);
    }

    #[test]
    fn test_read_metadata_format_jpeg() {
        let conn = create_in_memory_mbtiles();
        conn.execute("INSERT INTO metadata VALUES ('format', 'jpg')", [])
            .unwrap();

        let config = SourceConfig {
            id: "jpg".to_string(),
            source_type: crate::config::SourceType::MBTiles,
            path: "memory".to_string(),
            name: None,
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
        };
        let meta = MbTilesSource::read_metadata(&conn, &config).unwrap();
        assert_eq!(meta.format, TileFormat::Jpeg);
    }

    #[test]
    fn test_read_metadata_format_mlt() {
        let conn = create_in_memory_mbtiles();
        conn.execute("INSERT INTO metadata VALUES ('format', 'mlt')", [])
            .unwrap();

        let config = SourceConfig {
            id: "mlt".to_string(),
            source_type: crate::config::SourceType::MBTiles,
            path: "memory".to_string(),
            name: None,
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
        };
        let meta = MbTilesSource::read_metadata(&conn, &config).unwrap();
        assert_eq!(meta.format, TileFormat::Mlt);
    }

    #[test]
    fn test_read_metadata_zoom_levels() {
        let conn = create_in_memory_mbtiles();
        conn.execute("INSERT INTO metadata VALUES ('minzoom', '5')", [])
            .unwrap();
        conn.execute("INSERT INTO metadata VALUES ('maxzoom', '14')", [])
            .unwrap();

        let config = SourceConfig {
            id: "zoom".to_string(),
            source_type: crate::config::SourceType::MBTiles,
            path: "memory".to_string(),
            name: None,
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
        };
        let meta = MbTilesSource::read_metadata(&conn, &config).unwrap();
        assert_eq!(meta.minzoom, 5);
        assert_eq!(meta.maxzoom, 14);
    }

    #[test]
    fn test_read_metadata_bounds() {
        let conn = create_in_memory_mbtiles();
        conn.execute(
            "INSERT INTO metadata VALUES ('bounds', '-180,-85,180,85')",
            [],
        )
        .unwrap();

        let config = SourceConfig {
            id: "bounds".to_string(),
            source_type: crate::config::SourceType::MBTiles,
            path: "memory".to_string(),
            name: None,
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
        };
        let meta = MbTilesSource::read_metadata(&conn, &config).unwrap();
        let b = meta.bounds.unwrap();
        assert!((b[0] - (-180.0)).abs() < 0.01);
        assert!((b[1] - (-85.0)).abs() < 0.01);
        assert!((b[2] - 180.0).abs() < 0.01);
        assert!((b[3] - 85.0).abs() < 0.01);
    }

    #[test]
    fn test_read_metadata_center_from_bounds() {
        let conn = create_in_memory_mbtiles();
        conn.execute(
            "INSERT INTO metadata VALUES ('bounds', '-10,-10,10,10')",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO metadata VALUES ('minzoom', '0')", [])
            .unwrap();
        conn.execute("INSERT INTO metadata VALUES ('maxzoom', '10')", [])
            .unwrap();

        let config = SourceConfig {
            id: "autocenter".to_string(),
            source_type: crate::config::SourceType::MBTiles,
            path: "memory".to_string(),
            name: None,
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
        };
        let meta = MbTilesSource::read_metadata(&conn, &config).unwrap();
        let c = meta.center.unwrap();
        assert!((c[0] - 0.0).abs() < 0.01);
        assert!((c[1] - 0.0).abs() < 0.01);
        assert!((c[2] - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_read_metadata_explicit_center() {
        let conn = create_in_memory_mbtiles();
        conn.execute("INSERT INTO metadata VALUES ('center', '8.5,47.3,12')", [])
            .unwrap();

        let config = SourceConfig {
            id: "center".to_string(),
            source_type: crate::config::SourceType::MBTiles,
            path: "memory".to_string(),
            name: None,
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
        };
        let meta = MbTilesSource::read_metadata(&conn, &config).unwrap();
        let c = meta.center.unwrap();
        assert!((c[0] - 8.5).abs() < 0.01);
        assert!((c[1] - 47.3).abs() < 0.01);
        assert!((c[2] - 12.0).abs() < 0.01);
    }

    #[test]
    fn test_read_metadata_db_name_wins_over_config_name() {
        let conn = create_in_memory_mbtiles();
        conn.execute("INSERT INTO metadata VALUES ('name', 'DB Name')", [])
            .unwrap();

        let config = SourceConfig {
            id: "override".to_string(),
            source_type: crate::config::SourceType::MBTiles,
            path: "memory".to_string(),
            name: Some("Config Name".to_string()),
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
        };
        let meta = MbTilesSource::read_metadata(&conn, &config).unwrap();
        assert_eq!(meta.name, "DB Name");
    }

    #[test]
    fn test_read_metadata_attribution_from_config() {
        let conn = create_in_memory_mbtiles();
        conn.execute(
            "INSERT INTO metadata VALUES ('attribution', 'DB Attribution')",
            [],
        )
        .unwrap();

        let config = SourceConfig {
            id: "attr".to_string(),
            source_type: crate::config::SourceType::MBTiles,
            path: "memory".to_string(),
            name: None,
            attribution: Some("Config Attribution".to_string()),
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
        };
        let meta = MbTilesSource::read_metadata(&conn, &config).unwrap();
        assert_eq!(meta.attribution.as_deref(), Some("Config Attribution"));
    }

    #[test]
    fn test_read_metadata_defaults_when_empty() {
        let conn = create_in_memory_mbtiles();

        let config = SourceConfig {
            id: "empty".to_string(),
            source_type: crate::config::SourceType::MBTiles,
            path: "memory".to_string(),
            name: None,
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
        };
        let meta = MbTilesSource::read_metadata(&conn, &config).unwrap();
        assert_eq!(meta.name, "empty");
        assert_eq!(meta.format, TileFormat::Pbf);
        assert_eq!(meta.minzoom, 0);
        assert_eq!(meta.maxzoom, 22);
        assert!(meta.bounds.is_none());
        assert!(meta.center.is_none());
    }

    #[test]
    fn test_gzip_detection_in_tile_data() {
        let gzip_bytes: Vec<u8> = vec![0x1f, 0x8b, 0x08, 0x00, 0x00];
        let compression = if gzip_bytes.len() >= 2 && gzip_bytes[0] == 0x1f && gzip_bytes[1] == 0x8b
        {
            TileCompression::Gzip
        } else {
            TileCompression::None
        };
        assert_eq!(compression, TileCompression::Gzip);
    }

    #[test]
    fn test_non_gzip_detection_in_tile_data() {
        let plain_bytes: Vec<u8> = vec![0x00, 0x01, 0x02, 0x03];
        let compression =
            if plain_bytes.len() >= 2 && plain_bytes[0] == 0x1f && plain_bytes[1] == 0x8b {
                TileCompression::Gzip
            } else {
                TileCompression::None
            };
        assert_eq!(compression, TileCompression::None);
    }
}
