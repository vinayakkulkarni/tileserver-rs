use crate::structs::style::Style;
use actix_web::{get, HttpResponse, Responder};

#[get("/styles.json")]
async fn styles_json() -> impl Responder {
    let mut styles: Vec<Style> = Vec::new();
    let style = Style {
        id: String::from("osm-bright"),
        version: 8,
        name: String::from("osm-bright"),
        url: String::from("http://localhost:3000/styles/osm-bright/style.json"),
    };
    styles.push(style);
    HttpResponse::Ok().json(styles)
}

#[get("/rendered.json")]
async fn rendered_json() -> impl Responder {
    HttpResponse::Ok().body("All TileJSON")
}

#[get("/data.json")]
async fn data_json() -> impl Responder {
    HttpResponse::Ok().body("Data")
}

#[get("/index.json")]
async fn index_json() -> impl Responder {
    HttpResponse::Ok().body("All")
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("OK")
}
