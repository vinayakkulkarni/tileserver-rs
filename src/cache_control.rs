use axum::{
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE},
        HeaderValue,
    },
    middleware::Next,
    response::Response,
};

/// Set a Cache-Control header for defined static files.
pub async fn set_cache_header<B>(req: axum::http::Request<B>, next: Next<B>) -> Response {
    let cache_types = vec![
        "text/html",
        "text/css",
        "application/javascript",
        "image/svg+xml",
        "image/webp",
        "font/woff2",
    ];

    let mut response = next.run(req).await;

    if let Some(content_type) = response.headers().get(CONTENT_TYPE) {
        if cache_types.contains(&content_type.to_str().unwrap()) {
            response
                .headers_mut()
                .insert(CACHE_CONTROL, HeaderValue::from_static("max-age=31536000"));
        }
    }

    response
}
