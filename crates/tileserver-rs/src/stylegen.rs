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
    let _ = (metadata, base_url, key);
    // Stub: fails the tests until implemented in commit 1.2/1.4.
    json!({})
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
        let ids = ["roads", "buildings", "water", "poi", "landuse", "boundaries"];
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
        assert_eq!(geometry_to_layer_type("MultiPoint"), Some(LayerKind::Circle));
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

    // Placeholder ref so meta_with_layers compiles before full-gen tests land.
    #[test]
    fn stub_meta_builder_is_usable() {
        let m = meta_with_layers(None, TileFormat::Pbf);
        assert_eq!(m.id, "src");
    }
}
