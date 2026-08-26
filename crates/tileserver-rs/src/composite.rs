//! Multi-source composite MVT endpoint (GitHub issue #601).
//!
//! Parses `+`-separated source ids, merges their MVT layers by appending
//! features when layer names collide (dictionary keys/values remapped into a
//! shared index space), and re-encodes as a single MVT PBF. Also builds a
//! composite TileJSON that unions member `vector_layers` and intersects zoom
//! ranges.
//!
//! Uses `geozero::mvt` (always compiled) as the MVT codec so composites work
//! regardless of the `mlt` feature flag.

use std::collections::HashMap;

use bytes::Bytes;
use geozero::mvt::{Message, Tile, tile};
use serde_json::Value;

use crate::error::{Result, TileServerError};
use crate::sources::{TileJson, TileMetadata};

/// Split `"a+b+c"` into `["a", "b", "c"]`, trimming whitespace and dropping
/// empty segments. Returns `None` when nothing survives (e.g. `"+"` or `""`).
#[must_use]
pub fn parse_composite_id(id: &str) -> Option<Vec<String>> {
    let parts: Vec<String> = id
        .split('+')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    if parts.is_empty() { None } else { Some(parts) }
}

/// True when the id looks like a composite (contains `+`).
#[must_use]
pub fn is_composite_id(id: &str) -> bool {
    id.contains('+')
}

/// Validate that every id resolves through `exists`. Empty input is an error.
pub fn validate_composite_source_ids<F>(ids: &[String], exists: F) -> Result<()>
where
    F: Fn(&str) -> bool,
{
    if ids.is_empty() {
        return Err(TileServerError::RenderError(
            "composite requires at least one source id".to_string(),
        ));
    }
    for id in ids {
        if !exists(id) {
            return Err(TileServerError::SourceNotFound(id.clone()));
        }
    }
    Ok(())
}

/// Decompress gzip-encoded tile data. Non-gzip input errors.
pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::with_capacity(data.len() * 4);
    decoder
        .read_to_end(&mut out)
        .map_err(|e| TileServerError::MltDecodeError(format!("composite gzip decode: {e}")))?;
    Ok(out)
}

/// Decode an MVT PBF into its layers.
pub fn decode_mvt_layers(raw: &[u8]) -> Result<Vec<tile::Layer>> {
    let tile = Tile::decode(raw)
        .map_err(|e| TileServerError::MltDecodeError(format!("composite MVT decode: {e}")))?;
    Ok(tile.layers)
}

/// Merge layers, appending features when names collide. Dictionary keys/values
/// from later layers are remapped into the accumulated index space so feature
/// tags keep pointing at the right key/value.
#[must_use]
pub fn merge_mvt_layers(layers: Vec<tile::Layer>) -> Tile {
    let mut grouped: HashMap<String, tile::Layer> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for layer in layers {
        if !grouped.contains_key(&layer.name) {
            order.push(layer.name.clone());
            grouped.insert(
                layer.name.clone(),
                tile::Layer {
                    version: layer.version,
                    name: layer.name.clone(),
                    features: Vec::new(),
                    keys: layer.keys.clone(),
                    values: layer.values.clone(),
                    extent: layer.extent,
                },
            );
        }
        append_layer_features(grouped.get_mut(&layer.name).expect("seeded"), &layer);
    }

    Tile {
        layers: order
            .into_iter()
            .filter_map(|name| grouped.remove(&name))
            .collect(),
    }
}

/// Append `src`'s features into `merged`, remapping each feature's key/value
/// tag indices into `merged`'s dictionary (deduping shared keys and values).
fn append_layer_features(merged: &mut tile::Layer, src: &tile::Layer) {
    let key_remap = remap_keys(merged, src);
    let value_remap = remap_values(merged, src);

    for feature in &src.features {
        let mut new_tags = Vec::with_capacity(feature.tags.len());
        for pair in feature.tags.as_chunks::<2>().0 {
            let (k, v) = (pair[0] as usize, pair[1] as usize);
            new_tags.push(*key_remap.get(k).unwrap_or(&pair[0]));
            new_tags.push(*value_remap.get(v).unwrap_or(&pair[1]));
        }
        merged.features.push(tile::Feature {
            id: feature.id,
            tags: new_tags,
            r#type: feature.r#type,
            geometry: feature.geometry.clone(),
        });
    }
}

