//! HTTP-level metrics middleware.
//!
//! Recorded labels:
//! - `route`: the matched Axum route pattern (e.g.
//!   `/data/{source}/{z}/{x}/{y}.{format}`), NOT the raw URL. Bounded
//!   cardinality regardless of traffic shape.
//! - `method`: HTTP method.
//! - `status_class`: 2xx/3xx/4xx/5xx bucket.
//!
//! Routes that don't match any known pattern (e.g. 404s on unknown paths)
//! are recorded with `route = "unmatched"` to avoid cardinality blow-up
//! from URL probes.

use std::time::Instant;

use axum::body::Body;
use axum::extract::MatchedPath;
use axum::http::{Request, Response};
use axum::middleware::Next;

use super::recorder::{HttpEvent, http_in_flight_dec, http_in_flight_inc, http_request_recorded};

/// Axum middleware that records HTTP request metrics.
///
/// Wire via `axum::middleware::from_fn(record_http_request)` AFTER routing
/// so [`MatchedPath`] is populated, but BEFORE response compression so the
/// observed status code reflects the handler's actual response.
pub async fn record_http_request(request: Request<Body>, next: Next) -> Response<Body> {
    let started = Instant::now();
    let method = request.method().as_str().to_owned();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_owned())
        .unwrap_or_else(|| "unmatched".to_string());

    http_in_flight_inc();
    let response = next.run(request).await;
    http_in_flight_dec();

    http_request_recorded(HttpEvent {
        method: &method,
        route: &route,
        status: response.status().as_u16(),
        duration: started.elapsed(),
    });

    response
}
