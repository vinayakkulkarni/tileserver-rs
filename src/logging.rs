//! HTTP request logging middleware
//!
//! Provides Martin/actix-web style request logging with the format:
//! `IP "METHOD PATH HTTP/VERSION" STATUS SIZE "REFERRER" "USER_AGENT" DURATION`
//!
//! Example output:
//! ```
//! 172.21.0.1 "GET /data/planet/12/2876/1828.pbf HTTP/1.1" 200 45883 "-" "node" 0.001492
//! ```

use axum::{
    body::Body,
    http::{header, Request, Response},
    middleware::Next,
};
use std::{net::SocketAddr, time::Instant};

/// Middleware that logs HTTP requests in Martin/actix-web combined format
pub async fn request_logger(request: Request<Body>, next: Next) -> Response<Body> {
    let start = Instant::now();

    // Extract request info before consuming the request
    let method = request.method().to_string();
    let path = request
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());
    let version = format!("{:?}", request.version());

    // Get client IP from x-forwarded-for header or connection info
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            request
                .extensions()
                .get::<axum::extract::ConnectInfo<SocketAddr>>()
                .map(|ci| ci.0.ip().to_string())
        })
        .unwrap_or_else(|| "-".to_string());

    // Get referrer
    let referrer = request
        .headers()
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();

    // Get user agent
    let user_agent = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();

    // Process the request
    let response = next.run(request).await;

    // Calculate duration
    let duration = start.elapsed();
    let duration_secs = duration.as_secs_f64();

    // Get response info
    let status = response.status().as_u16();
    let size = response
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Log in Martin/actix-web format
    // Format: IP "METHOD PATH HTTP/VERSION" STATUS SIZE "REFERRER" "USER_AGENT" DURATION
    tracing::info!(
        target: "tileserver_rs::http",
        "{} \"{} {} {}\" {} {} \"{}\" \"{}\" {:.6}",
        client_ip,
        method,
        path,
        version,
        status,
        size,
        referrer,
        user_agent,
        duration_secs
    );

    response
}
