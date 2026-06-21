//! Selective trailing-slash normalization.
//!
//! A blanket [`tower_http::normalize_path::NormalizePathLayer::trim_trailing_slash`]
//! is WRONG for this server because the trailing slash is *meaningful* on the
//! SPA viewer routes. `/data/protomaps` is the TileJSON **API**; `/data/protomaps/`
//! is the embedded **inspector UI** (served by the SPA fallback). Likewise
//! `/styles/{id}/` is the map viewer UI while `/styles/{id}.json` /
//! `/styles/{id}/style.json` are API endpoints. Blanket trimming collapsed every
//! `/styles/{id}/` and `/data/{id}/` UI link onto the greedy `/styles/{style_json}`
//! / `/data/{source}` API routes, 404ing every "Open in viewer" / "Inspect"
//! link from the home page.
//!
//! This module trims the trailing slash for *every* path EXCEPT the two SPA
//! viewer families, preserving the API hardening (`/health/`, `/ping/`,
//! `/_openapi/` all still resolve) without breaking the web UI. Like
//! `NormalizePathLayer`, [`SelectiveTrailingSlashLayer`] must wrap the router
//! from the OUTSIDE (routing happens before inner `.layer()` middleware) and is
//! served via `ServiceExt::into_make_service`.

use std::task::{Context, Poll};

use axum::extract::Request;
use axum::http::Uri;
use axum::http::uri::PathAndQuery;
use tower::{Layer, Service};

/// Returns the trailing-slash-trimmed form of `path` when it is eligible for
/// trimming, or `None` when the path must be preserved as-is.
///
/// Preserved (returns `None`): the SPA viewer routes `/styles/{id}/` and
/// `/data/{id}/` (single id segment + trailing slash), plus any path without a
/// trailing slash and the bare root `/`.
pub fn trim_path_if_eligible(path: &str) -> Option<String> {
    if path.len() <= 1 || !path.ends_with('/') {
        return None;
    }
    if is_spa_viewer_path(path) {
        return None;
    }
    let trimmed = path.trim_end_matches('/');
    // A path that was only slashes (e.g. "//") trims to empty — leave it for
    // the router to 404 rather than synthesizing an empty URI.
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_owned())
}

/// True when `path` is exactly `/styles/{id}/` or `/data/{id}/` — one id
/// segment followed by a trailing slash. These are the SPA viewer surfaces
/// whose trailing slash is load-bearing (it selects the UI over the API).
fn is_spa_viewer_path(path: &str) -> bool {
    let mut segments = path.split('/').filter(|s| !s.is_empty());
    matches!(
        (segments.next(), segments.next(), segments.next()),
        (Some("styles" | "data"), Some(_), None)
    )
}

/// Tower layer applying [`SelectiveTrailingSlash`].
#[derive(Clone, Copy, Debug, Default)]
pub struct SelectiveTrailingSlashLayer;

impl<S> Layer<S> for SelectiveTrailingSlashLayer {
    type Service = SelectiveTrailingSlash<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SelectiveTrailingSlash { inner }
    }
}

/// Service that trims a request's trailing slash unless the path is an SPA
/// viewer route (see [`trim_path_if_eligible`]). Must wrap the router from the
/// OUTSIDE so it runs before routing.
#[derive(Clone, Debug)]
pub struct SelectiveTrailingSlash<S> {
    inner: S,
}

impl<S> Service<Request> for SelectiveTrailingSlash<S>
where
    S: Service<Request>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        if let Some(trimmed) = trim_path_if_eligible(req.uri().path()) {
            rewrite_request_path(&mut req, &trimmed);
        }
        self.inner.call(req)
    }
}

