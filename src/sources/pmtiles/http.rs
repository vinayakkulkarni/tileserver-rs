use async_trait::async_trait;
use bytes::Bytes;
use pmtiles::{
    AsyncPmTilesReader, Compression as PmCompression, HashMapCache, HttpBackend, TileCoord,
    TileType,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::SourceConfig;
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

/// Type alias for HTTP PMTiles reader: Backend=HttpBackend, Cache=HashMapCache
type HttpReader = AsyncPmTilesReader<HttpBackend, HashMapCache>;

/// HTTP-based PMTiles tile source
pub struct HttpPmTilesSource {
    reader: Arc<RwLock<HttpReader>>,
    metadata: TileMetadata,
    tile_compression: TileCompression,
}

impl HttpPmTilesSource {
    /// Create a new PMTiles source from an HTTP URL
    pub async fn from_url(config: &SourceConfig, _client: reqwest::Client) -> Result<Self> {
        let url = &config.path;

        tracing::info!("Opening HTTP PMTiles source: {}", url);

        // Create a cache for directory entries
        let cache = HashMapCache::default();

        // Create HTTP client with rustls TLS
        let client = pmtiles::reqwest::Client::builder()
            .user_agent("tileserver-rs/0.1.0")
            .use_rustls_tls()
            .build()
            .map_err(|e| {
                TileServerError::MetadataError(format!("Failed to create HTTP client: {}", e))
            })?;

        // Create async reader with cached URL
        let reader: HttpReader =
            AsyncPmTilesReader::new_with_cached_url(cache, client, url)
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

        // Extract metadata from header (using correct field names)
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
            "Loaded HTTP PMTiles source '{}': zoom {}-{}, format {:?}",
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
impl TileSource for HttpPmTilesSource {
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

        // Create tile coordinate (TileCoord takes u32 for x and y)
        let coord = match TileCoord::new(z, x, y) {
            Ok(c) => c,
            Err(_) => return Err(TileServerError::InvalidCoordinates { z, x, y }),
        };

        let reader = self.reader.read().await;

        // Get tile from PMTiles over HTTP
        match reader.get_tile(coord).await {
            Ok(Some(tile_data)) => Ok(Some(TileData {
                data: Bytes::from(tile_data),
                format: self.metadata.format,
                compression: self.tile_compression,
            })),
            Ok(None) => Ok(None),
            Err(e) => {
                tracing::warn!("Error reading HTTP tile z={} x={} y={}: {}", z, x, y, e);
                Ok(None)
            }
        }
    }

    fn metadata(&self) -> &TileMetadata {
        &self.metadata
    }
}
