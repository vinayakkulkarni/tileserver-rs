use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Root {
    pub options: Options,
    pub styles: Styles,
    pub data: Data,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Options {
    pub paths: Paths,
    pub domains: Vec<String>,
    pub format_quality: FormatQuality,
    pub max_scale_factor: i64,
    pub max_size: i64,
    pub pbf_alias: String,
    pub serve_all_fonts: bool,
    pub serve_all_styles: bool,
    pub serve_static_maps: bool,
    pub allow_remote_marker_icons: bool,
    pub tile_margin: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paths {
    pub root: String,
    pub fonts: String,
    pub sprites: String,
    pub icons: String,
    pub styles: String,
    pub mbtiles: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormatQuality {
    pub jpeg: i64,
    pub webp: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Styles {
    pub basic: Basic,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Basic {
    pub style: String,
    pub tilejson: Tilejson,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tilejson {
    #[serde(rename = "type")]
    pub type_field: String,
    pub bounds: Vec<f64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tilejson2 {
    pub format: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Data {
    #[serde(rename = "zurich-vector")]
    pub zurich_vector: ZurichVector,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZurichVector {
    pub mbtiles: String,
}
