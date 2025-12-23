use axum::{
    body::Body,
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE},
        HeaderValue, Request,
    },
    middleware::Next,
    response::Response,
};

/// Set a Cache-Control header for defined static files.
pub async fn set_cache_header(req: Request<Body>, next: Next) -> Response {
    let cache_types = [
        "text/html",
        "text/css",
        "application/javascript",
        "image/svg+xml",
        "image/webp",
        "image/png",
        "font/woff2",
    ];

    let mut response = next.run(req).await;

    if let Some(content_type) = response.headers().get(CONTENT_TYPE) {
        if let Ok(content_type_str) = content_type.to_str() {
            if cache_types.iter().any(|&t| content_type_str.starts_with(t)) {
                response
                    .headers_mut()
                    .insert(CACHE_CONTROL, HeaderValue::from_static("max-age=31536000"));
            }
        }
    }

    response
}

/// Set cache headers for tile responses
pub fn tile_cache_headers() -> HeaderValue {
    HeaderValue::from_static("public, max-age=86400, stale-while-revalidate=604800")
}
