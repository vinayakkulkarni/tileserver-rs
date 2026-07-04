//! Auto-generate MapLibre GL style JSON for vector sources without a
//! configured `[[styles]]` entry (GitHub issue #710).
//!
//! TileJSON 3.0 `vector_layers` entries are `{id, fields, minzoom, maxzoom,
//! description?}` — there is **no** `geometry` field on real sources. So the
//! generator emits three paint layers per source-layer (fill + line + circle),
//! each gated by a legacy `["==", "$type", …]` filter, grouped so that all
//! fills paint before all lines before all circles. Colors are deterministic
//! per source-layer id (a stable hue derived from the id hash).
//!
//! When a non-standard source *does* carry a `geometry` hint on a layer entry,
//! the generator narrows that layer to the single matching kind.

use std::hash::{Hash, Hasher};

use serde_json::{Value, json};

use crate::sources::{TileFormat, TileMetadata};

/// MapLibre paint layer kind derived from a vector layer's geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    /// Polygon fill layer.
    Fill,
    /// Line / stroke layer.
    Line,
    /// Point circle layer.
    Circle,
}

impl LayerKind {
    /// The three kinds in paint order: fills, then lines, then circles.
    const ORDER: [LayerKind; 3] = [LayerKind::Fill, LayerKind::Line, LayerKind::Circle];

    /// The MapLibre layer `type` string.
    fn maplibre_type(self) -> &'static str {
        match self {
            LayerKind::Fill => "fill",
            LayerKind::Line => "line",
            LayerKind::Circle => "circle",
        }
    }

    /// Suffix appended to the emitted layer id for disambiguation.
    fn suffix(self) -> &'static str {
        match self {
            LayerKind::Fill => "fill",
            LayerKind::Line => "line",
            LayerKind::Circle => "circle",
        }
    }

    /// The legacy `$type` filter geometry token this kind renders.
    fn type_filter_token(self) -> &'static str {
        match self {
            LayerKind::Fill => "Polygon",
            LayerKind::Line => "LineString",
            LayerKind::Circle => "Point",
        }
    }
}

/// Map an (optional) `vector_layers[].geometry` hint into the single matching
/// paint kind. Returns `None` for unknown / empty geometry (caller then falls
/// back to emitting all three kinds).
#[must_use]
pub fn geometry_to_layer_type(geom: &str) -> Option<LayerKind> {
    match geom {
        "Point" | "MultiPoint" => Some(LayerKind::Circle),
        "LineString" | "MultiLineString" => Some(LayerKind::Line),
        "Polygon" | "MultiPolygon" => Some(LayerKind::Fill),
        _ => None,
    }
}

/// Deterministic HSL hue (degrees `[0, 360)`) for a source-layer id.
fn hue_for_layer(layer_id: &str) -> u16 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    layer_id.hash(&mut hasher);
    (hasher.finish() % 360) as u16
}

/// Deterministic fill/line/circle color for a source-layer: `hsl(h, 70%, 50%)`.
/// Same `layer_id` always yields the same color across requests.
#[must_use]
pub fn color_for_layer(layer_id: &str) -> String {
    format!("hsl({}, 70%, 50%)", hue_for_layer(layer_id))
}

/// Darker outline companion color: `hsl(h, 70%, 35%)`.
#[must_use]
pub fn outline_color_for_layer(layer_id: &str) -> String {
    format!("hsl({}, 70%, 35%)", hue_for_layer(layer_id))
}

