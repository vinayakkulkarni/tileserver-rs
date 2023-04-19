use serde::Serialize;

#[derive(Serialize)]
pub struct Style {
    pub id: String,
    pub version: u8,
    pub name: String,
    pub url: String,
}
