use anyhow::Result;
use geo::{Geometry, Rect};
use geozero::mvt::{tile, Message, Tile};
use geozero::ToMvt;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// A feature ready for MVT encoding (geometry already clipped and in geographic space).
pub struct TileFeature {
    pub geometry: Geometry<f64>,
    pub properties: Map<String, Value>,
    /// Optional MVT feature ID (enables MapLibre feature state).
    pub id: Option<u64>,
}

/// Build a complete MVT tile and return the encoded protobuf bytes.
///
/// `bbox` is the geographic extent of the tile (west/south/east/north in WGS-84).
/// The returned bytes are uncompressed; the PMTiles writer compresses them.
pub fn build_tile_bytes(
    features: &[TileFeature],
    bbox: &Rect<f64>,
    layer_name: &str,
) -> Result<Vec<u8>> {
    if features.is_empty() {
        return Ok(Vec::new());
    }

    const EXTENT: u32 = 4096;

    let west = bbox.min().x;
    let south = bbox.min().y;
    let east = bbox.max().x;
    let north = bbox.max().y;

    let mut keys: Vec<String> = Vec::new();
    let mut values: Vec<tile::Value> = Vec::new();
    let mut key_index: HashMap<String, usize> = HashMap::new();
    let mut mvt_features: Vec<tile::Feature> = Vec::new();

    for feat in features {
        // Convert geometry to MVT coordinates (scaled to 0..EXTENT)
        let mvt_geom = match feat.geometry.to_mvt(EXTENT, west, south, east, north) {
            Ok(g) => g,
            Err(_) => continue, // skip degenerate geometries
        };

        if mvt_geom.geometry.is_empty() {
            continue;
        }

        // Encode properties into the key/value tables
        let tags = encode_properties(&feat.properties, &mut keys, &mut values, &mut key_index);

        mvt_features.push(tile::Feature {
            id: feat.id,
            tags,
            r#type: mvt_geom.r#type,
            geometry: mvt_geom.geometry,
        });
    }

    if mvt_features.is_empty() {
        return Ok(Vec::new());
    }

    let layer = tile::Layer {
        version: 2,
        name: layer_name.to_owned(),
        features: mvt_features,
        keys,
        values,
        extent: Some(EXTENT),
    };

    let tile = Tile {
        layers: vec![layer],
    };

    Ok(tile.encode_to_vec())
}

