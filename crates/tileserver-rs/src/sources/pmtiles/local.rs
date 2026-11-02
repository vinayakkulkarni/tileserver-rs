//! Local file PMTiles source using memory-mapped I/O.

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
    /// The native format of the underlying tile data (before any `serve_as` override).
    /// When `serve_as` is configured, `metadata.format` reflects the target format
    /// (for TileJSON URLs and encoding), while `native_format` tracks the actual
    /// on-disk format so that tiles are returned with the correct format tag
    /// for the transcoding layer to detect and convert.
    native_format: TileFormat,
}

impl LocalPmTilesSource {
    /// Create a new PMTiles source from a local file path
    ///
    /// # Errors
    ///
    /// [`TileServerError::ConfigError`] when the file is missing, and
    /// [`TileServerError::MetadataError`] when it cannot be opened or read.
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
            TileServerError::MetadataError(format!("failed to open PMTiles file: {}", e))
        })?;

        // Create async reader
        let reader: LocalReader =
            AsyncPmTilesReader::try_from_source(backend)
                .await
                .map_err(|e| {
                    TileServerError::MetadataError(format!("failed to read PMTiles header: {}", e))
                })?;

        let header = reader.get_header();

        // Determine tile format
        let mut format = match header.tile_type {
            TileType::Mvt => TileFormat::Pbf,
            TileType::Png => TileFormat::Png,
            TileType::Jpeg => TileFormat::Jpeg,
            TileType::Webp => TileFormat::Webp,
            TileType::Avif => TileFormat::Avif,
            TileType::Mlt => TileFormat::Mlt,
            TileType::Unknown => TileFormat::Unknown,
        };

        // For Unknown tile type, probe a tile to detect MLT format
        if format == TileFormat::Unknown
            && let Ok(coord) = TileCoord::new(header.min_zoom, 0, 0)
            && let Ok(Some(sample)) = reader.get_tile(coord).await
            && crate::sources::detect_mlt_format(&sample)
        {
            format = TileFormat::Mlt;
            tracing::info!(
                "Auto-detected MLT format for source '{}' via tile probe",
                config.id
            );
        }

        // Apply `serve_as` override: the metadata format controls TileJSON URLs
        // and encoding, while native_format tracks the actual on-disk format.
        let native_format = format;
        let metadata_format = config.serve_as.unwrap_or(format);
        if config.serve_as.is_some() {
            tracing::info!(
                "Source '{}': native format {:?}, serving as {:?} (serve_as override)",
                config.id,
                native_format,
                metadata_format
            );
        }

        // Store tile compression for later use
        let tile_compression = convert_compression(header.tile_compression);

        // Try to extract vector_layers from PMTiles metadata JSON
        let vector_layers = match reader.get_metadata().await {
            Ok(metadata_str) => {
                if let Ok(metadata_json) = serde_json::from_str::<serde_json::Value>(&metadata_str)
                {
                    metadata_json.get("vector_layers").cloned()
                } else {
                    None
                }
            }
            Err(_) => None,
        };

        // Extract metadata from header
        let metadata = TileMetadata {
            id: config.id.clone(),
            name: config.name.clone().unwrap_or_else(|| config.id.clone()),
            description: config.description.clone(),
            attribution: config.attribution.clone(),
            format: metadata_format,
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
            vector_layers,
        };

        tracing::info!(
            "Loaded local PMTiles source '{}': zoom {}-{}, format {:?}",
            config.id,
            header.min_zoom,
            header.max_zoom,
            metadata_format
        );

        Ok(Self {
            reader: Arc::new(RwLock::new(reader)),
            metadata,
            tile_compression,
            native_format,
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
                format: self.native_format,
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
