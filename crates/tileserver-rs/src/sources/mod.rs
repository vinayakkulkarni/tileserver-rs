use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[cfg(feature = "raster")]
pub mod cog;
#[cfg(feature = "raster")]
pub mod dataset_cache;
#[cfg(feature = "duckdb")]
pub mod duckdb;
#[cfg(feature = "geoparquet")]
pub mod geoparquet;
pub mod manager;
pub mod mbtiles;
pub mod pmtiles;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "stac")]
pub mod stac;

pub use manager::SourceManager;

/// Tile format enum
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TileFormat {
    Pbf,
    Png,
    Jpeg,
    Webp,
    Avif,
    Mlt,
    Unknown,
}

impl TileFormat {
    #[inline]
    #[must_use]
    pub fn content_type(&self) -> &'static str {
        match self {
            TileFormat::Pbf => "application/x-protobuf",
            TileFormat::Png => "image/png",
            TileFormat::Jpeg => "image/jpeg",
            TileFormat::Webp => "image/webp",
            TileFormat::Avif => "image/avif",
            TileFormat::Mlt => "application/vnd.maplibre-vector-tile",
            TileFormat::Unknown => "application/octet-stream",
        }
    }

    #[inline]
    #[must_use]
    pub fn extension(&self) -> &'static str {
        match self {
            TileFormat::Pbf => "pbf",
            TileFormat::Png => "png",
            TileFormat::Jpeg => "jpg",
            TileFormat::Webp => "webp",
            TileFormat::Avif => "avif",
            TileFormat::Mlt => "mlt",
            TileFormat::Unknown => "bin",
        }
    }

    /// Returns true if this format contains vector tile data (MVT or MLT)
    #[inline]
    #[must_use]
    pub fn is_vector(&self) -> bool {
        matches!(self, TileFormat::Pbf | TileFormat::Mlt)
    }
}

impl FromStr for TileFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pbf" | "mvt" | "vector" => Ok(TileFormat::Pbf),
            "png" => Ok(TileFormat::Png),
            "jpg" | "jpeg" => Ok(TileFormat::Jpeg),
            "webp" => Ok(TileFormat::Webp),
            "avif" => Ok(TileFormat::Avif),
            "mlt" => Ok(TileFormat::Mlt),
            _ => Err(()),
        }
    }
}

/// Tile compression enum
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileCompression {
    None,
    Gzip,
    Zstd,
    Brotli,
}

impl TileCompression {
    #[inline]
    #[must_use]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

impl TileMetadata {
    /// Convert to TileJSON format
    #[must_use]
    pub fn to_tilejson(&self, base_url: &str) -> TileJson {
        self.to_tilejson_with_key(base_url, None)
    }