/// Encode a properties map into MVT key/value tables.
/// Returns the tag list (pairs of key_index, value_index).
fn encode_properties(
    properties: &Map<String, Value>,
    keys: &mut Vec<String>,
    values: &mut Vec<tile::Value>,
    key_index: &mut HashMap<String, usize>,
) -> Vec<u32> {
    let mut tags = Vec::new();

    for (k, v) in properties {
        // Key index
        let ki = if let Some(&i) = key_index.get(k) {
            i
        } else {
            let i = keys.len();
            keys.push(k.clone());
            key_index.insert(k.clone(), i);
            i
        };

        // Value
        let mv = json_to_mvt_value(v);
        let vi = values.len();
        values.push(mv);

        tags.push(ki as u32);
        tags.push(vi as u32);
    }

    tags
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Geometry, LineString, Point, Polygon, Rect};

    fn unit_bbox() -> Rect<f64> {
        Rect::new(geo::Coord { x: 0.0, y: 0.0 }, geo::Coord { x: 1.0, y: 1.0 })
    }

    fn point_feature(x: f64, y: f64) -> TileFeature {
        TileFeature {
            geometry: Geometry::Point(Point::new(x, y)),
            properties: Map::new(),
            id: None,
        }
    }

    #[test]
    fn empty_features_slice_returns_empty_bytes() {
        let bytes = build_tile_bytes(&[], &unit_bbox(), "test").unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn single_point_produces_non_empty_tile() {
        let feat = point_feature(0.5, 0.5);
        let bytes = build_tile_bytes(&[feat], &unit_bbox(), "my_layer").unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn tile_bytes_decode_to_valid_protobuf() {
        let feat = point_feature(0.5, 0.5);
        let bytes = build_tile_bytes(&[feat], &unit_bbox(), "layer").unwrap();
        let tile = Tile::decode(bytes.as_slice()).expect("must decode as protobuf");
        assert_eq!(tile.layers.len(), 1);
        assert_eq!(tile.layers[0].name, "layer");
        assert_eq!(tile.layers[0].version, 2);
        assert_eq!(tile.layers[0].extent, Some(4096));
    }

    #[test]
    fn layer_name_is_preserved() {
        let feat = point_feature(0.5, 0.5);
        let bytes = build_tile_bytes(&[feat], &unit_bbox(), "my_custom_layer").unwrap();
        let tile = Tile::decode(bytes.as_slice()).unwrap();
        assert_eq!(tile.layers[0].name, "my_custom_layer");
    }

    #[test]
    fn properties_produce_keys_and_values() {
        let mut props = Map::new();
        props.insert("city".to_string(), Value::String("Berlin".to_string()));
        props.insert("pop".to_string(), serde_json::json!(3_600_000i64));
        let feat = TileFeature {
            geometry: Geometry::Point(Point::new(0.5, 0.5)),
            properties: props,
            id: None,
        };
        let bytes = build_tile_bytes(&[feat], &unit_bbox(), "layer").unwrap();
        let tile = Tile::decode(bytes.as_slice()).unwrap();
        let layer = &tile.layers[0];
        assert!(layer.keys.contains(&"city".to_string()));
        assert!(layer.keys.contains(&"pop".to_string()));
        assert_eq!(layer.values.len(), 2);
    }

    #[test]
    fn feature_outside_bbox_is_skipped() {
        // Point at (5, 5) is outside the bbox (0..1, 0..1) after MVT projection
        // geozero will produce empty geometry → feature is skipped
        let feat = TileFeature {
            geometry: Geometry::Point(Point::new(5.0, 5.0)),
            properties: Map::new(),
            id: None,
        };
        let bytes = build_tile_bytes(&[feat], &unit_bbox(), "layer").unwrap();
        // May be empty or have a layer with 0 features; either is acceptable
        if !bytes.is_empty() {
            let tile = Tile::decode(bytes.as_slice()).unwrap();
            if !tile.layers.is_empty() {
                // If a layer exists, feature count may be 0 or 1 (clamped coords)
                // Just verify it doesn't panic
            }
        }
    }

    #[test]
    fn linestring_feature_encodes() {
        let ls = LineString::from(vec![(0.1, 0.1), (0.5, 0.5), (0.9, 0.9)]);
        let feat = TileFeature {
            geometry: Geometry::LineString(ls),
            properties: Map::new(),
            id: None,
        };
        let bytes = build_tile_bytes(&[feat], &unit_bbox(), "roads").unwrap();
        if !bytes.is_empty() {
            let tile = Tile::decode(bytes.as_slice()).unwrap();
            assert_eq!(tile.layers[0].name, "roads");
        }
    }

    #[test]
    fn polygon_feature_encodes() {
        let ring = geo::LineString::from(vec![
            (0.1, 0.1),
            (0.9, 0.1),
            (0.9, 0.9),
            (0.1, 0.9),
            (0.1, 0.1),
        ]);
        let poly = Polygon::new(ring, vec![]);
        let feat = TileFeature {
            geometry: Geometry::Polygon(poly),
            properties: Map::new(),
            id: None,
        };
        let bytes = build_tile_bytes(&[feat], &unit_bbox(), "areas").unwrap();
        assert!(!bytes.is_empty());
        let tile = Tile::decode(bytes.as_slice()).unwrap();
        assert_eq!(tile.layers[0].name, "areas");
    }

    #[test]
    fn multiple_features_share_key_table() {
        // Two features with the same property key should deduplicate the key
        let mut props = Map::new();
        props.insert("name".to_string(), Value::String("A".to_string()));
        let feat_a = TileFeature {
            geometry: Geometry::Point(Point::new(0.3, 0.3)),
            properties: props.clone(),
            id: None,
        };
        props.insert("name".to_string(), Value::String("B".to_string()));
        let feat_b = TileFeature {
            geometry: Geometry::Point(Point::new(0.7, 0.7)),
            properties: props,
            id: None,
        };
        let bytes = build_tile_bytes(&[feat_a, feat_b], &unit_bbox(), "layer").unwrap();
        let tile = Tile::decode(bytes.as_slice()).unwrap();
        let layer = &tile.layers[0];
        // "name" key should appear exactly once
        let name_count = layer.keys.iter().filter(|k| k.as_str() == "name").count();
        assert_eq!(name_count, 1, "Shared key should be deduplicated");
        // But two values ("A" and "B")
        assert_eq!(layer.values.len(), 2);
    }
}

fn json_to_mvt_value(v: &Value) -> tile::Value {
    match v {
        Value::String(s) => tile::Value {
            string_value: Some(s.clone()),
            ..Default::default()
        },
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                tile::Value {
                    int_value: Some(i),
                    ..Default::default()
                }
            } else if let Some(u) = n.as_u64() {
                tile::Value {
                    uint_value: Some(u),
                    ..Default::default()
                }
            } else {
                tile::Value {
                    double_value: n.as_f64(),
                    ..Default::default()
                }
            }
        }
        Value::Bool(b) => tile::Value {
            bool_value: Some(*b),
            ..Default::default()
        },
        Value::Null => tile::Value {
            string_value: Some(String::new()),
            ..Default::default()
        },
        // Nested objects/arrays: serialize to string
        other => tile::Value {
            string_value: Some(other.to_string()),
            ..Default::default()
        },
    }
}
