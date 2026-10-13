//! Per-feature CEL filtering of vector tiles (issue #1010).
//!
//! A [`CelFilter`] holds a compiled [CEL](https://github.com/google/cel-spec)
//! boolean expression. [`apply`] decodes an MVT tile, keeps only the features
//! for which every filter evaluates to `true`, drops layers that became empty,
//! and re-encodes the tile. Feature properties are bound as top-level CEL
//! variables, so `type == 'residential'` reads the feature's `type` property.
//!
//! Filters come from two places and compose with logical AND:
//!
//! - the source preset (`[sources.filter]`, or `filter` on a PostGIS table or
//!   function), applied by the source manager to every read of that source;
//! - the request (`?filter=` on the tile route), applied per request and
//!   cached under its own [`CelFilter::hash`].
//!
//! # Sandbox
//!
//! Only the functions in [`ALLOWED_FUNCTIONS`] may appear in an expression;
//! anything else is rejected at compile time. The allowlist holds operators,
//! string predicates and numeric conversions only, so no expression can read
//! the filesystem, the network, the process environment or the clock.
//! Expression length is capped at [`MAX_EXPRESSION_LEN`] and the parser caps
//! nesting depth, which bounds evaluation cost per feature.

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, LazyLock, Mutex};

use bytes::Bytes;
use cel::common::ast::operators;
use cel::{Context, Program, Value};
use geozero::mvt::{Message, Tile, tile};

use crate::config::CelFilterConfig;
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat};

/// Longest accepted expression, in bytes.
pub const MAX_EXPRESSION_LEN: usize = 4096;

/// Compiled programs kept in the process-wide compile cache. The cache is
/// cleared when it reaches this size.
const COMPILE_CACHE_CAPACITY: usize = 1024;

/// Every function or operator an expression may reference.
///
/// This is the sandbox boundary: an expression that references any other
/// function fails to compile. Keep it free of I/O, environment and clock
/// functions (`timestamp`, `duration`, `dyn`, `type` are deliberately absent).
pub const ALLOWED_FUNCTIONS: &[&str] = &[
    operators::CONDITIONAL,
    operators::LOGICAL_AND,
    operators::LOGICAL_OR,
    operators::LOGICAL_NOT,
    operators::SUBSTRACT,
    operators::ADD,
    operators::MULTIPLY,
    operators::DIVIDE,
    operators::MODULO,
    operators::EQUALS,
    operators::NOT_EQUALS,
    operators::GREATER_EQUALS,
    operators::LESS_EQUALS,
    operators::GREATER,
    operators::LESS,
    operators::NEGATE,
    operators::INDEX,
    operators::IN,
    operators::NOT_STRICTLY_FALSE,
    operators::HAS,
    operators::ALL,
    operators::EXISTS,
    operators::EXISTS_ONE,
    operators::MAP,
    operators::FILTER,
    "size",
    "contains",
    "startsWith",
    "endsWith",
    "matches",
    "int",
    "uint",
    "double",
    "string",
];

static COMPILED: LazyLock<Mutex<HashMap<String, Arc<Program>>>> =
    LazyLock::new(|| Mutex::new(HashMap::with_capacity(64)));

/// A compiled, sandbox-checked CEL filter.
#[derive(Debug, Clone)]
pub struct CelFilter {
    program: Arc<Program>,
    hash: u64,
    layers: Option<Arc<HashSet<String>>>,
}