    /// Convert to TileJSON format with optional API key
    #[must_use]
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
            encoding: if self.format == TileFormat::Mlt {
                Some("mlt".to_string())
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct TileData {
    pub data: Bytes,
    pub format: TileFormat,
    pub compression: TileCompression,
}

/// Detect if raw tile data is in MLT (MapLibre Tile) format.
///
/// MLT tiles start with a 7-bit varint size followed by tag byte `0x01`.
/// The minimal valid MLT tile is `[0x02, 0x01]`.
///
/// Based on Martin's detection logic:
/// <https://github.com/maplibre/martin/blob/c0c49a7/martin-tile-utils/src/lib.rs#L290>
#[must_use]
pub fn detect_mlt_format(data: &[u8]) -> bool {
    if data.len() < 2 {
        return false;
    }
    decode_7bit_length_and_tag(data).is_ok()
}

fn decode_7bit_length_and_tag(tile: &[u8]) -> std::result::Result<(), ()> {
    let mut pos = 0;
    let len = tile.len();

    while pos < len {
        let mut size: u64 = 0;
        let mut shift = 0u32;
        loop {
            if pos >= len {
                return Err(());
            }
            let b = tile[pos];
            pos += 1;
            size |= u64::from(b & 0x7F) << shift;
            shift += 7;
            if b & 0x80 == 0 {
                break;
            }
            if shift > 63 {
                return Err(());
            }
        }

        if size == 0 {
            return Err(());
        }

        if pos >= len {
            return Err(());
        }
        let tag = tile[pos];
        pos += 1;
        if tag != 0x01 {
            return Err(());
        }

        let payload_len = size.checked_sub(1).ok_or(())?;
        let payload_len_usize: usize = payload_len.try_into().map_err(|_| ())?;
        pos = pos.checked_add(payload_len_usize).ok_or(())?;
        if pos > len {
            return Err(());
        }
    }

    Ok(())
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

    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_format_mlt_content_type() {
        assert_eq!(
            TileFormat::Mlt.content_type(),
            "application/vnd.maplibre-vector-tile"
        );
    }

    #[test]
    fn test_tile_format_mlt_extension() {
        assert_eq!(TileFormat::Mlt.extension(), "mlt");
    }

    #[test]
    fn test_tile_format_from_str_mlt() {
        assert_eq!(TileFormat::from_str("mlt").unwrap(), TileFormat::Mlt);
    }

    #[test]
    fn test_tile_format_from_str_existing() {
        assert_eq!(TileFormat::from_str("pbf").unwrap(), TileFormat::Pbf);
        assert_eq!(TileFormat::from_str("mvt").unwrap(), TileFormat::Pbf);
        assert_eq!(TileFormat::from_str("png").unwrap(), TileFormat::Png);
    }

    #[test]
    fn test_tile_format_is_vector() {
        assert!(TileFormat::Pbf.is_vector());
        assert!(TileFormat::Mlt.is_vector());
        assert!(!TileFormat::Png.is_vector());
        assert!(!TileFormat::Jpeg.is_vector());
        assert!(!TileFormat::Webp.is_vector());
        assert!(!TileFormat::Unknown.is_vector());
    }

    #[test]
    fn test_detect_mlt_minimal_tile() {
        // size=1 (just tag, no payload), tag=0x01
        assert!(detect_mlt_format(&[0x01, 0x01]));
    }

    #[test]
    fn test_detect_mlt_with_payload() {
        // size=4 (tag + 3 payload bytes), tag=0x01, payload=[0xAA, 0xBB, 0xCC]
        assert!(detect_mlt_format(&[0x04, 0x01, 0xAA, 0xBB, 0xCC]));
    }

    #[test]
    fn test_detect_mlt_multiple_layers() {
        // layer1: size=1, tag=0x01 | layer2: size=2, tag=0x01, payload=[0xFF]
        assert!(detect_mlt_format(&[0x01, 0x01, 0x02, 0x01, 0xFF]));
    }

    #[test]
    fn test_detect_mlt_empty() {
        assert!(!detect_mlt_format(&[]));
    }

    #[test]
    fn test_detect_mlt_single_byte() {
        assert!(!detect_mlt_format(&[0x01]));
    }

    #[test]
    fn test_detect_mlt_wrong_tag() {
        assert!(!detect_mlt_format(&[0x02, 0x02]));
    }

    #[test]
    fn test_detect_mlt_rejects_gzip() {
        assert!(!detect_mlt_format(&[0x1F, 0x8B, 0x08, 0x00]));
    }

    #[test]
    fn test_detect_mlt_rejects_protobuf() {
        assert!(!detect_mlt_format(&[0x1A, 0x03, 0x77, 0x61, 0x74]));
    }

    #[test]
    fn test_detect_mlt_size_mismatch() {
        assert!(!detect_mlt_format(&[0x0A, 0x01, 0xFF]));
    }

    #[test]
    fn test_tile_format_mlt_serde_roundtrip() {
        let json = serde_json::to_string(&TileFormat::Mlt).unwrap();
        assert_eq!(json, "\"mlt\"");
        let deserialized: TileFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, TileFormat::Mlt);
    }

    #[test]
    fn test_tilejson_encoding_mlt() {
        let metadata = TileMetadata {
            id: "test".to_string(),
            name: "Test MLT".to_string(),
            description: None,
            attribution: None,
            format: TileFormat::Mlt,
            minzoom: 0,
            maxzoom: 14,
            bounds: None,
            center: None,
            vector_layers: None,
        };
        let tilejson = metadata.to_tilejson("http://localhost:8080");
        assert_eq!(tilejson.encoding, Some("mlt".to_string()));
        assert!(tilejson.tiles[0].contains(".mlt"));
    }

    #[test]
    fn test_tilejson_encoding_pbf() {
        let metadata = TileMetadata {
            id: "test".to_string(),
            name: "Test PBF".to_string(),
            description: None,
            attribution: None,
            format: TileFormat::Pbf,
            minzoom: 0,
            maxzoom: 14,
            bounds: None,
            center: None,
            vector_layers: None,
        };
        let tilejson = metadata.to_tilejson("http://localhost:8080");
        assert_eq!(tilejson.encoding, None);
        assert!(tilejson.tiles[0].contains(".pbf"));
    }

    // --- TileFormat::content_type() exhaustive ---

    #[test]
    fn tile_format_content_type_pbf() {
        assert_eq!(TileFormat::Pbf.content_type(), "application/x-protobuf");
    }

    #[test]
    fn tile_format_content_type_png() {
        assert_eq!(TileFormat::Png.content_type(), "image/png");
    }

    #[test]
    fn tile_format_content_type_jpeg() {
        assert_eq!(TileFormat::Jpeg.content_type(), "image/jpeg");
    }

    #[test]
    fn tile_format_content_type_webp() {
        assert_eq!(TileFormat::Webp.content_type(), "image/webp");
    }

    #[test]
    fn tile_format_content_type_avif() {
        assert_eq!(TileFormat::Avif.content_type(), "image/avif");
    }

    #[test]
    fn tile_format_content_type_unknown() {
        assert_eq!(
            TileFormat::Unknown.content_type(),
            "application/octet-stream"
        );
    }

    // --- TileFormat::extension() exhaustive ---

    #[test]
    fn tile_format_extension_pbf() {
        assert_eq!(TileFormat::Pbf.extension(), "pbf");
    }

    #[test]
    fn tile_format_extension_png() {
        assert_eq!(TileFormat::Png.extension(), "png");
    }

    #[test]
    fn tile_format_extension_jpeg() {
        assert_eq!(TileFormat::Jpeg.extension(), "jpg");
    }

    #[test]
    fn tile_format_extension_webp() {
        assert_eq!(TileFormat::Webp.extension(), "webp");
    }

    #[test]
    fn tile_format_extension_avif() {
        assert_eq!(TileFormat::Avif.extension(), "avif");
    }

    #[test]
    fn tile_format_extension_unknown() {
        assert_eq!(TileFormat::Unknown.extension(), "bin");
    }

    // --- TileFormat::is_vector() exhaustive ---

    #[test]
    fn tile_format_is_vector_pbf_is_true() {
        assert!(TileFormat::Pbf.is_vector());
    }

    #[test]
    fn tile_format_is_vector_mlt_is_true() {
        assert!(TileFormat::Mlt.is_vector());
    }

    #[test]
    fn tile_format_is_vector_png_is_false() {
        assert!(!TileFormat::Png.is_vector());
    }

    #[test]
    fn tile_format_is_vector_jpeg_is_false() {
        assert!(!TileFormat::Jpeg.is_vector());
    }

    #[test]
    fn tile_format_is_vector_webp_is_false() {
        assert!(!TileFormat::Webp.is_vector());
    }

    #[test]
    fn tile_format_is_vector_avif_is_false() {
        assert!(!TileFormat::Avif.is_vector());
    }

    #[test]
    fn tile_format_is_vector_unknown_is_false() {
        assert!(!TileFormat::Unknown.is_vector());
    }

    // --- TileFormat::from_str() exhaustive ---

    #[test]
    fn tile_format_from_str_pbf() {
        assert_eq!("pbf".parse::<TileFormat>(), Ok(TileFormat::Pbf));
    }

    #[test]
    fn tile_format_from_str_mvt() {
        assert_eq!("mvt".parse::<TileFormat>(), Ok(TileFormat::Pbf));
    }

    #[test]
    fn tile_format_from_str_vector() {
        assert_eq!("vector".parse::<TileFormat>(), Ok(TileFormat::Pbf));
    }

    #[test]
    fn tile_format_from_str_png() {
        assert_eq!("png".parse::<TileFormat>(), Ok(TileFormat::Png));
    }

    #[test]
    fn tile_format_from_str_jpeg() {
        assert_eq!("jpeg".parse::<TileFormat>(), Ok(TileFormat::Jpeg));
    }

    #[test]
    fn tile_format_from_str_jpg() {
        assert_eq!("jpg".parse::<TileFormat>(), Ok(TileFormat::Jpeg));
    }

    #[test]
    fn tile_format_from_str_webp() {
        assert_eq!("webp".parse::<TileFormat>(), Ok(TileFormat::Webp));
    }

    #[test]
    fn tile_format_from_str_avif() {
        assert_eq!("avif".parse::<TileFormat>(), Ok(TileFormat::Avif));
    }

    #[test]
    fn tile_format_from_str_mlt() {
        assert_eq!("mlt".parse::<TileFormat>(), Ok(TileFormat::Mlt));
    }

    #[test]
    fn tile_format_from_str_case_insensitive_uppercase() {
        assert_eq!("PNG".parse::<TileFormat>(), Ok(TileFormat::Png));
    }

    #[test]
    fn tile_format_from_str_case_insensitive_mixed() {
        assert_eq!("Pbf".parse::<TileFormat>(), Ok(TileFormat::Pbf));
    }

    #[test]
    fn tile_format_from_str_unknown_returns_err() {
        assert!("xyz".parse::<TileFormat>().is_err());
    }

    #[test]
    fn tile_format_from_str_empty_returns_err() {
        assert!("".parse::<TileFormat>().is_err());
    }

    // --- Equality and clone ---

    #[test]
    fn tile_format_equality() {
        assert_eq!(TileFormat::Pbf, TileFormat::Pbf);
        assert_ne!(TileFormat::Pbf, TileFormat::Png);
    }

    #[test]
    fn tile_format_clone() {
        let fmt = TileFormat::Jpeg;
        let cloned = fmt;
        assert_eq!(fmt, cloned);
    }

    // --- TileFormat serialization roundtrip ---

    #[test]
    fn tile_format_serialize_pbf() {
        let json = serde_json::to_string(&TileFormat::Pbf).expect("serialize TileFormat");
        assert_eq!(json, "\"pbf\"");
    }

    #[test]
    fn tile_format_deserialize_png() {
        let fmt: TileFormat = serde_json::from_str("\"png\"").expect("deserialize TileFormat");
        assert_eq!(fmt, TileFormat::Png);
    }

    #[test]
    fn tile_format_deserialize_jpeg() {
        let fmt: TileFormat = serde_json::from_str("\"jpeg\"").expect("deserialize jpeg");
        assert_eq!(fmt, TileFormat::Jpeg);
    }

    #[test]
    fn tile_format_deserialize_webp() {
        let fmt: TileFormat = serde_json::from_str("\"webp\"").expect("deserialize webp");
        assert_eq!(fmt, TileFormat::Webp);
    }

    #[test]
    fn tile_format_deserialize_avif() {
        let fmt: TileFormat = serde_json::from_str("\"avif\"").expect("deserialize avif");
        assert_eq!(fmt, TileFormat::Avif);
    }

    #[test]
    fn tile_format_deserialize_unknown() {
        let fmt: TileFormat = serde_json::from_str("\"unknown\"").expect("deserialize unknown");
        assert_eq!(fmt, TileFormat::Unknown);
    }
}