/// Build a MapLibre GL v8 style JSON for a single vector source.
///
/// Pure function; no I/O. Used by both `routes/styles.rs::get_style_json` and
/// tests. Empty / missing `vector_layers` yields a valid minimal style with
/// just a `background` layer so the viewer never errors.
#[must_use]
pub fn generate_style_for_vector_source(
    metadata: &TileMetadata,
    base_url: &str,
    key: Option<&str>,
) -> Value {
    let key_query = key
        .map(|k| format!("?key={}", urlencoding::encode(k)))
        .unwrap_or_default();

    let source_obj = build_source_object(metadata, base_url, &key_query);
    let paint_layers = build_paint_layers(metadata);

    let mut layers = Vec::with_capacity(paint_layers.len() + 1);
    layers.push(json!({
        "id": "background",
        "type": "background",
        "paint": { "background-color": "hsl(240, 8%, 12%)" }
    }));
    layers.extend(paint_layers);

    json!({
        "version": 8,
        "name": format!("{} (auto)", metadata.id),
        "sources": { metadata.id.clone(): source_obj },
        "layers": layers,
        "glyphs": format!("{base_url}/fonts/{{fontstack}}/{{range}}.pbf{key_query}"),
    })
}

/// Build the single vector-source object. MLT sources emit inline `.pbf`
/// tiles because `maplibre-gl-js` cannot decode MLT; PBF sources reference the
/// TileJSON endpoint via `url`.
fn build_source_object(metadata: &TileMetadata, base_url: &str, key_query: &str) -> Value {
    if metadata.format == TileFormat::Mlt {
        let tiles = format!(
            "{base_url}/data/{}/{{z}}/{{x}}/{{y}}.pbf{key_query}",
            metadata.id
        );
        json!({
            "type": "vector",
            "tiles": [tiles],
            "minzoom": metadata.minzoom,
            "maxzoom": metadata.maxzoom,
        })
    } else {
        json!({
            "type": "vector",
            "url": format!("{base_url}/data/{}.json{key_query}", metadata.id),
            "minzoom": metadata.minzoom,
            "maxzoom": metadata.maxzoom,
        })
    }
}

/// Emit paint layers grouped by kind: all fills, then all lines, then all
/// circles. Interleaving would let fills paint over points.
fn build_paint_layers(metadata: &TileMetadata) -> Vec<Value> {
    let source_layers = collect_source_layers(metadata);
    let mut out = Vec::with_capacity(source_layers.len() * 3);
    for kind in LayerKind::ORDER {
        for sl in &source_layers {
            if sl.kinds.contains(&kind) {
                out.push(build_layer(&metadata.id, sl, kind));
            }
        }
    }
    out
}

struct SourceLayer {
    id: String,
    minzoom: Option<u64>,
    maxzoom: Option<u64>,
    kinds: Vec<LayerKind>,
}

/// Extract source-layers from `metadata.vector_layers`, dropping empty ids.
/// A `geometry` hint narrows a layer to one kind; otherwise it gets all three.
fn collect_source_layers(metadata: &TileMetadata) -> Vec<SourceLayer> {
    let Some(Value::Array(entries)) = metadata.vector_layers.as_ref() else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(id) = entry.get("id").and_then(Value::as_str) else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        let kinds = match entry.get("geometry").and_then(Value::as_str) {
            Some(g) => match geometry_to_layer_type(g) {
                Some(k) => vec![k],
                None => LayerKind::ORDER.to_vec(),
            },
            None => LayerKind::ORDER.to_vec(),
        };
        out.push(SourceLayer {
            id: id.to_string(),
            minzoom: entry.get("minzoom").and_then(Value::as_u64),
            maxzoom: entry.get("maxzoom").and_then(Value::as_u64),
            kinds,
        });
    }
    out
}

fn build_layer(source_id: &str, sl: &SourceLayer, kind: LayerKind) -> Value {
    let mut layer = json!({
        "id": format!("{source_id}-{}-{}", sl.id, kind.suffix()),
        "type": kind.maplibre_type(),
        "source": source_id,
        "source-layer": sl.id,
        "filter": ["==", "$type", kind.type_filter_token()],
        "paint": paint_for_kind(kind, &sl.id),
    });
    if let (Some(obj), Some(mz)) = (layer.as_object_mut(), sl.minzoom) {
        obj.insert("minzoom".to_string(), json!(mz));
    }
    if let (Some(obj), Some(mz)) = (layer.as_object_mut(), sl.maxzoom) {
        obj.insert("maxzoom".to_string(), json!(mz));
    }
    layer
}

