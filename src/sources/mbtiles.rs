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
        let metadata = Self::read_metadata(&conn, config)?;

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
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&value) {
                        if let Some(layers) = json.get("vector_layers") {
                            vector_layers = Some(layers.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        // If no center specified, calculate from bounds
        if center.is_none() {
            if let Some(b) = bounds {
                let center_lon = (b[0] + b[2]) / 2.0;
                let center_lat = (b[1] + b[3]) / 2.0;
                let center_zoom = ((minzoom as f64 + maxzoom as f64) / 2.0).floor();
                center = Some([center_lon, center_lat, center_zoom]);
            }
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
        let format = self.metadata.format;

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
}