/// Build `src.keys[i] -> merged.keys` index remap, extending `merged.keys`.
fn remap_keys(merged: &mut tile::Layer, src: &tile::Layer) -> Vec<u32> {
    let mut remap = Vec::with_capacity(src.keys.len());
    for k in &src.keys {
        let idx = match merged.keys.iter().position(|x| x == k) {
            Some(i) => i,
            None => {
                merged.keys.push(k.clone());
                merged.keys.len() - 1
            }
        };
        remap.push(idx as u32);
    }
    remap
}

/// Build `src.values[i] -> merged.values` index remap, extending `merged.values`.
fn remap_values(merged: &mut tile::Layer, src: &tile::Layer) -> Vec<u32> {
    let mut remap = Vec::with_capacity(src.values.len());
    for v in &src.values {
        let idx = match merged.values.iter().position(|x| x == v) {
            Some(i) => i,
            None => {
                merged.values.push(v.clone());
                merged.values.len() - 1
            }
        };
        remap.push(idx as u32);
    }
    remap
}

/// Encode a `Tile` back into MVT PBF bytes.
#[must_use]
pub fn encode_mvt_pbf(tile: &Tile) -> Bytes {
    Bytes::from(tile.encode_to_vec())
}

/// Build a minimal single-layer MVT PBF (one point feature per requested
/// count) for use by integration tests that cannot depend on `geozero`
/// directly. Hidden from docs — not part of the stable API.
#[doc(hidden)]
#[must_use]
pub fn encode_test_tile(layer_name: &str, feature_count: usize) -> Vec<u8> {
    let features = (0..feature_count)
        .map(|i| tile::Feature {
            id: Some(i as u64 + 1),
            tags: Vec::new(),
            r#type: Some(tile::GeomType::Point as i32),
            geometry: vec![9, 0, 0],
        })
        .collect();
    let tile = Tile {
        layers: vec![tile::Layer {
            version: 2,
            name: layer_name.to_string(),
            features,
            keys: Vec::new(),
            values: Vec::new(),
            extent: Some(4096),
        }],
    };
    tile.encode_to_vec()
}

/// Compose a composite TileJSON from member sources: union of `vector_layers`
/// (deduped by id, first wins, empty ids dropped), `minzoom = max(members)`,
/// `maxzoom = min(members)`, tiles URL under the composite id.
#[must_use]
pub fn composite_tilejson(
    composite_id: &str,
    sources: &[&TileMetadata],
    base_url: &str,
    key: Option<&str>,
) -> TileJson {
    let key_query = key
        .map(|k| format!("?key={}", urlencoding::encode(k)))
        .unwrap_or_default();
    let tile_url = format!("{base_url}/data/{composite_id}/{{z}}/{{x}}/{{y}}.pbf{key_query}");

    let minzoom = sources.iter().map(|m| m.minzoom).max().unwrap_or(0);
    let maxzoom = sources.iter().map(|m| m.maxzoom).min().unwrap_or(0);

    TileJson {
        tilejson: "3.0.0".to_string(),
        id: composite_id.to_string(),
        tiles: vec![tile_url],
        name: composite_id.to_string(),
        description: None,
        attribution: None,
        minzoom,
        maxzoom,
        bounds: None,
        center: None,
        vector_layers: merge_vector_layers(sources),
        encoding: None,
    }
}

