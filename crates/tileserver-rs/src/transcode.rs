//! MLT (MapLibre Tiles) transcoding module.
//!
//! Provides conversion between MVT (Mapbox Vector Tiles, protobuf)
//! and MLT (MapLibre Tiles) formats. This enables:
//!
//! - **Phase 2**: Serve existing MVT/PBF sources as MLT tiles (MVT→MLT encoding)
//! - **Phase 3**: Serve MLT sources as MVT/PBF for backward compatibility with legacy clients
//!
//! Gated behind the `mlt` cargo feature.

use bytes::Bytes;
use flate2::read::GzDecoder;
use std::io::Read;

use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat};

// ---------------------------------------------------------------------------
// MVT Protobuf types (minimal prost-generated structs for encoding MVT tiles)
// ---------------------------------------------------------------------------

/// Minimal MVT protobuf types for encoding vector tiles.
///
/// These mirror the Mapbox Vector Tile specification v2.1 protobuf schema.
/// Used only for MLT→MVT reverse transcoding (Phase 3).
#[allow(non_snake_case)]
pub mod MvtProto {
    /// MVT geometry type.
    #[non_exhaustive]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, prost::Enumeration)]
    #[repr(i32)]
    pub enum GeomType {
        Unknown = 0,
        Point = 1,
        Linestring = 2,
        Polygon = 3,
    }

    /// MVT property value.
    #[derive(Clone, PartialEq, prost::Message)]
    pub struct Value {
        #[prost(string, optional, tag = "1")]
        pub string_value: Option<String>,
        #[prost(float, optional, tag = "2")]
        pub float_value: Option<f32>,
        #[prost(double, optional, tag = "3")]
        pub double_value: Option<f64>,
        #[prost(int64, optional, tag = "4")]
        pub int_value: Option<i64>,
        #[prost(uint64, optional, tag = "5")]
        pub uint_value: Option<u64>,
        #[prost(sint64, optional, tag = "6")]
        pub sint_value: Option<i64>,
        #[prost(bool, optional, tag = "7")]
        pub bool_value: Option<bool>,
    }

    /// MVT feature within a layer.
    #[derive(Clone, PartialEq, prost::Message)]
    pub struct Feature {
        #[prost(uint64, optional, tag = "1")]
        pub id: Option<u64>,
        #[prost(uint32, repeated, packed = "true", tag = "2")]
        pub tags: Vec<u32>,
        #[prost(enumeration = "GeomType", optional, tag = "3")]
        pub r#type: Option<i32>,
        #[prost(uint32, repeated, packed = "true", tag = "4")]
        pub geometry: Vec<u32>,
    }

    /// MVT layer within a tile.
    #[derive(Clone, PartialEq, prost::Message)]
    pub struct Layer {
        #[prost(uint32, required, tag = "15")]
        pub version: u32,
        #[prost(string, required, tag = "1")]
        pub name: String,
        #[prost(message, repeated, tag = "2")]
        pub features: Vec<Feature>,
        #[prost(string, repeated, tag = "3")]
        pub keys: Vec<String>,
        #[prost(message, repeated, tag = "4")]
        pub values: Vec<Value>,
        #[prost(uint32, optional, tag = "5")]
        pub extent: Option<u32>,
    }