impl CelFilter {
    /// Compile a request expression. Applies to every layer.
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::InvalidFilter`] when the expression is empty,
    /// too long, does not parse (the message carries line and column), or uses
    /// a function outside [`ALLOWED_FUNCTIONS`].
    pub fn compile(expression: &str) -> Result<Self> {
        let expression = expression.trim();
        Ok(Self {
            program: compile_cached(expression)?,
            hash: hash_expression(expression, None),
            layers: None,
        })
    }

    /// Compile a source preset, honouring `applies_to_layers`.
    ///
    /// # Errors
    ///
    /// Same as [`CelFilter::compile`].
    pub fn from_config(config: &CelFilterConfig) -> Result<Self> {
        let expression = config.expression.trim();
        let layers = config
            .applies_to_layers
            .as_ref()
            .map(|names| Arc::new(names.iter().cloned().collect::<HashSet<_>>()));
        Ok(Self {
            program: compile_cached(expression)?,
            hash: hash_expression(expression, config.applies_to_layers.as_deref()),
            layers,
        })
    }

    /// Stable, non-zero hash of the canonical expression (and layer scope).
    /// Used as the `filter` part of the tile cache key.
    #[must_use]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    fn applies_to(&self, layer: &str) -> bool {
        self.layers
            .as_ref()
            .is_none_or(|names| names.contains(layer))
    }

    /// Whether `feature` (a member of `layer`) passes this filter. An
    /// evaluation error (unknown property, type mismatch) or a non-bool
    /// result drops the feature.
    fn matches(
        &self,
        root: &Context<'_, '_>,
        layer: &tile::Layer,
        feature: &tile::Feature,
    ) -> bool {
        let mut scope = root.new_inner_scope();
        let (pairs, _) = feature.tags.as_chunks::<2>();
        for [key_index, value_index] in pairs {
            let key = layer.keys.get(*key_index as usize);
            let value = layer.values.get(*value_index as usize).and_then(mvt_value);
            if let (Some(key), Some(value)) = (key, value) {
                scope.add_variable_from_value(key.as_str(), value);
            }
        }
        matches!(self.program.execute(&scope), Ok(Value::Bool(true)))
    }
}

/// Combine several filter hashes into one cache-key component. `0` means
/// "no filter".
#[must_use]
pub fn combined_hash(filters: &[&CelFilter]) -> u64 {
    match filters {
        [] => 0,
        [one] => one.hash,
        many => {
            let mut hasher = DefaultHasher::new();
            for filter in many {
                filter.hash.hash(&mut hasher);
            }
            hasher.finish().max(1)
        }
    }
}

/// Filter `tile` with every filter (logical AND).
///
/// Returns the tile unchanged when `filters` is empty. Otherwise the result is
/// an uncompressed MVT (`Pbf`) tile; its data is empty when no feature passed.
/// MLT tiles are transcoded to MVT first.
///
/// # Errors
///
/// [`TileServerError::InvalidFilter`] for a raster tile (or an MLT tile when
/// the `mlt` feature is off); a decode error when the tile is not valid MVT.
pub fn apply(tile: TileData, filters: &[&CelFilter]) -> Result<TileData> {
    if filters.is_empty() {
        return Ok(tile);
    }
    if !tile.format.is_vector() {
        return Err(TileServerError::InvalidFilter(
            "CEL filters apply only to vector tiles".to_string(),
        ));
    }
    let tile = to_pbf(tile)?;
    let raw = crate::compression::decode(&tile.data, tile.compression)?;
    Ok(TileData {
        data: Bytes::from(filter_mvt(&raw, filters)?),
        format: TileFormat::Pbf,
        compression: TileCompression::None,
    })
}

#[cfg(feature = "mlt")]
fn to_pbf(tile: TileData) -> Result<TileData> {
    if tile.format == TileFormat::Mlt {
        return crate::transcode::transcode_tile(&tile, TileFormat::Pbf);
    }
    Ok(tile)
}

#[cfg(not(feature = "mlt"))]
fn to_pbf(tile: TileData) -> Result<TileData> {
    if tile.format == TileFormat::Mlt {
        return Err(TileServerError::InvalidFilter(
            "filtering MLT tiles needs the `mlt` cargo feature".to_string(),
        ));
    }
    Ok(tile)
}

/// Filter raw (uncompressed) MVT bytes. Returns empty bytes when every
/// filtered layer ended up empty and no untouched layer remains.
fn filter_mvt(raw: &[u8], filters: &[&CelFilter]) -> Result<Vec<u8>> {
    let mut tile = Tile::decode(raw).map_err(|e| {
        TileServerError::RenderError(format!("failed to decode MVT tile for filtering: {e}"))
    })?;
    let root = Context::default();

    tile.layers.retain_mut(|layer| {
        let active: Vec<&CelFilter> = filters
            .iter()
            .copied()
            .filter(|f| f.applies_to(&layer.name))
            .collect();
        if active.is_empty() {
            return true;
        }
        let features = std::mem::take(&mut layer.features);
        let view: &tile::Layer = layer;
        let kept: Vec<tile::Feature> = features
            .into_iter()
            .filter(|feature| active.iter().all(|f| f.matches(&root, view, feature)))
            .collect();
        layer.features = kept;
        !layer.features.is_empty()
    });

    if tile.layers.is_empty() {
        return Ok(Vec::new());
    }
    Ok(tile.encode_to_vec())
}

/// Convert an MVT property value to a CEL value. `uint` values that fit are
/// widened to `int` so `height > 50` compares like-for-like.
fn mvt_value(value: &tile::Value) -> Option<Value> {
    if let Some(s) = &value.string_value {
        return Some(Value::String(Arc::new(s.clone())));
    }
    if let Some(b) = value.bool_value {
        return Some(Value::Bool(b));
    }
    if let Some(i) = value.int_value.or(value.sint_value) {
        return Some(Value::Int(i));
    }
    if let Some(u) = value.uint_value {
        return Some(i64::try_from(u).map_or(Value::UInt(u), Value::Int));
    }
    if let Some(d) = value.double_value {
        return Some(Value::Float(d));
    }
    value.float_value.map(|f| Value::Float(f64::from(f)))
}

fn hash_expression(expression: &str, layers: Option<&[String]>) -> u64 {
    let mut hasher = DefaultHasher::new();
    expression.hash(&mut hasher);
    if let Some(layers) = layers {
        let mut sorted: Vec<&String> = layers.iter().collect();
        sorted.sort();
        sorted.hash(&mut hasher);
    }
    hasher.finish().max(1)
}

fn compile_cached(expression: &str) -> Result<Arc<Program>> {
    if expression.is_empty() {
        return Err(TileServerError::InvalidFilter(
            "filter expression is empty".to_string(),
        ));
    }
    if expression.len() > MAX_EXPRESSION_LEN {
        return Err(TileServerError::InvalidFilter(format!(
            "filter expression is longer than {MAX_EXPRESSION_LEN} bytes"
        )));
    }

    let mut cache = COMPILED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(program) = cache.get(expression) {
        crate::metrics::cel_compile_recorded(true);
        return Ok(Arc::clone(program));
    }
    crate::metrics::cel_compile_recorded(false);

    let program =
        Program::compile(expression).map_err(|e| TileServerError::InvalidFilter(e.to_string()))?;
    let references = program.references();
    let mut rejected: Vec<&str> = references
        .functions()
        .into_iter()
        .filter(|f| !ALLOWED_FUNCTIONS.contains(f))
        .collect();
    if !rejected.is_empty() {
        rejected.sort_unstable();
        return Err(TileServerError::InvalidFilter(format!(
            "function not allowed in filter: {}",
            rejected.join(", ")
        )));
    }

    if cache.len() >= COMPILE_CACHE_CAPACITY {
        cache.clear();
    }
    let program = Arc::new(program);
    cache.insert(expression.to_string(), Arc::clone(&program));
    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string(s: &str) -> tile::Value {
        tile::Value {
            string_value: Some(s.to_string()),
            ..Default::default()
        }
    }

    fn int(i: i64) -> tile::Value {
        tile::Value {
            int_value: Some(i),
            ..Default::default()
        }
    }

    /// Build a layer from `(properties)` rows. Every feature gets the same
    /// trivial point geometry; only properties matter to the filter.
    fn layer(name: &str, rows: &[&[(&str, tile::Value)]]) -> tile::Layer {
        let mut keys: Vec<String> = Vec::new();
        let mut values: Vec<tile::Value> = Vec::new();
        let mut features = Vec::with_capacity(rows.len());
        for (id, row) in rows.iter().enumerate() {
            let mut tags = Vec::with_capacity(row.len() * 2);
            for (key, value) in row.iter() {
                let k = keys.iter().position(|k| k == key).unwrap_or_else(|| {
                    keys.push((*key).to_string());
                    keys.len() - 1
                });
                values.push(value.clone());
                tags.push(u32::try_from(k).expect("key index"));
                tags.push(u32::try_from(values.len() - 1).expect("value index"));
            }
            features.push(tile::Feature {
                id: Some(id as u64),
                tags,
                r#type: Some(1),
                geometry: vec![9, 50, 34],
            });
        }
        tile::Layer {
            version: 2,
            name: name.to_string(),
            features,
            keys,
            values,
            extent: Some(4096),
        }
    }

    fn tile_data(layers: Vec<tile::Layer>) -> TileData {
        TileData {
            data: Bytes::from(Tile { layers }.encode_to_vec()),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        }
    }

    fn decoded(tile: &TileData) -> Tile {
        Tile::decode(tile.data.as_ref()).expect("valid MVT")
    }

    /// Feature ids kept per layer, in layer order.
    fn kept(tile: &TileData) -> Vec<(String, Vec<u64>)> {
        decoded(tile)
            .layers
            .into_iter()
            .map(|l| (l.name, l.features.iter().filter_map(|f| f.id).collect()))
            .collect()
    }

    fn landuse() -> tile::Layer {
        layer(
            "landuse",
            &[
                &[("type", string("residential")), ("height", int(10))],
                &[("type", string("commercial")), ("height", int(120))],
                &[("type", string("park")), ("height", int(0))],
                &[("type", string("residential")), ("height", int(80))],
            ],
        )
    }

    #[test]
    fn spec_examples_compile() {
        for expr in [
            "type == 'residential'",
            "height > 50 && levels >= 5",
            "name.startsWith('St ')",
            "class in ['motorway', 'trunk', 'primary']",
            "metadata.year_built > 2020",
            "type == 'building' && height > 100 && name.matches('.+ Tower$')",
        ] {
            CelFilter::compile(expr).unwrap_or_else(|e| panic!("{expr}: {e}"));
        }
    }

    #[test]
    fn invalid_expression_reports_position() {
        let err = CelFilter::compile("type == ").expect_err("must not compile");
        let msg = err.to_string();
        assert!(matches!(err, TileServerError::InvalidFilter(_)), "{msg}");
        assert!(
            msg.contains("1:"),
            "message should carry line:column, got {msg}"
        );
    }

    #[test]
    fn empty_and_oversized_expressions_rejected() {
        assert!(CelFilter::compile("   ").is_err());
        let long = format!("a == '{}'", "x".repeat(MAX_EXPRESSION_LEN));
        assert!(CelFilter::compile(&long).is_err());
    }

    #[test]
    fn evaluates_string_and_numeric_predicates() {
        let tile = tile_data(vec![landuse()]);
        let f = CelFilter::compile("type == 'residential'").expect("compile");
        assert_eq!(
            kept(&apply(tile.clone(), &[&f]).expect("apply")),
            vec![("landuse".to_string(), vec![0, 3])]
        );
        let f = CelFilter::compile("height > 50").expect("compile");
        assert_eq!(
            kept(&apply(tile.clone(), &[&f]).expect("apply")),
            vec![("landuse".to_string(), vec![1, 3])]
        );
        let f = CelFilter::compile("type in ['park', 'commercial']").expect("compile");
        assert_eq!(
            kept(&apply(tile, &[&f]).expect("apply")),
            vec![("landuse".to_string(), vec![1, 2])]
        );
    }

    #[test]
    fn missing_property_drops_feature() {
        let tile = tile_data(vec![landuse()]);
        let f = CelFilter::compile("levels >= 5").expect("compile");
        assert!(apply(tile, &[&f]).expect("apply").data.is_empty());
    }

    #[test]
    fn preset_and_request_compose_with_and() {
        let preset = CelFilter::from_config(&CelFilterConfig {
            expression: "type == 'residential'".to_string(),
            applies_to_layers: None,
        })
        .expect("compile preset");
        let request = CelFilter::compile("height > 50").expect("compile request");
        let out = apply(tile_data(vec![landuse()]), &[&preset, &request]).expect("apply");
        assert_eq!(kept(&out), vec![("landuse".to_string(), vec![3])]);
    }

    #[test]
    fn empty_filtered_layer_dropped_untouched_layer_kept() {
        let roads = layer("roads", &[&[("class", string("primary"))]]);
        let preset = CelFilter::from_config(&CelFilterConfig {
            expression: "type == 'nothing-matches'".to_string(),
            applies_to_layers: Some(vec!["landuse".to_string()]),
        })
        .expect("compile preset");
        let out = apply(tile_data(vec![landuse(), roads]), &[&preset]).expect("apply");
        assert_eq!(kept(&out), vec![("roads".to_string(), vec![0])]);
    }

    #[test]
    fn all_layers_empty_yields_empty_tile() {
        let f = CelFilter::compile("type == 'nothing-matches'").expect("compile");
        let out = apply(tile_data(vec![landuse()]), &[&f]).expect("apply");
        assert!(out.data.is_empty());
        assert_eq!(out.format, TileFormat::Pbf);
        assert_eq!(out.compression, TileCompression::None);
    }

    #[test]
    fn gzip_input_is_decoded() {
        let plain = tile_data(vec![landuse()]);
        let gz = crate::compression::recode(
            &plain,
            TileCompression::Gzip,
            &crate::config::CompressionConfig::default(),
        )
        .expect("gzip");
        assert_eq!(gz.compression, TileCompression::Gzip);
        let f = CelFilter::compile("type == 'park'").expect("compile");
        let out = apply(gz, &[&f]).expect("apply");
        assert_eq!(kept(&out), vec![("landuse".to_string(), vec![2])]);
    }

    #[test]
    fn raster_tile_rejected() {
        let png = TileData {
            data: Bytes::from_static(b"\x89PNG"),
            format: TileFormat::Png,
            compression: TileCompression::None,
        };
        let f = CelFilter::compile("type == 'a'").expect("compile");
        assert!(matches!(
            apply(png, &[&f]),
            Err(TileServerError::InvalidFilter(_))
        ));
    }

    #[test]
    fn no_filters_is_passthrough() {
        let tile = tile_data(vec![landuse()]);
        let out = apply(tile.clone(), &[]).expect("apply");
        assert_eq!(out.data, tile.data);
    }

    #[test]
    fn hash_is_stable_distinct_and_non_zero() {
        let a = CelFilter::compile("type == 'a'").expect("compile");
        let a2 = CelFilter::compile("  type == 'a'  ").expect("compile");
        let b = CelFilter::compile("type == 'b'").expect("compile");
        assert_eq!(a.hash(), a2.hash());
        assert_ne!(a.hash(), b.hash());
        assert_ne!(a.hash(), 0);
        assert_eq!(combined_hash(&[]), 0);
        assert_eq!(combined_hash(&[&a]), a.hash());
        assert_ne!(combined_hash(&[&a, &b]), combined_hash(&[&b, &a]));
    }

    #[tokio::test]
    async fn filtered_variants_get_separate_cache_entries() {
        let cache = crate::cache::TileCache::new(16, 60);
        let key = |filter| crate::cache::TileCacheKey {
            source_id: "s".into(),
            z: 1,
            x: 0,
            y: 0,
            encoding: TileCompression::None,
            filter,
        };
        let a = CelFilter::compile("type == 'a'").expect("compile");
        let b = CelFilter::compile("type == 'b'").expect("compile");
        let tile = |bytes: &'static [u8]| TileData {
            data: Bytes::from_static(bytes),
            format: TileFormat::Pbf,
            compression: TileCompression::None,
        };
        cache.insert(key(0), tile(b"plain")).await;
        cache.insert(key(a.hash()), tile(b"a")).await;
        assert_eq!(
            cache.get(&key(0)).await.expect("plain").data.as_ref(),
            b"plain"
        );
        assert_eq!(
            cache.get(&key(a.hash())).await.expect("a").data.as_ref(),
            b"a"
        );
        assert!(cache.get(&key(b.hash())).await.is_none());
    }

    /// Security gate: no expression may reach host functions. Any future
    /// `cel` upgrade must keep this test green.
    #[test]
    fn sandbox_rejects_non_allowlisted_functions() {
        for expr in [
            "timestamp('2024-01-01T00:00:00Z') > timestamp('2023-01-01T00:00:00Z')",
            "duration('1s') > duration('0s')",
            "dyn(height) > 1",
            "type(height) == int",
            "readFile('/etc/passwd') == ''",
            "env('HOME') == ''",
            "http_get('https://example.com') == ''",
        ] {
            let err = CelFilter::compile(expr).expect_err(expr);
            assert!(
                err.to_string().contains("not allowed"),
                "{expr} should be rejected by the allowlist, got {err}"
            );
        }
        for name in ALLOWED_FUNCTIONS {
            for banned in [
                "file", "read", "env", "http", "net", "exec", "time", "now", "dyn",
            ] {
                assert!(
                    !name.to_ascii_lowercase().contains(banned),
                    "allowlist entry {name} looks like a host function"
                );
            }
        }
    }
}
