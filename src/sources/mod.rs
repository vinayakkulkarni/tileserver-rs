use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub mod manager;
pub mod mbtiles;
pub mod pmtiles;
#[cfg(feature = "postgres")]
pub mod postgres;

pub use manager::SourceManager;

/// Tile format enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TileFormat {
    Pbf,
    Png,
    Jpeg,
    Webp,
    Avif,
    Unknown,
}

impl TileFormat {
    pub fn content_type(&self) -> &'static str {
        match self {
            TileFormat::Pbf => "application/x-protobuf",
            TileFormat::Png => "image/png",
            TileFormat::Jpeg => "image/jpeg",
            TileFormat::Webp => "image/webp",
            TileFormat::Avif => "image/avif",
            TileFormat::Unknown => "application/octet-stream",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            TileFormat::Pbf => "pbf",
            TileFormat::Png => "png",
            TileFormat::Jpeg => "jpg",
            TileFormat::Webp => "webp",
            TileFormat::Avif => "avif",
            TileFormat::Unknown => "bin",
        }
    }
}

impl FromStr for TileFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "pbf" | "mvt" | "vector" => TileFormat::Pbf,
            "png" => TileFormat::Png,
            "jpg" | "jpeg" => TileFormat::Jpeg,
            "webp" => TileFormat::Webp,
            "avif" => TileFormat::Avif,
            _ => TileFormat::Unknown,
        })
    }
}

/// Tile compression enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileCompression {
    None,
    Gzip,
    Zstd,
    Brotli,
}

impl TileCompression {
    pub fn content_encoding(&self) -> Option<&'static str> {
        match self {
            TileCompression::None => None,
            TileCompression::Gzip => Some("gzip"),
            TileCompression::Zstd => Some("zstd"),
            TileCompression::Brotli => Some("br"),
        }
    }
}

/// Metadata for a tile source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileMetadata {
    /// Source identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Attribution HTML
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    /// Tile format
    pub format: TileFormat,
    /// Minimum zoom level
    pub minzoom: u8,
    /// Maximum zoom level
    pub maxzoom: u8,
    /// Bounds [west, south, east, north]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[f64; 4]>,
    /// Center [lon, lat, zoom]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub center: Option<[f64; 3]>,
    /// Vector layers (for vector tiles)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_layers: Option<serde_json::Value>,
}

/// TileJSON 3.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileJson {
    pub tilejson: String,
    /// Source identifier (used by frontend to navigate)
    pub id: String,
    pub tiles: Vec<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    pub minzoom: u8,
    pub maxzoom: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub center: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_layers: Option<serde_json::Value>,
}

impl TileMetadata {
    /// Convert to TileJSON format
    pub fn to_tilejson(&self, base_url: &str) -> TileJson {
        self.to_tilejson_with_key(base_url, None)
    }

    /// Convert to TileJSON format with optional API key
    pub fn to_tilejson_with_key(&self, base_url: &str, key: Option<&str>) -> TileJson {
        let key_query = key
            .map(|k| format!("?key={}", urlencoding::encode(k)))
            .unwrap_or_default();

        let tile_url = format!(
            "{}/data/{}/{{z}}/{{x}}/{{y}}.{}{}",
            base_url,
            self.id,
            self.format.extension(),
            key_query
        );

        TileJson {
            tilejson: "3.0.0".to_string(),
            id: self.id.clone(),
            tiles: vec![tile_url],
            name: self.name.clone(),
            description: self.description.clone(),
            attribution: self.attribution.clone(),
            minzoom: self.minzoom,
            maxzoom: self.maxzoom,
            bounds: self.bounds,
            center: self.center,
            vector_layers: self.vector_layers.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TileData {
    pub data: Bytes,
    pub format: TileFormat,
    pub compression: TileCompression,
}

/// Trait for tile sources
#[async_trait]
pub trait TileSource: Send + Sync {
    /// Get a tile at the specified coordinates
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> crate::error::Result<Option<TileData>>;

    /// Get metadata for this source
    fn metadata(&self) -> &TileMetadata;

    /// Get the tile format
    fn format(&self) -> TileFormat {
        self.metadata().format
    }
}