/// Rewrite `req`'s URI path to `new_path`, preserving the query string. A
/// malformed rebuild leaves the request untouched (the inner router then sees
/// the original URI), so this never drops a request.
fn rewrite_request_path(req: &mut Request, new_path: &str) {
    let rebuilt = match req.uri().query() {
        Some(query) => format!("{new_path}?{query}"),
        None => new_path.to_owned(),
    };
    let Ok(path_and_query) = PathAndQuery::try_from(rebuilt) else {
        return;
    };
    let mut parts = req.uri().clone().into_parts();
    parts.path_and_query = Some(path_and_query);
    if let Ok(uri) = Uri::from_parts(parts) {
        *req.uri_mut() = uri;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use tower::{ServiceExt, service_fn};

    // ---- pure: trim_path_if_eligible ------------------------------------

    #[test]
    fn trims_api_trailing_slash() {
        assert_eq!(
            trim_path_if_eligible("/health/").as_deref(),
            Some("/health")
        );
        assert_eq!(trim_path_if_eligible("/ping/").as_deref(), Some("/ping"));
        assert_eq!(
            trim_path_if_eligible("/_openapi/").as_deref(),
            Some("/_openapi")
        );
        assert_eq!(
            trim_path_if_eligible("/data.json/").as_deref(),
            Some("/data.json")
        );
    }

    #[test]
    fn preserves_spa_viewer_routes() {
        assert_eq!(trim_path_if_eligible("/styles/india-dark-mlt/"), None);
        assert_eq!(trim_path_if_eligible("/data/protomaps/"), None);
    }

    #[test]
    fn trims_deeper_style_and_data_api_paths() {
        // Two+ segments after the prefix are API routes, not the SPA viewer.
        assert_eq!(
            trim_path_if_eligible("/styles/foo/style.json/").as_deref(),
            Some("/styles/foo/style.json")
        );
        assert_eq!(
            trim_path_if_eligible("/styles/foo/0/0/0.png/").as_deref(),
            Some("/styles/foo/0/0/0.png")
        );
        assert_eq!(
            trim_path_if_eligible("/data/foo/0/0/0.pbf/").as_deref(),
            Some("/data/foo/0/0/0.pbf")
        );
        assert_eq!(
            trim_path_if_eligible("/styles/foo/sprite/").as_deref(),
            Some("/styles/foo/sprite")
        );
    }

    #[test]
    fn preserves_paths_without_trailing_slash() {
        assert_eq!(trim_path_if_eligible("/health"), None);
        assert_eq!(trim_path_if_eligible("/styles/foo"), None);
        assert_eq!(trim_path_if_eligible("/data/protomaps"), None);
    }

    #[test]
    fn preserves_root_and_slash_only_paths() {
        assert_eq!(trim_path_if_eligible("/"), None);
        assert_eq!(trim_path_if_eligible("//"), None);
    }

    #[test]
    fn bare_prefix_with_slash_is_trimmed() {
        // "/styles/" has no id segment, so it is not a viewer route.
        assert_eq!(
            trim_path_if_eligible("/styles/").as_deref(),
            Some("/styles")
        );
        assert_eq!(trim_path_if_eligible("/data/").as_deref(), Some("/data"));
    }

    // ---- pure: is_spa_viewer_path ---------------------------------------

    #[test]
    fn spa_viewer_path_classification() {
        assert!(is_spa_viewer_path("/styles/x/"));
        assert!(is_spa_viewer_path("/data/x/"));
        assert!(is_spa_viewer_path("/styles/india-dark-mlt/"));
        // Not viewer routes:
        assert!(!is_spa_viewer_path("/styles/x/y/"));
        assert!(!is_spa_viewer_path("/health/"));
        assert!(!is_spa_viewer_path("/styles/"));
        assert!(!is_spa_viewer_path("/fonts/x/"));
    }

    // ---- service: SelectiveTrailingSlash --------------------------------

    /// Echo the (possibly rewritten) request URI back as the response body so
    /// tests can assert exactly what the inner service received.
    async fn echo_uri(path: &str) -> String {
        let svc = SelectiveTrailingSlashLayer.layer(service_fn(|req: Request| async move {
            Ok::<_, std::convert::Infallible>(axum::response::Response::new(Body::from(
                req.uri().to_string(),
            )))
        }));
        let req = Request::builder()
            .uri(path)
            .body(Body::empty())
            .expect("valid request");
        let resp = svc.oneshot(req).await.expect("service ok");
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("body");
        String::from_utf8(bytes.to_vec()).expect("utf8")
    }

    #[tokio::test]
    async fn service_trims_api_path() {
        assert_eq!(echo_uri("/health/").await, "/health");
    }

    #[tokio::test]
    async fn service_preserves_spa_path() {
        assert_eq!(
            echo_uri("/styles/india-dark-mlt/").await,
            "/styles/india-dark-mlt/"
        );
        assert_eq!(echo_uri("/data/protomaps/").await, "/data/protomaps/");
    }

    #[tokio::test]
    async fn service_preserves_query_string_when_trimming() {
        // The trailing slash is trimmed but ?raster (and any query) survives.
        assert_eq!(echo_uri("/_openapi/?foo=bar").await, "/_openapi?foo=bar");
    }

    #[tokio::test]
    async fn service_preserves_query_on_spa_path() {
        assert_eq!(
            echo_uri("/styles/india-dark-mlt/?raster").await,
            "/styles/india-dark-mlt/?raster"
        );
    }

    #[tokio::test]
    async fn service_passes_through_non_slash_path() {
        assert_eq!(echo_uri("/health").await, "/health");
    }
}