/// Union member `vector_layers` deduped by `id` (first wins, empty ids
/// dropped). Returns `None` only when no member contributes any layer.
fn merge_vector_layers(sources: &[&TileMetadata]) -> Option<Value> {
    let mut seen = std::collections::HashSet::new();
    let mut merged: Vec<Value> = Vec::new();
    for meta in sources {
        let Some(Value::Array(entries)) = meta.vector_layers.as_ref() else {
            continue;
        };
        for entry in entries {
            let id = entry.get("id").and_then(Value::as_str).unwrap_or("");
            if id.is_empty() || !seen.insert(id.to_string()) {
                continue;
            }
            merged.push(entry.clone());
        }
    }
    if merged.is_empty() {
        None
    } else {
        Some(Value::Array(merged))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value_string(s: &str) -> tile::Value {
        tile::Value {
            string_value: Some(s.to_string()),
            ..Default::default()
        }
    }

    fn value_int(n: i64) -> tile::Value {
        tile::Value {
            int_value: Some(n),
            ..Default::default()
        }
    }

    fn feature(tags: Vec<u32>) -> tile::Feature {
        tile::Feature {
            id: Some(1),
            tags,
            r#type: Some(tile::GeomType::Point as i32),
            geometry: vec![9, 0, 0],
        }
    }

    fn layer(
        name: &str,
        keys: Vec<&str>,
        values: Vec<tile::Value>,
        features: Vec<tile::Feature>,
    ) -> tile::Layer {
        tile::Layer {
            version: 2,
            name: name.to_string(),
            features,
            keys: keys.into_iter().map(String::from).collect(),
            values,
            extent: Some(4096),
        }
    }

    fn meta(id: &str, minzoom: u8, maxzoom: u8, vl: Option<Value>) -> TileMetadata {
        TileMetadata {
            id: id.to_string(),
            name: id.to_string(),
            description: None,
            attribution: None,
            format: crate::sources::TileFormat::Pbf,
            minzoom,
            maxzoom,
            bounds: None,
            center: None,
            vector_layers: vl,
        }
    }

    // --- parse_composite_id ---

    #[test]
    fn parse_composite_id_single() {
        assert_eq!(parse_composite_id("a"), Some(vec!["a".to_string()]));
    }

    #[test]
    fn parse_composite_id_two() {
        assert_eq!(
            parse_composite_id("a+b"),
            Some(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn parse_composite_id_three() {
        assert_eq!(
            parse_composite_id("a+b+c"),
            Some(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    #[test]
    fn parse_composite_id_trims_empty_segments() {
        assert_eq!(
            parse_composite_id("a++b"),
            Some(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn parse_composite_id_rejects_empty() {
        assert_eq!(parse_composite_id(""), None);
    }

    #[test]
    fn parse_composite_id_rejects_only_plus() {
        assert_eq!(parse_composite_id("+"), None);
    }

    #[test]
    fn parse_composite_id_trailing_plus_drops_empty() {
        assert_eq!(parse_composite_id("a+"), Some(vec!["a".to_string()]));
    }

    // --- is_composite_id ---

    #[test]
    fn is_composite_id_basic() {
        assert!(is_composite_id("a+b"));
    }

    #[test]
    fn is_composite_id_no_plus_is_false() {
        assert!(!is_composite_id("solo"));
    }

    #[test]
    fn is_composite_id_only_plus_is_true() {
        assert!(is_composite_id("+"));
    }

    // --- validate_composite_source_ids ---

    #[test]
    fn validate_composite_source_ids_all_present_ok() {
        let ids = vec!["a".to_string(), "b".to_string()];
        assert!(validate_composite_source_ids(&ids, |_| true).is_ok());
    }

    #[test]
    fn validate_composite_source_ids_missing_returns_err() {
        let ids = vec!["a".to_string(), "b".to_string()];
        let r = validate_composite_source_ids(&ids, |id| id == "a");
        assert!(matches!(r, Err(TileServerError::SourceNotFound(_))));
    }

    #[test]
    fn validate_composite_source_ids_empty_returns_err() {
        let r = validate_composite_source_ids(&[], |_| true);
        assert!(r.is_err());
    }

    // --- decompress_gzip ---

    #[test]
    fn decompress_gzip_roundtrips_simple_payload() {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(b"hello composite").unwrap();
        let gz = enc.finish().unwrap();
        assert_eq!(decompress_gzip(&gz).unwrap(), b"hello composite");
    }

    #[test]
    fn decompress_gzip_invalid_data_returns_err() {
        assert!(decompress_gzip(&[0x00, 0x01, 0x02]).is_err());
    }

    // --- decode_mvt_layers ---

    #[test]
    fn decode_mvt_layers_empty_input_produces_empty_vec() {
        let empty = Tile::default();
        let raw = encode_mvt_pbf(&empty);
        assert!(decode_mvt_layers(&raw).unwrap().is_empty());
    }

    #[test]
    fn decode_mvt_layers_garbage_input_returns_err() {
        assert!(decode_mvt_layers(&[0xFF, 0xFF, 0xFF, 0xFF]).is_err());
    }

    #[test]
    fn decode_mvt_layers_single_layer_roundtrips() {
        let t = Tile {
            layers: vec![layer(
                "roads",
                vec!["name"],
                vec![value_string("main")],
                vec![feature(vec![0, 0])],
            )],
        };
        let raw = encode_mvt_pbf(&t);
        let decoded = decode_mvt_layers(&raw).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].name, "roads");
    }

    #[test]
    fn decode_mvt_layers_multi_layer_roundtrips() {
        let t = Tile {
            layers: vec![
                layer("roads", vec![], vec![], vec![]),
                layer("water", vec![], vec![], vec![]),
            ],
        };
        let raw = encode_mvt_pbf(&t);
        assert_eq!(decode_mvt_layers(&raw).unwrap().len(), 2);
    }

    // --- merge_mvt_layers ---

    #[test]
    fn merge_mvt_layers_empty_input_produces_empty_tile() {
        assert!(merge_mvt_layers(vec![]).layers.is_empty());
    }

    #[test]
    fn merge_mvt_layers_single_layer_passes_through() {
        let t = merge_mvt_layers(vec![layer("roads", vec![], vec![], vec![feature(vec![])])]);
        assert_eq!(t.layers.len(), 1);
        assert_eq!(t.layers[0].features.len(), 1);
    }

    #[test]
    fn merge_mvt_layers_distinct_layer_names_are_preserved() {
        let t = merge_mvt_layers(vec![
            layer("roads", vec![], vec![], vec![]),
            layer("water", vec![], vec![], vec![]),
        ]);
        let names: Vec<&str> = t.layers.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, vec!["roads", "water"]);
    }

    #[test]
    fn merge_mvt_layers_collision_appends_features_from_second_layer() {
        let a = layer(
            "roads",
            vec![],
            vec![],
            vec![feature(vec![]), feature(vec![])],
        );
        let b = layer("roads", vec![], vec![], vec![feature(vec![])]);
        let t = merge_mvt_layers(vec![a, b]);
        assert_eq!(t.layers.len(), 1);
        assert_eq!(t.layers[0].features.len(), 3);
    }

    #[test]
    fn merge_mvt_layers_collision_reuses_first_layers_keys_values_when_disjoint() {
        let a = layer(
            "roads",
            vec!["name"],
            vec![value_string("main")],
            vec![feature(vec![0, 0])],
        );
        let b = layer(
            "roads",
            vec!["height"],
            vec![value_int(5)],
            vec![feature(vec![0, 0])],
        );
        let t = merge_mvt_layers(vec![a, b]);
        let merged = &t.layers[0];
        assert_eq!(merged.keys, vec!["name", "height"]);
        assert_eq!(merged.values.len(), 2);
        // second feature's tags must be remapped to (1,1): height key idx 1, int value idx 1
        assert_eq!(merged.features[1].tags, vec![1, 1]);
    }

    #[test]
    fn merge_mvt_layers_collision_deduplicates_overlapping_keys() {
        let a = layer(
            "roads",
            vec!["name", "type"],
            vec![value_string("main"), value_string("primary")],
            vec![feature(vec![0, 0])],
        );
        let b = layer(
            "roads",
            vec!["name", "height"],
            vec![value_string("side"), value_int(9)],
            vec![feature(vec![0, 0, 1, 1])],
        );
        let t = merge_mvt_layers(vec![a, b]);
        let merged = &t.layers[0];
        assert_eq!(merged.keys, vec!["name", "type", "height"]);
        // b feature referenced its key 0 ("name") -> merged key 0; key 1 ("height") -> merged key 2
        assert_eq!(merged.features[1].tags[0], 0);
        assert_eq!(merged.features[1].tags[2], 2);
    }

    #[test]
    fn merge_mvt_layers_collision_dedups_equal_values() {
        let a = layer(
            "roads",
            vec!["name"],
            vec![value_string("shared")],
            vec![feature(vec![0, 0])],
        );
        let b = layer(
            "roads",
            vec!["label"],
            vec![value_string("shared")],
            vec![feature(vec![0, 0])],
        );
        let t = merge_mvt_layers(vec![a, b]);
        let merged = &t.layers[0];
        // "shared" value should be deduped to a single entry
        assert_eq!(merged.values.len(), 1);
        // b's value tag must point at the reused value index 0
        assert_eq!(merged.features[1].tags[1], 0);
    }

    #[test]
    fn merge_mvt_layers_preserves_extent_from_first_collision_winner() {
        let mut a = layer("roads", vec![], vec![], vec![]);
        a.extent = Some(8192);
        let mut b = layer("roads", vec![], vec![], vec![]);
        b.extent = Some(4096);
        let t = merge_mvt_layers(vec![a, b]);
        assert_eq!(t.layers[0].extent, Some(8192));
    }

    #[test]
    fn encode_mvt_pbf_roundtrips_through_decode() {
        let t = Tile {
            layers: vec![layer(
                "roads",
                vec!["k"],
                vec![value_string("v")],
                vec![feature(vec![0, 0])],
            )],
        };
        let raw = encode_mvt_pbf(&t);
        let decoded = decode_mvt_layers(&raw).unwrap();
        assert_eq!(decoded[0].name, "roads");
        assert_eq!(decoded[0].keys, vec!["k"]);
    }

    // --- composite_tilejson ---

    #[test]
    fn composite_tilejson_single_source_passes_through_minmax() {
        let a = meta("a", 3, 12, None);
        let tj = composite_tilejson("a", &[&a], "http://h", None);
        assert_eq!(tj.minzoom, 3);
        assert_eq!(tj.maxzoom, 12);
    }

    #[test]
    fn composite_tilejson_minzoom_is_max_of_sources() {
        let a = meta("a", 0, 14, None);
        let b = meta("b", 5, 14, None);
        let tj = composite_tilejson("a+b", &[&a, &b], "http://h", None);
        assert_eq!(tj.minzoom, 5);
    }

    #[test]
    fn composite_tilejson_maxzoom_is_min_of_sources() {
        let a = meta("a", 0, 14, None);
        let b = meta("b", 0, 10, None);
        let tj = composite_tilejson("a+b", &[&a, &b], "http://h", None);
        assert_eq!(tj.maxzoom, 10);
    }

    #[test]
    fn composite_tilejson_url_includes_composite_id() {
        let a = meta("a", 0, 14, None);
        let b = meta("b", 0, 14, None);
        let tj = composite_tilejson("a+b", &[&a, &b], "http://h", None);
        assert!(tj.tiles[0].contains("/data/a+b/"), "got {}", tj.tiles[0]);
        assert!(
            tj.tiles[0].ends_with("/{z}/{x}/{y}.pbf"),
            "got {}",
            tj.tiles[0]
        );
    }

    #[test]
    fn composite_tilejson_includes_key_query_when_provided() {
        let a = meta("a", 0, 14, None);
        let tj = composite_tilejson("a", &[&a], "http://h", Some("xyz"));
        assert!(tj.tiles[0].contains("?key=xyz"), "got {}", tj.tiles[0]);
    }

    #[test]
    fn composite_tilejson_id_is_composite_id() {
        let a = meta("a", 0, 14, None);
        let tj = composite_tilejson("a+b", &[&a], "http://h", None);
        assert_eq!(tj.id, "a+b");
    }

    #[test]
    fn composite_tilejson_vector_layers_are_merged() {
        let a = meta("a", 0, 14, Some(serde_json::json!([{ "id": "roads" }])));
        let b = meta("b", 0, 14, Some(serde_json::json!([{ "id": "water" }])));
        let tj = composite_tilejson("a+b", &[&a, &b], "http://h", None);
        let vl = tj.vector_layers.unwrap();
        let arr = vl.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn composite_tilejson_drops_vector_layers_with_empty_id() {
        let a = meta(
            "a",
            0,
            14,
            Some(serde_json::json!([{ "id": "" }, { "id": "roads" }])),
        );
        let tj = composite_tilejson("a", &[&a], "http://h", None);
        let arr = tj.vector_layers.unwrap();
        let arr = arr.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "roads");
    }

    #[test]
    fn composite_tilejson_dedupes_by_layer_id_first_wins() {
        let a = meta(
            "a",
            0,
            14,
            Some(serde_json::json!([{ "id": "roads", "src": "a" }])),
        );
        let b = meta(
            "b",
            0,
            14,
            Some(serde_json::json!([{ "id": "roads", "src": "b" }])),
        );
        let tj = composite_tilejson("a+b", &[&a, &b], "http://h", None);
        let arr = tj.vector_layers.unwrap();
        let arr = arr.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["src"], "a");
    }

    #[test]
    fn composite_tilejson_handles_none_vector_layers() {
        let a = meta("a", 0, 14, None);
        let b = meta("b", 0, 14, Some(serde_json::json!([{ "id": "water" }])));
        let tj = composite_tilejson("a+b", &[&a, &b], "http://h", None);
        let arr = tj.vector_layers.unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 1);
    }
}
