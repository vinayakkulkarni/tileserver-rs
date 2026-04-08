//! Cloud object storage PMTiles source (S3/Azure Blob/GCS) via `object_store` crate.

use async_trait::async_trait;
use pmtiles::{
    AsyncPmTilesReader, Compression as PmCompression, HashMapCache, ObjectStoreBackend, TileCoord,
    TileType,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::SourceConfig;
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

type CloudReader = AsyncPmTilesReader<ObjectStoreBackend, HashMapCache>;

pub struct CloudPmTilesSource {
    reader: Arc<RwLock<CloudReader>>,
    metadata: TileMetadata,
    tile_compression: TileCompression,
    native_format: TileFormat,
}

/// URL schemes recognized as cloud object storage paths.
const CLOUD_SCHEMES: &[&str] = &["s3://", "s3a://", "az://", "gs://"];

/// Returns `true` if the path starts with a cloud object storage URL scheme.
#[must_use]
pub fn is_cloud_url(path: &str) -> bool {
    CLOUD_SCHEMES.iter().any(|s| path.starts_with(s))
}

impl CloudPmTilesSource {
    pub async fn from_url(config: &SourceConfig) -> Result<Self> {
        let url_str = &config.path;

        tracing::info!("Opening cloud PMTiles source: {}", url_str);

        let url: url::Url = url_str.parse().map_err(|e| {
            TileServerError::ConfigError(format!("invalid cloud storage URL '{}': {}", url_str, e))
        })?;

        let opts: HashMap<String, String> = config.options.clone().unwrap_or_default();

        let (store, path) = object_store::parse_url_opts(&url, &opts).map_err(|e| {
            TileServerError::ConfigError(format!(
                "failed to configure object store for '{}': {}",
                url_str, e
            ))
        })?;

        let cache = HashMapCache::default();
        let backend = ObjectStoreBackend::new(store, path);

        let reader: CloudReader = AsyncPmTilesReader::try_from_cached_source(backend, cache)
            .await
            .map_err(|e| {
                TileServerError::MetadataError(format!(
                    "failed to read PMTiles header from '{}': {}",
                    url_str, e
                ))
            })?;

        let header = reader.get_header();

        let mut format = match header.tile_type {
            TileType::Mvt => TileFormat::Pbf,
            TileType::Png => TileFormat::Png,
            TileType::Jpeg => TileFormat::Jpeg,
            TileType::Webp => TileFormat::Webp,
            TileType::Avif => TileFormat::Avif,
            TileType::Mlt => TileFormat::Mlt,
            TileType::Unknown => TileFormat::Unknown,
        };

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

        let tile_compression = convert_compression(header.tile_compression);

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
            "Loaded cloud PMTiles source '{}': zoom {}-{}, format {:?}",
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
impl TileSource for CloudPmTilesSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        let max_tile = 1u32 << z;
        if x >= max_tile || y >= max_tile {
            return Err(TileServerError::InvalidCoordinates { z, x, y });
        }

        if z < self.metadata.minzoom || z > self.metadata.maxzoom {
            return Ok(None);
        }

        let coord = match TileCoord::new(z, x, y) {
            Ok(c) => c,
            Err(_) => return Err(TileServerError::InvalidCoordinates { z, x, y }),
        };

        let reader = self.reader.read().await;

        match reader.get_tile(coord).await {
            Ok(Some(tile_data)) => Ok(Some(TileData {
                data: tile_data,
                format: self.native_format,
                compression: self.tile_compression,
            })),
            Ok(None) => Ok(None),
            Err(e) => {
                tracing::warn!("Error reading cloud tile z={} x={} y={}: {}", z, x, y, e);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cloud_url_s3() {
        assert!(is_cloud_url("s3://bucket/path/tiles.pmtiles"));
        assert!(is_cloud_url("s3a://bucket/path/tiles.pmtiles"));
    }

    #[test]
    fn test_is_cloud_url_azure() {
        assert!(is_cloud_url("az://container/tiles.pmtiles"));
    }

    #[test]
    fn test_is_cloud_url_gcs() {
        assert!(is_cloud_url("gs://bucket/tiles.pmtiles"));
    }

    #[test]
    fn test_is_cloud_url_not_cloud() {
        assert!(!is_cloud_url("/local/path/tiles.pmtiles"));
        assert!(!is_cloud_url("https://example.com/tiles.pmtiles"));
        assert!(!is_cloud_url("http://example.com/tiles.pmtiles"));
        assert!(!is_cloud_url("./relative/path.pmtiles"));
    }
}