fn paint_for_kind(kind: LayerKind, layer_id: &str) -> Value {
    let color = color_for_layer(layer_id);
    match kind {
        LayerKind::Fill => json!({
            "fill-color": color,
            "fill-opacity": 0.5,
            "fill-outline-color": outline_color_for_layer(layer_id),
        }),
        LayerKind::Line => json!({
            "line-color": color,
            "line-width": ["interpolate", ["linear"], ["zoom"], 0, 0.5, 22, 2],
        }),
        LayerKind::Circle => json!({
            "circle-color": color,
            "circle-radius": ["interpolate", ["linear"], ["zoom"], 0, 1, 8, 2, 14, 4, 22, 12],
            "circle-stroke-width": 1,
            "circle-stroke-color": "#ffffff",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta_with_layers(vector_layers: Option<Value>, format: TileFormat) -> TileMetadata {
        TileMetadata {
            id: "src".to_string(),
            name: "Source".to_string(),
            description: None,
            attribution: None,
            format,
            minzoom: 0,
            maxzoom: 14,
            bounds: None,
            center: None,
            vector_layers,
        }
    }

    // --- color_for_layer ---

    #[test]
    fn color_for_layer_is_deterministic() {
        assert_eq!(color_for_layer("roads"), color_for_layer("roads"));
    }

    #[test]
    fn color_for_layer_different_ids_can_produce_different_colors() {
        let ids = [
            "roads",
            "buildings",
            "water",
            "poi",
            "landuse",
            "boundaries",
        ];
        let distinct: std::collections::HashSet<String> =
            ids.iter().map(|id| color_for_layer(id)).collect();
        assert!(
            distinct.len() >= 3,
            "expected >=3 distinct colors, got {}",
            distinct.len()
        );
    }

    #[test]
    fn color_for_layer_returns_valid_hsl() {
        let c = color_for_layer("roads");
        assert!(c.starts_with("hsl("), "got {c}");
        assert!(c.ends_with("70%, 50%)"), "got {c}");
    }

    #[test]
    fn outline_color_for_layer_uses_darker_lightness() {
        let c = outline_color_for_layer("roads");
        assert!(c.ends_with("70%, 35%)"), "got {c}");
    }

    #[test]
    fn outline_shares_hue_with_fill() {
        let fill = color_for_layer("water");
        let outline = outline_color_for_layer("water");
        let fill_hue = fill
            .trim_start_matches("hsl(")
            .split(',')
            .next()
            .unwrap()
            .to_string();
        let outline_hue = outline
            .trim_start_matches("hsl(")
            .split(',')
            .next()
            .unwrap()
            .to_string();
        assert_eq!(fill_hue, outline_hue);
    }

    // --- geometry_to_layer_type ---

    #[test]
    fn geometry_to_layer_type_point() {
        assert_eq!(geometry_to_layer_type("Point"), Some(LayerKind::Circle));
    }

    #[test]
    fn geometry_to_layer_type_multipoint() {
        assert_eq!(
            geometry_to_layer_type("MultiPoint"),
            Some(LayerKind::Circle)
        );
    }

    #[test]
    fn geometry_to_layer_type_linestring() {
        assert_eq!(geometry_to_layer_type("LineString"), Some(LayerKind::Line));
    }

    #[test]
    fn geometry_to_layer_type_multilinestring() {
        assert_eq!(
            geometry_to_layer_type("MultiLineString"),
            Some(LayerKind::Line)
        );
    }

    #[test]
    fn geometry_to_layer_type_polygon() {
        assert_eq!(geometry_to_layer_type("Polygon"), Some(LayerKind::Fill));
    }

    #[test]
    fn geometry_to_layer_type_multipolygon() {
        assert_eq!(
            geometry_to_layer_type("MultiPolygon"),
            Some(LayerKind::Fill)
        );
    }

    #[test]
    fn geometry_to_layer_type_unknown_returns_none() {
        assert_eq!(geometry_to_layer_type("Frobnicate"), None);
    }

    #[test]
    fn geometry_to_layer_type_empty_returns_none() {
        assert_eq!(geometry_to_layer_type(""), None);
    }

    #[test]
    fn geometry_to_layer_type_geometrycollection_returns_none() {
        assert_eq!(geometry_to_layer_type("GeometryCollection"), None);
    }

    // --- full style generation ---

    fn three_layers() -> Value {
        json!([
            { "id": "buildings", "minzoom": 12, "maxzoom": 14 },
            { "id": "roads" },
            { "id": "water" }
        ])
    }

    fn layers_array(style: &Value) -> &Vec<Value> {
        style["layers"].as_array().expect("layers is array")
    }

    fn paint_layers(style: &Value) -> Vec<&Value> {
        layers_array(style)
            .iter()
            .filter(|l| l["type"] != "background")
            .collect()
    }

    #[test]
    fn generate_style_version_is_8() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        assert_eq!(s["version"], 8);
    }

    #[test]
    fn generate_style_name_is_id_with_auto_suffix() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        assert_eq!(s["name"], "src (auto)");
    }

    #[test]
    fn generate_style_has_single_vector_source_pointing_at_tilejson() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        assert_eq!(s["sources"]["src"]["type"], "vector");
        assert_eq!(s["sources"]["src"]["url"], "http://h/data/src.json");
    }

    #[test]
    fn generate_style_includes_key_query_when_provided() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", Some("abc"));
        let url = s["sources"]["src"]["url"].as_str().unwrap();
        assert!(url.contains("?key=abc"), "got {url}");
    }

    #[test]
    fn generate_style_omits_query_when_no_key() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        let url = s["sources"]["src"]["url"].as_str().unwrap();
        assert!(!url.contains('?'), "got {url}");
    }

    #[test]
    fn generate_style_min_max_zoom_from_metadata() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        assert_eq!(s["sources"]["src"]["minzoom"], 0);
        assert_eq!(s["sources"]["src"]["maxzoom"], 14);
    }

    #[test]
    fn generate_style_always_includes_background_layer() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        let layers = layers_array(&s);
        assert_eq!(layers[0]["type"], "background");
        assert!(layers[0]["paint"]["background-color"].is_string());
    }

    #[test]
    fn generate_style_emits_three_layers_per_source_layer_without_geometry_hint() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        assert_eq!(paint_layers(&s).len(), 3 * 3);
    }

    #[test]
    fn generate_style_orders_all_fills_before_lines_before_circles() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        let types: Vec<&str> = paint_layers(&s)
            .iter()
            .map(|l| l["type"].as_str().unwrap())
            .collect();
        assert_eq!(
            types,
            vec![
                "fill", "fill", "fill", "line", "line", "line", "circle", "circle", "circle"
            ]
        );
    }

    #[test]
    fn generate_style_uses_legacy_type_filters() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        let fills: Vec<&Value> = paint_layers(&s)
            .into_iter()
            .filter(|l| l["type"] == "fill")
            .collect();
        assert_eq!(fills[0]["filter"], json!(["==", "$type", "Polygon"]));
    }

    #[test]
    fn generate_style_layer_id_includes_source_layer_and_kind() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        let ids: Vec<&str> = layers_array(&s)
            .iter()
            .map(|l| l["id"].as_str().unwrap())
            .collect();
        assert!(ids.contains(&"src-buildings-fill"), "ids: {ids:?}");
        assert!(ids.contains(&"src-roads-line"), "ids: {ids:?}");
        assert!(ids.contains(&"src-water-circle"), "ids: {ids:?}");
    }

    #[test]
    fn generate_style_sets_source_layer_on_every_paint_layer() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        for l in paint_layers(&s) {
            assert_eq!(l["source"], "src");
            assert!(l["source-layer"].is_string());
        }
    }

    #[test]
    fn generate_style_fill_has_outline_color() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        let fill = paint_layers(&s)
            .into_iter()
            .find(|l| l["type"] == "fill")
            .unwrap();
        assert!(fill["paint"]["fill-outline-color"].is_string());
        assert!(fill["paint"]["fill-color"].is_string());
        assert!(fill["paint"]["fill-opacity"].is_number());
    }

    #[test]
    fn generate_style_paint_line_uses_zoom_interpolated_width() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        let line = paint_layers(&s)
            .into_iter()
            .find(|l| l["type"] == "line")
            .unwrap();
        let w = &line["paint"]["line-width"];
        assert_eq!(w[0], "interpolate");
        assert_eq!(w[2], json!(["zoom"]));
        assert!(line["paint"]["line-color"].is_string());
    }

    #[test]
    fn generate_style_paint_circle_has_required_fields() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        let circle = paint_layers(&s)
            .into_iter()
            .find(|l| l["type"] == "circle")
            .unwrap();
        assert_eq!(circle["paint"]["circle-radius"][0], "interpolate");
        assert!(circle["paint"]["circle-color"].is_string());
        assert_eq!(circle["paint"]["circle-stroke-width"], 1);
        assert_eq!(circle["paint"]["circle-stroke-color"], "#ffffff");
    }

    #[test]
    fn generate_style_copies_layer_minzoom_maxzoom() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        // buildings had minzoom 12 / maxzoom 14
        let bfill = paint_layers(&s)
            .into_iter()
            .find(|l| l["id"] == "src-buildings-fill")
            .unwrap();
        assert_eq!(bfill["minzoom"], 12);
        assert_eq!(bfill["maxzoom"], 14);
    }

    #[test]
    fn generate_style_omits_layer_zoom_when_absent() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        // roads had no minzoom/maxzoom
        let rline = paint_layers(&s)
            .into_iter()
            .find(|l| l["id"] == "src-roads-line")
            .unwrap();
        assert!(rline.get("minzoom").is_none() || rline["minzoom"].is_null());
    }

    #[test]
    fn generate_style_narrows_to_single_kind_when_geometry_hint_present() {
        let vl = json!([{ "id": "roads", "geometry": "LineString" }]);
        let m = meta_with_layers(Some(vl), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        let paint = paint_layers(&s);
        assert_eq!(paint.len(), 1);
        assert_eq!(paint[0]["type"], "line");
    }

    #[test]
    fn generate_style_empty_vector_layers_returns_background_only_style() {
        let m = meta_with_layers(Some(json!([])), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        assert_eq!(layers_array(&s).len(), 1);
        assert_eq!(layers_array(&s)[0]["type"], "background");
    }

    #[test]
    fn generate_style_none_vector_layers_returns_background_only_style() {
        let m = meta_with_layers(None, TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        assert_eq!(layers_array(&s).len(), 1);
        assert_eq!(layers_array(&s)[0]["type"], "background");
    }

    #[test]
    fn generate_style_skips_vector_layers_with_empty_id() {
        let vl = json!([{ "id": "" }, { "id": "roads" }]);
        let m = meta_with_layers(Some(vl), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        // only "roads" contributes 3 layers
        assert_eq!(paint_layers(&s).len(), 3);
    }

    #[test]
    fn generate_style_with_mlt_format_uses_pbf_tile_url_for_viewer_compat() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Mlt);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        // MLT can't be decoded by maplibre-gl-js: emit inline .pbf tiles URL.
        let tiles = s["sources"]["src"]["tiles"]
            .as_array()
            .expect("mlt source uses inline tiles");
        assert!(
            tiles[0].as_str().unwrap().ends_with("/{z}/{x}/{y}.pbf"),
            "got {tiles:?}"
        );
    }

    #[test]
    fn generate_style_includes_glyphs_url() {
        let m = meta_with_layers(Some(three_layers()), TileFormat::Pbf);
        let s = generate_style_for_vector_source(&m, "http://h", None);
        assert!(s["glyphs"].as_str().unwrap().contains("/fonts/"));
    }
}
