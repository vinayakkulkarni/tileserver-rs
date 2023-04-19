use axum::{
    http::{
        header::{ACCEPT, CONTENT_SECURITY_POLICY, CONTENT_TYPE},
        HeaderValue, Method, StatusCode,
    },
    middleware,
    routing::get,
    Json, Router, Server,
};
use std::{net::SocketAddr, time::Duration};
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

mod cache_control;

mod structs;
use structs::data::Data;
use structs::style::Style;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let router = Router::new()
        .nest("/", api_handler())
        .merge(static_file_handler())
        .layer(
            CorsLayer::new()
                .allow_headers([ACCEPT, CONTENT_TYPE])
                .max_age(Duration::from_secs(86400)) // 1 day
                .allow_origin(
                    std::env::var("CORS_ORIGIN")
                        .unwrap_or_else(|_| "*".to_string())
                        .parse::<HeaderValue>()?,
                )
                .allow_methods(vec![Method::GET, Method::OPTIONS, Method::HEAD]),
        )
        .layer(SetResponseHeaderLayer::if_not_present(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'self'; object-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; form-action 'self'; frame-ancestors 'none'; worker-src 'self' blob:;"),
        ))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    tracing::debug!("listening on {}", addr);

    Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

fn static_file_handler() -> Router {
    // Static assets served from this router will be cached.
    Router::new()
        .nest_service("/_nuxt", ServeDir::new("../client/dist/_nuxt"))
        .nest_service(
            "/",
            ServeDir::new("../client/dist/")
                .not_found_service(ServeFile::new("../client/dist/404.html")),
        )
        .layer(middleware::from_fn(cache_control::set_cache_header))
}

fn api_handler() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/styles.json", get(styles_json))
        .route("/data.json", get(data_json))
}

async fn health() -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}

async fn styles_json() -> anyhow::Result<Json<Vec<Style>>, StatusCode> {
    let mut styles: Vec<Style> = Vec::new();
    let style = Style {
        id: String::from("osm-bright"),
        version: 8,
        name: String::from("osm-bright"),
        url: String::from("http://localhost:3000/styles/osm-bright/style.json"),
    };
    styles.push(style);
    Ok(Json(styles))
}

async fn data_json() -> anyhow::Result<Json<Vec<Data>>, StatusCode> {
    let mut data: Vec<Data> = Vec::new();
    let item = Data {
        tiles: vec![String::from(
            "http://[::]:8080/data/openmaptiles/{z}/{x}/{y}.pbf",
        )],
        name: String::from("OpenMapTiles"),
        format: String::from("pbf"),
        basename: String::from("planet.mbtiles"),
        id: String::from("openmaptiles"),
        attribution: String::from( "<a href=\"https://www.openstreetmap.org/copyright\" target=\"_blank\">&copy; OpenStreetMap contributors</a>"),
        bounds: vec![-180.0, -85.0511, 180.0, 85.0511],
        center: vec![-12.2168, 28.6135],
        description: String::from("A tileset showcasing all layers in OpenMapTiles. https://openmaptiles.org"),
        maxzoom: 14,
        minzoom: 0,
        mask_level: String::from("8"),
        tilejson: String::from("2.0.0"),
        version: String::from("3.11"),
    };
    data.push(item);
    Ok(Json(data))
}
