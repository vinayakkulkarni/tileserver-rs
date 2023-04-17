use axum::{
    http::{
        header::{ACCEPT, CONTENT_SECURITY_POLICY, CONTENT_TYPE},
        HeaderValue, Method, StatusCode,
    },
    middleware,
    routing::get,
    Router, Server,
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

// mod drivers;
// mod routes;

// mod structs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let router = Router::new()
        .merge(static_file_handler())
        .nest("/api", api_handler())
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
            HeaderValue::from_static("default-src 'self'; object-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; form-action 'self'; frame-ancestors 'none';"),
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
    Router::new().route("/health", get(|| async { (StatusCode::OK, "OK") }))
}
