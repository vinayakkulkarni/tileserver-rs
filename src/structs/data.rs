use serde::Deserialize;
use serde::Serialize;
// use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub tiles: Vec<String>,
    pub name: String,
    pub format: String,
    pub basename: String,
    pub id: String,
    pub attribution: String,
    pub bounds: Vec<f64>,
    pub center: Vec<f64>,
    pub description: String,
    pub maxzoom: i64,
    pub minzoom: i64,
    // #[serde(rename = "vector_layers")]
    // pub vector_layers: Vec<Value>,
    pub mask_level: String,
    pub version: String,
    pub tilejson: String,
}
