use async_trait::async_trait;
use pmtiles::{AsyncPmTilesReader, Compression as PmCompression, MmapBackend, TileCoord, TileType};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::SourceConfig;
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

/// Type alias for local PMTiles reader: Backend=MmapBackend
type LocalReader = AsyncPmTilesReader<MmapBackend>;

/// Local file-based PMTiles tile source using memory-mapped I/O
pub struct LocalPmTilesSource {
    reader: Arc<RwLock<LocalReader>>,
    metadata: TileMetadata,
    tile_compression: TileCompression,
}

impl LocalPmTilesSource {
    /// Create a new PMTiles source from a local file path
    pub async fn from_file(config: &SourceConfig) -> Result<Self> {
        let path = &config.path;

        tracing::info!("Opening local PMTiles source: {}", path);

        // Verify file exists
        if !Path::new(path).exists() {
            return Err(TileServerError::ConfigError(format!(
                "PMTiles file not found: {}",
                path
            )));
        }

        // Create memory-mapped backend
        let backend = MmapBackend::try_from(Path::new(path)).await.map_err(|e| {
            TileServerError::MetadataError(format!("Failed to open PMTiles file: {}", e))
        })?;

        // Create async reader
        let reader: LocalReader =
            AsyncPmTilesReader::try_from_source(backend)
                .await
                .map_err(|e| {
                    TileServerError::MetadataError(format!("Failed to read PMTiles header: {}", e))
                })?;

        let header = reader.get_header();

        // Determine tile format
        let format = match header.tile_type {
            TileType::Mvt => TileFormat::Pbf,
            TileType::Png => TileFormat::Png,
            TileType::Jpeg => TileFormat::Jpeg,
            TileType::Webp => TileFormat::Webp,
            TileType::Avif => TileFormat::Avif,
            TileType::Unknown => TileFormat::Unknown,
        };

        // Store tile compression for later use
        let tile_compression = convert_compression(header.tile_compression);

        // Extract metadata from header
        let metadata = TileMetadata {
            id: config.id.clone(),
            name: config.name.clone().unwrap_or_else(|| config.id.clone()),
            description: None,
            attribution: config.attribution.clone(),
            format,
            minzoom: header.min_zoom,
            maxzoom: header.max_zoom,
            bounds: Some([
                header.min_longitude,
                header.min_latitude,
                header.max_longitude,
                header.max_latitude,
            ]),
            center: Some([
                header.center_longitude,
                header.center_latitude,
                header.center_zoom as f64,
            ]),
            vector_layers: None,
        };

        tracing::info!(
            "Loaded local PMTiles source '{}': zoom {}-{}, format {:?}",
            config.id,
            header.min_zoom,
            header.max_zoom,
            format
        );

        Ok(Self {
            reader: Arc::new(RwLock::new(reader)),
            metadata,
            tile_compression,
        })
    }
}

/// Convert PMTiles compression to our compression enum
fn convert_compression(compression: PmCompression) -> TileCompression {
    match compression {
        PmCompression::None => TileCompression::None,
        PmCompression::Gzip => TileCompression::Gzip,
        PmCompression::Brotli => TileCompression::Brotli,
        PmCompression::Zstd => TileCompression::Zstd,
        PmCompression::Unknown => TileCompression::None,
    }
}

#[async_trait]
impl TileSource for LocalPmTilesSource {
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

        // Create tile coordinate
        let coord = match TileCoord::new(z, x, y) {
            Ok(c) => c,
            Err(_) => return Err(TileServerError::InvalidCoordinates { z, x, y }),
        };

        let reader = self.reader.read().await;

        // Get tile from PMTiles via memory-mapped I/O
        match reader.get_tile(coord).await {
            Ok(Some(tile_data)) => Ok(Some(TileData {
                data: tile_data,
                format: self.metadata.format,
                compression: self.tile_compression,
            })),
            Ok(None) => Ok(None),
            Err(e) => {
                tracing::warn!("Error reading local tile z={} x={} y={}: {}", z, x, y, e);
                Ok(None)
            }
        }
    }

    fn metadata(&self) -> &TileMetadata {
        &self.metadata
    }
}