    /// MVT tile (top-level message).
    #[derive(Clone, PartialEq, prost::Message)]
    pub struct Tile {
        #[prost(message, repeated, tag = "3")]
        pub layers: Vec<Layer>,
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Transcode a tile from its current format to a target format.
///
/// Handles decompression of source data if needed (gzip).
/// Returns a new `TileData` with the transcoded bytes and appropriate format.
///
/// # Supported conversions
///
/// | From | To  | Description |
/// |------|-----|-------------|
/// | PBF  | MLT | MVT→MLT encoding via mlt-core (Phase 2) |
/// | MLT  | PBF | MLT→MVT decoding (Phase 3) |
///
/// # Errors
///
/// Returns `TileServerError::TranscodeUnsupported` for unsupported format pairs.
/// Returns `TileServerError::MltEncodeError` or `MltDecodeError` on conversion failure.
pub fn transcode_tile(tile: &TileData, target_format: TileFormat) -> Result<TileData> {
    // No-op if formats already match
    if tile.format == target_format {
        return Ok(tile.clone());
    }

    match (tile.format, target_format) {
        (TileFormat::Pbf, TileFormat::Mlt) => {
            // Phase 2: MVT→MLT encoding using mlt-core's encoding API.
            // Wrap in catch_unwind because mlt-core can panic on certain
            // geometries (off-by-one in geometry encoder, see #651).
            let raw = decompress_tile_data(tile)?;
            let mlt_bytes = std::panic::catch_unwind(|| mvt_to_mlt(&raw)).map_err(|panic| {
                let msg = panic
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| panic.downcast_ref::<&str>().copied())
                    .unwrap_or("unknown panic");
                TileServerError::MltEncodeError(format!(
                    "mlt-core panicked during MVT→MLT encoding: {msg}"
                ))
            })??;
            Ok(TileData {
                data: mlt_bytes,
                format: TileFormat::Mlt,
                compression: TileCompression::None,
            })
        }
        (TileFormat::Mlt, TileFormat::Pbf) => {
            let raw = decompress_tile_data(tile)?;
            let mvt_bytes = mlt_to_mvt(&raw)?;
            Ok(TileData {
                data: mvt_bytes,
                format: TileFormat::Pbf,
                compression: TileCompression::None,
            })
        }
        (from, to) => Err(TileServerError::TranscodeUnsupported {
            from: format!("{from:?}"),
            to: format!("{to:?}"),
        }),
    }
}

// ---------------------------------------------------------------------------
// Internal: Decompression
// ---------------------------------------------------------------------------

/// Decompress tile data if compressed, returning raw bytes.
fn decompress_tile_data(tile: &TileData) -> Result<Vec<u8>> {
    match tile.compression {
        TileCompression::None => Ok(tile.data.to_vec()),
        TileCompression::Gzip => {
            let mut decoder = GzDecoder::new(tile.data.as_ref());
            let mut decompressed = Vec::with_capacity(tile.data.len() * 4);
            decoder.read_to_end(&mut decompressed).map_err(|e| {
                TileServerError::MltDecodeError(format!("gzip decompression failed: {e}"))
            })?;
            Ok(decompressed)
        }
        _ => Err(TileServerError::MltDecodeError(format!(
            "{:?} decompression not supported for transcoding",
            tile.compression
        ))),
    }
}

// ---------------------------------------------------------------------------
// Phase 2: MVT → MLT encoding
// ---------------------------------------------------------------------------

/// Convert MVT (protobuf) bytes to MLT format.
///
/// Uses `mlt-core` to:
/// 1. Parse MVT binary into a `FeatureCollection`
/// 2. Group features by layer (via `_layer` property)
/// 3. Build decoded geometry, IDs, and column-oriented properties per layer
/// 4. Encode each column using mlt-core's encoding API
/// 5. Serialize encoded layers to MLT wire format
fn mvt_to_mlt(mvt_bytes: &[u8]) -> Result<Bytes> {
    use std::collections::BTreeMap;

    use mlt_core::encoder::EncoderConfig;

    let fc = mlt_core::mvt::mvt_to_feature_collection(mvt_bytes)
        .map_err(|e| TileServerError::MltEncodeError(format!("failed to parse MVT tile: {e}")))?;

    let mut layer_map: BTreeMap<String, Vec<&mlt_core::geojson::Feature>> = BTreeMap::new();
    for feature in &fc.features {
        let layer_name = feature
            .properties
            .get("_layer")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        layer_map.entry(layer_name).or_default().push(feature);
    }

    let mut output = Vec::with_capacity(mvt_bytes.len());
    for (layer_name, features) in &layer_map {
        let tile_layer = build_tile_layer(layer_name, features);
        // mlt-core 0.9: TileLayer::encode tries sort strategies internally + returns bytes.
        // (Renamed from TileLayer01 in 0.9.0; identical field layout + encode signature.)
        let bytes = tile_layer.encode(EncoderConfig::default()).map_err(|e| {
            TileServerError::MltEncodeError(format!("failed to encode MLT layer: {e}"))
        })?;
        output.extend_from_slice(&bytes);
    }

    Ok(Bytes::from(output))
}

/// Build a [`TileLayer`] from a set of features belonging to one MVT layer.
///
/// Collects unique property keys, infers the dominant type for each key across
/// all features, and builds per-feature [`PropValue`] vectors parallel to
/// `property_names`.
fn build_tile_layer(
    layer_name: &str,
    features: &[&mlt_core::geojson::Feature],
) -> mlt_core::TileLayer {
    use std::collections::BTreeSet;

    use mlt_core::{PropValue, TileFeature, TileLayer};

    // Extract extent from first feature (injected by mvt_to_feature_collection as _extent)
    let extent = features
        .first()
        .and_then(|f| f.properties.get("_extent"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(4096);

    // Collect all unique property keys (excluding internal _layer, _extent)
    let mut key_set = BTreeSet::new();
    for feature in features {
        for key in feature.properties.keys() {
            if !key.starts_with('_') {
                key_set.insert(key.clone());
            }
        }
    }
    let property_names: Vec<String> = key_set.into_iter().collect();

    // Determine dominant type per property key
    let key_types: Vec<DominantType> = property_names
        .iter()
        .map(|key| infer_dominant_type(features, key))
        .collect();

    // Build TileFeature per feature
    let tile_features: Vec<TileFeature> = features
        .iter()
        .map(|feature| {
            let properties: Vec<PropValue> = property_names
                .iter()
                .zip(&key_types)
                .map(|(key, dominant)| json_to_prop_value(feature.properties.get(key), *dominant))
                .collect();

            TileFeature {
                id: feature.id,
                geometry: feature.geometry.clone(),
                properties,
            }
        })
        .collect();

    TileLayer {
        name: layer_name.to_string(),
        extent,
        property_names,
        features: tile_features,
    }
}

/// Dominant type classification for a property column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DominantType {
    String,
    Bool,
    Float,
    Int,
}

/// Infer the dominant type for a property key by scanning all features.
///
/// Priority: String (if mixed or string present) > Bool > Float > Int.
/// Falls back to String if no values are found.
fn infer_dominant_type(features: &[&mlt_core::geojson::Feature], key: &str) -> DominantType {
    let mut has_string = false;
    let mut has_float = false;
    let mut has_int = false;
    let mut has_bool = false;

    for feature in features {
        if let Some(val) = feature.properties.get(key) {
            match val {
                serde_json::Value::String(_) => has_string = true,
                serde_json::Value::Bool(_) => has_bool = true,
                serde_json::Value::Number(n) => {
                    if n.is_f64() && !n.is_i64() && !n.is_u64() {
                        has_float = true;
                    } else {
                        has_int = true;
                    }
                }
                _ => {}
            }
        }
    }

    // Mixed types → String (can represent anything)
    if has_string || (!has_int && !has_float && !has_bool) {
        DominantType::String
    } else if has_bool && !has_int && !has_float {
        DominantType::Bool
    } else if has_float {
        DominantType::Float
    } else {
        DominantType::Int
    }
}

/// Convert a JSON property value to a per-feature [`PropValue`] based on the dominant type.
fn json_to_prop_value(
    value: Option<&serde_json::Value>,
    dominant: DominantType,
) -> mlt_core::PropValue {
    use mlt_core::PropValue;

    match dominant {
        DominantType::String => {
            let s = value.and_then(|v| match v {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Null => None,
                other => Some(other.to_string()),
            });
            PropValue::Str(s)
        }
        DominantType::Bool => PropValue::Bool(value.and_then(|v| v.as_bool())),
        DominantType::Float => PropValue::F64(value.and_then(|v| v.as_f64())),
        DominantType::Int => PropValue::I64(value.and_then(|v| v.as_i64())),
    }
}

// ---------------------------------------------------------------------------
// Phase 3: MLT → MVT decoding
// ---------------------------------------------------------------------------

/// Convert MLT bytes to MVT (protobuf) format via `mlt-core`'s native writer.
///
/// `Layer01::into_tile` carries the MLT layer name and extent through directly,
/// so the `_layer`/`_extent` injected-property convention used by the GeoJSON
/// (`FeatureCollection::from_layers`) bridge is intentionally not needed here.
/// `Layer::Unknown` variants carry tags this mlt-core version cannot model as
/// MVT and are skipped rather than failing the whole tile.
fn mlt_to_mvt(mlt_bytes: &[u8]) -> Result<Bytes> {
    let mut parser = mlt_core::Parser::default();
    let layers = parser
        .parse_layers(mlt_bytes)
        .map_err(|e| TileServerError::MltDecodeError(format!("failed to parse MLT tile: {e}")))?;

    let mut decoder = mlt_core::Decoder::default();
    let mut tile_layers: Vec<mlt_core::TileLayer> = Vec::with_capacity(layers.len());
    for layer in layers {
        if let mlt_core::Layer::Tag01(tag01) = layer {
            let tile_layer = tag01.into_tile(&mut decoder).map_err(|e| {
                TileServerError::MltDecodeError(format!("failed to decode MLT layer: {e}"))
            })?;
            tile_layers.push(tile_layer);
        }
    }

    let encoded = mlt_core::mvt::tile_layers_to_mvt(tile_layers)
        .map_err(|e| TileServerError::MltEncodeError(format!("failed to encode MVT tile: {e}")))?;

    Ok(Bytes::from(encoded))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// MVT command integer: `(id & 0x7) | (count << 3)`.
    fn command_integer(id: u32, count: u32) -> u32 {
        (id & 0x7) | (count << 3)
    }

    /// Zigzag encoding for signed protobuf ints: `(n << 1) ^ (n >> 31)`.
    fn zigzag_encode(n: i32) -> u32 {
        ((n << 1) ^ (n >> 31)) as u32
    }

    #[test]
    fn test_zigzag_encode() {
        assert_eq!(zigzag_encode(0), 0);
        assert_eq!(zigzag_encode(-1), 1);
        assert_eq!(zigzag_encode(1), 2);
        assert_eq!(zigzag_encode(-2), 3);
        assert_eq!(zigzag_encode(2), 4);
    }

    #[test]
    fn test_command_integer() {
        // MoveTo, count=1
        assert_eq!(command_integer(1, 1), 9);
        // LineTo, count=3
        assert_eq!(command_integer(2, 3), 26);
        // ClosePath, count=1
        assert_eq!(command_integer(7, 1), 15);
    }

    #[test]
    fn test_transcode_same_format_is_noop() {
        let tile = TileData {
            data: Bytes::from_static(b"test"),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        let result = transcode_tile(&tile, TileFormat::Pbf).unwrap();
        assert_eq!(result.data, tile.data);
        assert_eq!(result.format, TileFormat::Pbf);
    }

    #[test]
    fn test_transcode_unsupported_pair() {
        let tile = TileData {
            data: Bytes::from_static(b"test"),
            format: TileFormat::Png,
            compression: TileCompression::None,
        };
        let result = transcode_tile(&tile, TileFormat::Mlt);
        assert!(result.is_err());
        match result.unwrap_err() {
            TileServerError::TranscodeUnsupported { from, to } => {
                assert_eq!(from, "Png");
                assert_eq!(to, "Mlt");
            }
            e => panic!("expected TranscodeUnsupported, got: {e:?}"),
        }
    }

    #[test]
    fn test_mvt_to_mlt_invalid_input_returns_error() {
        // Invalid protobuf bytes should return an encode error, not panic
        let tile = TileData {
            data: Bytes::from_static(b"not valid protobuf"),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        let result = transcode_tile(&tile, TileFormat::Mlt);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TileServerError::MltEncodeError(_)
        ));
    }

    #[test]
    fn test_mvt_to_mlt_catches_panic_from_mlt_core() {
        // Craft a tile with valid protobuf structure but geometry that
        // could trigger an mlt-core panic. Even if mlt-core panics,
        // transcode_tile should return an error, not crash the thread.
        // We verify the catch_unwind wrapper by ensuring any failure
        // on malformed geometry returns Err, not a panic propagation.
        use prost::Message;
        let tile_proto = MvtProto::Tile {
            layers: vec![MvtProto::Layer {
                version: 2,
                name: "test".to_string(),
                features: vec![MvtProto::Feature {
                    id: Some(1),
                    tags: vec![],
                    r#type: Some(3), // POLYGON
                    // Malformed geometry: ClosePath without preceding MoveTo/LineTo
                    geometry: vec![
                        command_integer(7, 1), // ClosePath x1 (invalid without MoveTo)
                    ],
                }],
                keys: vec![],
                values: vec![],
                extent: Some(4096),
            }],
        };
        let mut mvt_bytes = Vec::new();
        tile_proto.encode(&mut mvt_bytes).unwrap();

        let tile = TileData {
            data: Bytes::from(mvt_bytes),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        // Should not panic — either succeeds or returns an error
        let result = transcode_tile(&tile, TileFormat::Mlt);
        // We don't assert is_err() because mlt-core may handle this
        // gracefully; the key assertion is that we reach this line
        // (no panic propagation).
        let _ = result;
    }

    // -------------------------------------------------------------------------
    // Helper: Build a valid MVT protobuf tile from layers of features
    // -------------------------------------------------------------------------

    /// Build a minimal valid MVT tile with one layer, one point feature.
    fn make_mvt_point_tile(layer_name: &str, x: i32, y: i32) -> Vec<u8> {
        use prost::Message;
        let tile = MvtProto::Tile {
            layers: vec![MvtProto::Layer {
                version: 2,
                name: layer_name.to_string(),
                features: vec![MvtProto::Feature {
                    id: Some(1),
                    tags: vec![0, 0], // key[0] = "name", value[0] = "test"
                    r#type: Some(MvtProto::GeomType::Point as i32),
                    geometry: vec![
                        command_integer(1, 1), // MoveTo(1)
                        zigzag_encode(x),
                        zigzag_encode(y),
                    ],
                }],
                keys: vec!["name".to_string()],
                values: vec![MvtProto::Value {
                    string_value: Some("test".to_string()),
                    ..Default::default()
                }],
                extent: Some(4096),
            }],
        };
        tile.encode_to_vec()
    }

    /// Build a multi-feature MVT layer with various geometry types.
    fn make_mvt_multi_feature_tile() -> Vec<u8> {
        use prost::Message;
        let tile = MvtProto::Tile {
            layers: vec![MvtProto::Layer {
                version: 2,
                name: "buildings".to_string(),
                features: vec![
                    // Feature 1: Point
                    MvtProto::Feature {
                        id: Some(1),
                        tags: vec![0, 0, 1, 1], // name=building_a, height=10
                        r#type: Some(MvtProto::GeomType::Point as i32),
                        geometry: vec![
                            command_integer(1, 1),
                            zigzag_encode(100),
                            zigzag_encode(200),
                        ],
                    },
                    // Feature 2: Point
                    MvtProto::Feature {
                        id: Some(2),
                        tags: vec![0, 2, 1, 3], // name=building_b, height=25
                        r#type: Some(MvtProto::GeomType::Point as i32),
                        geometry: vec![
                            command_integer(1, 1),
                            zigzag_encode(300),
                            zigzag_encode(400),
                        ],
                    },
                    // Feature 3: Point with no ID
                    MvtProto::Feature {
                        id: None,
                        tags: vec![0, 4], // name=building_c
                        r#type: Some(MvtProto::GeomType::Point as i32),
                        geometry: vec![
                            command_integer(1, 1),
                            zigzag_encode(500),
                            zigzag_encode(600),
                        ],
                    },
                ],
                keys: vec!["name".to_string(), "height".to_string()],
                values: vec![
                    MvtProto::Value {
                        string_value: Some("building_a".to_string()),
                        ..Default::default()
                    },
                    MvtProto::Value {
                        int_value: Some(10),
                        ..Default::default()
                    },
                    MvtProto::Value {
                        string_value: Some("building_b".to_string()),
                        ..Default::default()
                    },
                    MvtProto::Value {
                        int_value: Some(25),
                        ..Default::default()
                    },
                    MvtProto::Value {
                        string_value: Some("building_c".to_string()),
                        ..Default::default()
                    },
                ],
                extent: Some(4096),
            }],
        };
        tile.encode_to_vec()
    }

    /// Build an MVT tile with multiple layers.
    fn make_mvt_multi_layer_tile() -> Vec<u8> {
        use prost::Message;
        let tile = MvtProto::Tile {
            layers: vec![
                MvtProto::Layer {
                    version: 2,
                    name: "roads".to_string(),
                    features: vec![MvtProto::Feature {
                        id: Some(1),
                        tags: vec![0, 0],
                        r#type: Some(MvtProto::GeomType::Linestring as i32),
                        geometry: vec![
                            command_integer(1, 1), // MoveTo
                            zigzag_encode(0),
                            zigzag_encode(0),
                            command_integer(2, 1), // LineTo(1)
                            zigzag_encode(100),
                            zigzag_encode(0),
                        ],
                    }],
                    keys: vec!["class".to_string()],
                    values: vec![MvtProto::Value {
                        string_value: Some("highway".to_string()),
                        ..Default::default()
                    }],
                    extent: Some(4096),
                },
                MvtProto::Layer {
                    version: 2,
                    name: "water".to_string(),
                    features: vec![MvtProto::Feature {
                        id: Some(10),
                        tags: vec![0, 0],
                        r#type: Some(MvtProto::GeomType::Polygon as i32),
                        geometry: vec![
                            command_integer(1, 1), // MoveTo
                            zigzag_encode(10),
                            zigzag_encode(10),
                            command_integer(2, 3), // LineTo(3)
                            zigzag_encode(100),
                            zigzag_encode(0),
                            zigzag_encode(0),
                            zigzag_encode(100),
                            zigzag_encode(-100),
                            zigzag_encode(0),
                            command_integer(7, 1), // ClosePath
                        ],
                    }],
                    keys: vec!["type".to_string()],
                    values: vec![MvtProto::Value {
                        string_value: Some("lake".to_string()),
                        ..Default::default()
                    }],
                    extent: Some(4096),
                },
            ],
        };
        tile.encode_to_vec()
    }

    // -------------------------------------------------------------------------
    // MVT → MLT transcoding tests (Phase 2)
    // -------------------------------------------------------------------------

    #[test]
    fn test_mvt_to_mlt_point_tile() {
        let mvt_bytes = make_mvt_point_tile("places", 100, 200);
        let tile = TileData {
            data: Bytes::from(mvt_bytes),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        let result = transcode_tile(&tile, TileFormat::Mlt);
        assert!(
            result.is_ok(),
            "MVT→MLT transcoding should succeed: {:?}",
            result.err()
        );
        let mlt_tile = result.unwrap();
        assert_eq!(mlt_tile.format, TileFormat::Mlt);
        assert_eq!(mlt_tile.compression, TileCompression::None);
        assert!(!mlt_tile.data.is_empty(), "MLT output should not be empty");
        // Verify MLT data is valid by parsing it back
        let layers = mlt_core::Parser::default().parse_layers(&mlt_tile.data);
        assert!(
            layers.is_ok(),
            "MLT output should be parseable: {:?}",
            layers.err()
        );
    }

    #[test]
    fn test_mvt_to_mlt_multi_feature_tile() {
        let mvt_bytes = make_mvt_multi_feature_tile();
        let tile = TileData {
            data: Bytes::from(mvt_bytes),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        let result = transcode_tile(&tile, TileFormat::Mlt).unwrap();
        assert_eq!(result.format, TileFormat::Mlt);
        assert!(!result.data.is_empty());
        // Verify MLT is parseable
        let layers = mlt_core::Parser::default().parse_layers(&result.data);
        assert!(layers.is_ok());
    }

    #[test]
    fn test_mvt_to_mlt_gzip_compressed_input() {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        let mvt_bytes = make_mvt_point_tile("compressed", 50, 50);
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&mvt_bytes).unwrap();
        let compressed = encoder.finish().unwrap();

        let tile = TileData {
            data: Bytes::from(compressed),
            format: TileFormat::Pbf,
            compression: TileCompression::Gzip,
        };
        let result = transcode_tile(&tile, TileFormat::Mlt).unwrap();
        assert_eq!(result.format, TileFormat::Mlt);
        assert_eq!(result.compression, TileCompression::None);
        assert!(!result.data.is_empty());
        let layers = mlt_core::Parser::default().parse_layers(&result.data);
        assert!(layers.is_ok());
    }

    #[test]
    fn test_mvt_to_mlt_output_differs_from_input() {
        // Ensure the transcoded output is actually different from the MVT input
        let mvt_bytes = make_mvt_point_tile("test_layer", 100, 200);
        let tile = TileData {
            data: Bytes::from(mvt_bytes.clone()),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        let result = transcode_tile(&tile, TileFormat::Mlt).unwrap();
        // MLT format is structurally different from MVT protobuf
        assert_ne!(
            result.data.as_ref(),
            mvt_bytes.as_slice(),
            "MLT output should differ from MVT input"
        );
    }

    // -------------------------------------------------------------------------
    // MVT → MLT → MVT roundtrip tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_roundtrip_mvt_mlt_mvt_point() {
        use prost::Message;

        let mvt_bytes = make_mvt_point_tile("roundtrip", 100, 200);
        let original_tile = TileData {
            data: Bytes::from(mvt_bytes),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        let mlt_tile = transcode_tile(&original_tile, TileFormat::Mlt).unwrap();
        assert_eq!(mlt_tile.format, TileFormat::Mlt);
        let roundtripped = transcode_tile(&mlt_tile, TileFormat::Pbf).unwrap();
        assert_eq!(roundtripped.format, TileFormat::Pbf);

        let tile = MvtProto::Tile::decode(roundtripped.data.as_ref())
            .expect("roundtripped MVT should decode");
        assert_eq!(tile.layers.len(), 1, "point tile keeps its single layer");
        let layer = tile
            .layers
            .iter()
            .find(|l| l.name == "roundtrip")
            .expect("layer name \"roundtrip\" should survive the roundtrip");
        assert_eq!(layer.features.len(), 1, "single point feature preserved");
        assert!(
            layer
                .features
                .iter()
                .any(|f| f.r#type == Some(MvtProto::GeomType::Point as i32)),
            "point geometry type preserved"
        );
    }

    #[test]
    fn test_roundtrip_mvt_mlt_mvt_multi_feature() {
        use prost::Message;

        let mvt_bytes = make_mvt_multi_feature_tile();
        let original = TileData {
            data: Bytes::from(mvt_bytes),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        let mlt = transcode_tile(&original, TileFormat::Mlt).unwrap();
        let roundtripped = transcode_tile(&mlt, TileFormat::Pbf).unwrap();
        assert_eq!(roundtripped.format, TileFormat::Pbf);

        let tile = MvtProto::Tile::decode(roundtripped.data.as_ref())
            .expect("roundtripped MVT should decode");
        assert_eq!(tile.layers.len(), 1, "single layer preserved");
        let layer = tile
            .layers
            .iter()
            .find(|l| l.name == "buildings")
            .expect("layer name \"buildings\" should survive the roundtrip");
        assert_eq!(layer.features.len(), 3, "all three features preserved");
        assert!(
            layer
                .features
                .iter()
                .any(|f| f.r#type == Some(MvtProto::GeomType::Point as i32)),
            "point geometry type preserved"
        );
    }

    #[test]
    fn test_roundtrip_mvt_mlt_mvt_multi_layer() {
        use prost::Message;

        let mvt_bytes = make_mvt_multi_layer_tile();
        let original = TileData {
            data: Bytes::from(mvt_bytes),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        let mlt = transcode_tile(&original, TileFormat::Mlt).unwrap();
        let roundtripped = transcode_tile(&mlt, TileFormat::Pbf).unwrap();
        assert_eq!(roundtripped.format, TileFormat::Pbf);

        let tile = MvtProto::Tile::decode(roundtripped.data.as_ref())
            .expect("roundtripped MVT should decode");
        assert_eq!(tile.layers.len(), 2, "both layers preserved");

        let roads = tile
            .layers
            .iter()
            .find(|l| l.name == "roads")
            .expect("layer name \"roads\" should survive the roundtrip");
        assert_eq!(roads.features.len(), 1, "roads keeps its single feature");
        assert!(
            roads
                .features
                .iter()
                .any(|f| f.r#type == Some(MvtProto::GeomType::Linestring as i32)),
            "roads linestring geometry preserved"
        );

        let water = tile
            .layers
            .iter()
            .find(|l| l.name == "water")
            .expect("layer name \"water\" should survive the roundtrip");
        assert_eq!(water.features.len(), 1, "water keeps its single feature");
        assert!(
            water
                .features
                .iter()
                .any(|f| f.r#type == Some(MvtProto::GeomType::Polygon as i32)),
            "water polygon geometry preserved"
        );
    }

    #[test]
    fn test_roundtrip_preserves_layer_count() {
        let mvt_bytes = make_mvt_multi_layer_tile();
        let original = TileData {
            data: Bytes::from(mvt_bytes.clone()),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        // Parse original MVT
        use prost::Message;
        let orig_tile = MvtProto::Tile::decode(mvt_bytes.as_slice()).unwrap();
        // Roundtrip
        let mlt = transcode_tile(&original, TileFormat::Mlt).unwrap();
        let roundtripped = transcode_tile(&mlt, TileFormat::Pbf).unwrap();
        let rt_tile = MvtProto::Tile::decode(roundtripped.data.as_ref()).unwrap();
        assert_eq!(
            orig_tile.layers.len(),
            rt_tile.layers.len(),
            "Roundtrip should preserve layer count"
        );
    }

    #[test]
    fn test_roundtrip_preserves_feature_count() {
        let mvt_bytes = make_mvt_multi_feature_tile();
        let original = TileData {
            data: Bytes::from(mvt_bytes.clone()),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        use prost::Message;
        let orig_tile = MvtProto::Tile::decode(mvt_bytes.as_slice()).unwrap();
        let orig_feature_count: usize = orig_tile.layers.iter().map(|l| l.features.len()).sum();
        // Roundtrip
        let mlt = transcode_tile(&original, TileFormat::Mlt).unwrap();
        let roundtripped = transcode_tile(&mlt, TileFormat::Pbf).unwrap();
        let rt_tile = MvtProto::Tile::decode(roundtripped.data.as_ref()).unwrap();
        let rt_feature_count: usize = rt_tile.layers.iter().map(|l| l.features.len()).sum();
        assert_eq!(
            orig_feature_count, rt_feature_count,
            "Roundtrip should preserve total feature count"
        );
    }

    // -------------------------------------------------------------------------
    // Internal function tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_mvt_to_mlt_internal_single_point() {
        let mvt_bytes = make_mvt_point_tile("internal_test", 42, 84);
        let result = mvt_to_mlt(&mvt_bytes);
        assert!(
            result.is_ok(),
            "mvt_to_mlt should succeed: {:?}",
            result.err()
        );
        let mlt_bytes = result.unwrap();
        assert!(!mlt_bytes.is_empty());
    }

    #[test]
    fn test_mvt_to_mlt_internal_empty_input() {
        // Empty protobuf encodes as zero bytes
        let result = mvt_to_mlt(&[]);
        // Should either succeed (empty tile) or give a clean error
        if let Ok(mlt_bytes) = result {
            // Empty tile might produce empty output
            let _ = mlt_bytes;
        }
    }

    #[test]
    fn test_decompress_tile_data_none() {
        let tile = TileData {
            data: Bytes::from_static(b"raw bytes"),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        let result = decompress_tile_data(&tile).unwrap();
        assert_eq!(result, b"raw bytes");
    }

    #[test]
    fn test_decompress_tile_data_gzip() {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        let original = b"hello world";
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let tile = TileData {
            data: Bytes::from(compressed),
            format: TileFormat::Pbf,
            compression: TileCompression::Gzip,
        };
        let result = decompress_tile_data(&tile).unwrap();
        assert_eq!(result, original);
    }

    #[test]
    fn test_decompress_tile_data_invalid_gzip() {
        let tile = TileData {
            data: Bytes::from_static(b"not gzip data"),
            format: TileFormat::Pbf,
            compression: TileCompression::Gzip,
        };
        let result = decompress_tile_data(&tile);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // Zigzag encoding edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn test_zigzag_encode_min_max() {
        assert_eq!(zigzag_encode(i32::MAX), (u32::MAX - 1));
        assert_eq!(zigzag_encode(i32::MIN), u32::MAX);
    }

    #[test]
    fn test_zigzag_encode_symmetry() {
        // Positive and negative values should alternate
        for i in 0..100 {
            let pos = zigzag_encode(i);
            let neg = zigzag_encode(-i);
            if i == 0 {
                assert_eq!(pos, neg);
            } else {
                assert_eq!(pos, neg + 1, "zigzag({i}) should be zigzag(-{i}) + 1");
            }
        }
    }

    // -------------------------------------------------------------------------
    // Command integer encoding tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_command_integer_all_types() {
        // MoveTo = 1
        assert_eq!(command_integer(1, 1), 0b0000_1001); // 9
        assert_eq!(command_integer(1, 2), 0b0001_0001); // 17
        // LineTo = 2
        assert_eq!(command_integer(2, 1), 0b0000_1010); // 10
        assert_eq!(command_integer(2, 5), 0b0010_1010); // 42
        // ClosePath = 7
        assert_eq!(command_integer(7, 1), 0b0000_1111); // 15
    }

    // -------------------------------------------------------------------------
    // Type inference tests (infer_dominant_type)
    // -------------------------------------------------------------------------

    #[test]
    fn test_infer_dominant_type_all_strings() {
        let features = [
            make_test_feature(serde_json::json!({"k": "a"})),
            make_test_feature(serde_json::json!({"k": "b"})),
        ];
        let refs: Vec<&mlt_core::geojson::Feature> = features.iter().collect();
        let dt = infer_dominant_type(&refs, "k");
        assert!(
            matches!(dt, DominantType::String),
            "Expected String, got: {dt:?}"
        );
    }

    #[test]
    fn test_infer_dominant_type_all_ints() {
        let features = [
            make_test_feature(serde_json::json!({"k": 10})),
            make_test_feature(serde_json::json!({"k": 20})),
        ];
        let refs: Vec<&mlt_core::geojson::Feature> = features.iter().collect();
        let dt = infer_dominant_type(&refs, "k");
        assert!(matches!(dt, DominantType::Int), "Expected Int, got: {dt:?}");
    }

    #[test]
    fn test_infer_dominant_type_all_bools() {
        let features = [
            make_test_feature(serde_json::json!({"k": true})),
            make_test_feature(serde_json::json!({"k": false})),
        ];
        let refs: Vec<&mlt_core::geojson::Feature> = features.iter().collect();
        let dt = infer_dominant_type(&refs, "k");
        assert!(
            matches!(dt, DominantType::Bool),
            "Expected Bool, got: {dt:?}"
        );
    }

    #[test]
    fn test_infer_dominant_type_all_floats() {
        let features = [
            make_test_feature(serde_json::json!({"k": 1.5})),
            make_test_feature(serde_json::json!({"k": 2.7})),
        ];
        let refs: Vec<&mlt_core::geojson::Feature> = features.iter().collect();
        let dt = infer_dominant_type(&refs, "k");
        assert!(
            matches!(dt, DominantType::Float),
            "Expected Float, got: {dt:?}"
        );
    }

    #[test]
    fn test_infer_dominant_type_mixed_int_float_promotes_to_float() {
        let features = [
            make_test_feature(serde_json::json!({"k": 10})),
            make_test_feature(serde_json::json!({"k": 2.72})),
        ];
        let refs: Vec<&mlt_core::geojson::Feature> = features.iter().collect();
        let dt = infer_dominant_type(&refs, "k");
        assert!(
            matches!(dt, DominantType::Float),
            "Expected Float for mixed int/float, got: {dt:?}"
        );
    }

    #[test]
    fn test_infer_dominant_type_mixed_types_falls_back_to_string() {
        let features = [
            make_test_feature(serde_json::json!({"k": "hello"})),
            make_test_feature(serde_json::json!({"k": 42})),
        ];
        let refs: Vec<&mlt_core::geojson::Feature> = features.iter().collect();
        let dt = infer_dominant_type(&refs, "k");
        assert!(
            matches!(dt, DominantType::String),
            "Expected String for mixed types, got: {dt:?}"
        );
    }

    #[test]
    fn test_infer_dominant_type_missing_key() {
        let features = [
            make_test_feature(serde_json::json!({"k": "present"})),
            make_test_feature(serde_json::json!({"other": "no k"})),
        ];
        let refs: Vec<&mlt_core::geojson::Feature> = features.iter().collect();
        let dt = infer_dominant_type(&refs, "k");
        assert!(
            matches!(dt, DominantType::String),
            "Expected String when key present in some features, got: {dt:?}"
        );
    }

    #[test]
    fn test_infer_dominant_type_null_values() {
        let features = [
            make_test_feature(serde_json::json!({"k": null})),
            make_test_feature(serde_json::json!({"k": "present"})),
        ];
        let refs: Vec<&mlt_core::geojson::Feature> = features.iter().collect();
        let dt = infer_dominant_type(&refs, "k");
        assert!(
            matches!(dt, DominantType::String),
            "Expected String when one value is null, got: {dt:?}"
        );
    }

    // -------------------------------------------------------------------------
    // json_to_prop_value tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_json_to_prop_value_string() {
        let val = serde_json::json!("hello");
        let pv = json_to_prop_value(Some(&val), DominantType::String);
        assert!(
            matches!(pv, mlt_core::PropValue::Str(Some(ref s)) if s == "hello"),
            "Expected Str(Some(\"hello\")), got: {pv:?}"
        );
    }

    #[test]
    fn test_json_to_prop_value_int() {
        let val = serde_json::json!(42);
        let pv = json_to_prop_value(Some(&val), DominantType::Int);
        assert!(
            matches!(pv, mlt_core::PropValue::I64(Some(42))),
            "Expected I64(Some(42)), got: {pv:?}"
        );
    }

    #[test]
    fn test_json_to_prop_value_bool() {
        let val = serde_json::json!(true);
        let pv = json_to_prop_value(Some(&val), DominantType::Bool);
        assert!(
            matches!(pv, mlt_core::PropValue::Bool(Some(true))),
            "Expected Bool(Some(true)), got: {pv:?}"
        );
    }

    #[test]
    fn test_json_to_prop_value_float() {
        let val = serde_json::json!(1.23);
        let pv = json_to_prop_value(Some(&val), DominantType::Float);
        assert!(
            matches!(pv, mlt_core::PropValue::F64(Some(v)) if (v - 1.23).abs() < f64::EPSILON),
            "Expected F64(Some(1.23)), got: {pv:?}"
        );
    }

    #[test]
    fn test_json_to_prop_value_null_returns_none() {
        let pv = json_to_prop_value(None, DominantType::Int);
        assert!(
            matches!(pv, mlt_core::PropValue::I64(None)),
            "Expected I64(None) for null, got: {pv:?}"
        );
    }

    #[test]
    fn test_json_to_prop_value_int_as_float() {
        let val = serde_json::json!(10);
        let pv = json_to_prop_value(Some(&val), DominantType::Float);
        assert!(
            matches!(pv, mlt_core::PropValue::F64(Some(v)) if (v - 10.0).abs() < f64::EPSILON),
            "Expected F64(Some(10.0)), got: {pv:?}"
        );
    }

    #[test]
    fn test_json_to_prop_value_int_as_string() {
        let val = serde_json::json!(42);
        let pv = json_to_prop_value(Some(&val), DominantType::String);
        assert!(
            matches!(pv, mlt_core::PropValue::Str(Some(ref s)) if s == "42"),
            "Expected Str(Some(\"42\")), got: {pv:?}"
        );
    }

    // -------------------------------------------------------------------------
    // build_tile_layer tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_build_tile_layer_skips_internal_keys() {
        let features = [make_test_feature(serde_json::json!({
            "_layer": "internal",
            "_extent": 4096,
            "name": "visible"
        }))];
        let refs: Vec<&mlt_core::geojson::Feature> = features.iter().collect();
        let layer = build_tile_layer("test", &refs);
        // Should only have "name", not _layer or _extent
        assert_eq!(layer.property_names.len(), 1);
        assert_eq!(layer.property_names[0], "name");
    }

    #[test]
    fn test_build_tile_layer_no_properties() {
        let features = [make_test_feature(serde_json::json!({}))];
        let refs: Vec<&mlt_core::geojson::Feature> = features.iter().collect();
        let layer = build_tile_layer("test", &refs);
        assert!(layer.property_names.is_empty());
    }

    #[test]
    fn test_build_tile_layer_multiple_keys() {
        let features = [make_test_feature(
            serde_json::json!({"a": 1, "b": "two", "c": true}),
        )];
        let refs: Vec<&mlt_core::geojson::Feature> = features.iter().collect();
        let layer = build_tile_layer("test", &refs);
        assert_eq!(layer.property_names.len(), 3);
        // BTreeSet ordering: a, b, c
        assert_eq!(layer.property_names, vec!["a", "b", "c"]);
    }

    // -------------------------------------------------------------------------
    // MLT → MVT tests (Phase 3, existing path)
    // -------------------------------------------------------------------------

    #[test]
    fn test_mlt_to_mvt_from_valid_mlt() {
        // Create a valid MLT tile via MVT→MLT, then convert back
        let mvt_bytes = make_mvt_point_tile("phase3_test", 50, 75);
        let mlt_bytes = mvt_to_mlt(&mvt_bytes).unwrap();
        let mlt_tile = TileData {
            data: mlt_bytes,
            format: TileFormat::Mlt,
            compression: TileCompression::None,
        };
        let result = transcode_tile(&mlt_tile, TileFormat::Pbf);
        assert!(result.is_ok(), "MLT→MVT should succeed: {:?}", result.err());
        let mvt_tile = result.unwrap();
        assert_eq!(mvt_tile.format, TileFormat::Pbf);
        // Verify it's valid protobuf
        use prost::Message;
        let decoded = MvtProto::Tile::decode(mvt_tile.data.as_ref());
        assert!(decoded.is_ok());
    }

    #[test]
    fn test_mlt_to_mvt_invalid_input() {
        let tile = TileData {
            data: Bytes::from_static(b"not valid MLT"),
            format: TileFormat::Mlt,
            compression: TileCompression::None,
        };
        let result = transcode_tile(&tile, TileFormat::Pbf);
        assert!(result.is_err());
    }

    /// Exercises the `.decode_all()` error arm in `mlt_to_mvt`. We take a valid
    /// MLT tile and truncate the final byte so `parse_layers` returns structurally
    /// valid `Layer<Lazy>` objects but column decoding fails on the truncated body.
    #[test]
    fn test_mlt_to_mvt_corrupt_body_triggers_decode_error() {
        let mvt_bytes = make_mvt_point_tile("decode_err_test", 10, 20);
        let mut mlt_bytes = mvt_to_mlt(&mvt_bytes).unwrap().to_vec();
        assert!(mlt_bytes.len() > 10);

        let mut last_err: Option<TileServerError> = None;
        for trunc_from_end in 1..mlt_bytes.len().min(64) {
            let truncated = &mlt_bytes[..mlt_bytes.len() - trunc_from_end];
            if let Err(e) = mlt_to_mvt(truncated) {
                last_err = Some(e);
                break;
            }
        }
        assert!(
            matches!(
                last_err,
                Some(TileServerError::MltDecodeError(_)) | Some(TileServerError::MltEncodeError(_))
            ),
            "expected MltDecodeError or MltEncodeError from truncated MLT, got: {last_err:?}"
        );

        let last_idx = mlt_bytes.len() - 1;
        mlt_bytes[last_idx] = mlt_bytes[last_idx].wrapping_add(0x7f);
        let _ = mlt_to_mvt(&mlt_bytes);
    }

    // -------------------------------------------------------------------------
    // Property helpers for test fixtures
    // -------------------------------------------------------------------------

    /// Create a test Feature with given properties and a dummy point geometry.
    fn make_test_feature(properties: serde_json::Value) -> mlt_core::geojson::Feature {
        use std::collections::BTreeMap;
        let props: BTreeMap<String, serde_json::Value> = match properties {
            serde_json::Value::Object(map) => map.into_iter().collect(),
            _ => BTreeMap::new(),
        };
        mlt_core::geojson::Feature {
            geometry: geo_types::Geometry::Point(geo_types::Point::new(0, 0)),
            id: None,
            properties: props,
            ty: String::new(),
        }
    }

    // -------------------------------------------------------------------------
    // decompress_tile_data — error / unsupported-compression branches
    // -------------------------------------------------------------------------

    #[test]
    fn test_decompress_zstd_compression_unsupported() {
        // Zstd compression isn't supported by the transcoder's gzip-only decompressor.
        let tile = TileData {
            data: Bytes::from_static(b"any bytes"),
            format: TileFormat::Pbf,
            compression: TileCompression::Zstd,
        };
        let err = transcode_tile(&tile, TileFormat::Mlt).unwrap_err();
        match err {
            TileServerError::MltDecodeError(msg) => {
                assert!(msg.contains("Zstd"), "unexpected message: {msg}");
                assert!(msg.contains("not supported"), "unexpected message: {msg}");
            }
            e => panic!("expected MltDecodeError, got {e:?}"),
        }
    }

    #[test]
    fn test_decompress_brotli_compression_unsupported() {
        let tile = TileData {
            data: Bytes::from_static(b"any bytes"),
            format: TileFormat::Mlt,
            compression: TileCompression::Brotli,
        };
        let err = transcode_tile(&tile, TileFormat::Pbf).unwrap_err();
        assert!(matches!(err, TileServerError::MltDecodeError(_)));
    }

    #[test]
    fn test_decompress_malformed_gzip_returns_error() {
        // Not a valid gzip stream (missing magic header).
        let tile = TileData {
            data: Bytes::from_static(b"this is definitely not a gzip stream"),
            format: TileFormat::Pbf,
            compression: TileCompression::Gzip,
        };
        let err = transcode_tile(&tile, TileFormat::Mlt).unwrap_err();
        match err {
            TileServerError::MltDecodeError(msg) => {
                assert!(
                    msg.contains("gzip"),
                    "expected message to mention gzip, got: {msg}"
                );
            }
            e => panic!("expected MltDecodeError, got {e:?}"),
        }
    }

    #[test]
    fn test_decompress_empty_gzip_returns_error() {
        // Empty bytes with Gzip compression — GzDecoder should error reading the header.
        let tile = TileData {
            data: Bytes::new(),
            format: TileFormat::Pbf,
            compression: TileCompression::Gzip,
        };
        let err = transcode_tile(&tile, TileFormat::Mlt).unwrap_err();
        assert!(matches!(err, TileServerError::MltDecodeError(_)));
    }

    #[test]
    fn test_decompress_truncated_gzip_stream_returns_error() {
        // Build a valid gzip stream then chop the trailer to corrupt it.
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        let mvt_bytes = make_mvt_point_tile("trunc", 1, 2);
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&mvt_bytes).unwrap();
        let mut compressed = enc.finish().unwrap();
        // Drop the trailer (last 8 bytes = CRC32 + ISIZE) — leaves a half-stream.
        let truncated_len = compressed.len().saturating_sub(8);
        compressed.truncate(truncated_len);

        let tile = TileData {
            data: Bytes::from(compressed),
            format: TileFormat::Pbf,
            compression: TileCompression::Gzip,
        };
        let result = transcode_tile(&tile, TileFormat::Mlt);
        assert!(result.is_err(), "truncated gzip should fail to transcode");
    }

    // -------------------------------------------------------------------------
    // transcode_tile — same-format passthrough and unsupported-pair coverage
    // -------------------------------------------------------------------------

    #[test]
    fn test_transcode_mlt_to_mlt_is_passthrough() {
        // Same target format short-circuits before decompression — even compressed
        // input is returned unchanged.
        let tile = TileData {
            data: Bytes::from_static(b"\x1f\x8b not really gzipped"),
            format: TileFormat::Mlt,
            compression: TileCompression::Gzip,
        };
        let result = transcode_tile(&tile, TileFormat::Mlt).unwrap();
        assert_eq!(result.format, TileFormat::Mlt);
        assert_eq!(result.compression, TileCompression::Gzip);
        assert_eq!(result.data, tile.data);
    }

    #[test]
    fn test_transcode_mlt_to_png_unsupported() {
        let tile = TileData {
            data: Bytes::from_static(b"any"),
            format: TileFormat::Mlt,
            compression: TileCompression::None,
        };
        let err = transcode_tile(&tile, TileFormat::Png).unwrap_err();
        match err {
            TileServerError::TranscodeUnsupported { from, to } => {
                assert_eq!(from, "Mlt");
                assert_eq!(to, "Png");
            }
            e => panic!("expected TranscodeUnsupported, got {e:?}"),
        }
    }

    #[test]
    fn test_transcode_pbf_to_jpeg_unsupported() {
        let tile = TileData {
            data: Bytes::from_static(b"any"),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        let err = transcode_tile(&tile, TileFormat::Jpeg).unwrap_err();
        assert!(matches!(err, TileServerError::TranscodeUnsupported { .. }));
    }
}
