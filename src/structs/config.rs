use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub options: Options,
    pub styles: Styles,
    pub data: Data,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Options {
    pub paths: Paths,
    pub front_page: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Paths {
    pub root: String,
    pub fonts: String,
    pub styles: String,
    pub mbtiles: String,
    pub pmtiles: String,
    pub sprites: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Styles {
    pub positron: Positron,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Positron {
    pub style: String,
    #[serde(rename = "serve_data")]
    pub serve_data: bool,
    pub tilejson: Tilejson,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tilejson {
    pub format: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub openmaptiles: Openmaptiles,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Openmaptiles {
    pub mbtiles: String,
    pub pmtiles: String,
}
