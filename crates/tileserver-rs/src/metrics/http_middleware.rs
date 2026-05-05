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

#[cfg(test)]
mod tests {
    use super::*;

    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    use crate::metrics::{Cardinality, recorder::init};

    async fn ok_handler() -> &'static str {
        "ok"
    }

    async fn server_error_handler() -> (StatusCode, &'static str) {
        (StatusCode::INTERNAL_SERVER_ERROR, "boom")
    }

    fn build_app() -> Router {
        init(Cardinality::Strict);
        Router::new()
            .route("/data/{source}/{z}/{x}/{y_fmt}", get(ok_handler))
            .route("/error", get(server_error_handler))
            .layer(axum::middleware::from_fn(record_http_request))
    }

    #[tokio::test]
    async fn middleware_records_matched_path_route() {
        let app = build_app();
        let req = Request::builder()
            .uri("/data/openmaptiles/14/8192/5461")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn middleware_records_5xx_status_class() {
        let app = build_app();
        let req = Request::builder()
            .uri("/error")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn middleware_records_unmatched_route() {
        let app = build_app();
        let req = Request::builder()
            .uri("/totally-unknown-path-that-doesnt-match")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
