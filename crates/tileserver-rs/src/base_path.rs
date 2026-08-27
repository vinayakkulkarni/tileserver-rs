//! Base-path prefix stripping for `SubfolderMode::Nested` deployments.
//!
//! When the server is deployed under a URL subfolder and the reverse proxy
//! forwards the prefix untouched (rather than stripping it), every request
//! arrives with the subfolder prefix (e.g. `/maps/ping`, `/maps/_nuxt/x.js`).
//! This layer strips that prefix before routing so the existing router — which
//! is defined at the root — matches unchanged. It is the in-server equivalent
//! of a `proxy_pass http://backend/;` prefix strip, and it reuses the whole
//! route table, SPA fallback, and trailing-slash handling verbatim.
//!
//! Like [`crate::trailing_slash::SelectiveTrailingSlashLayer`], this layer must
//! wrap the router from the OUTSIDE (routing happens before inner `.layer()`
//! middleware) and is served via `ServiceExt::into_make_service`.

use std::sync::Arc;
use std::task::{Context, Poll};

use axum::extract::Request;
use axum::http::Uri;
use axum::http::uri::PathAndQuery;
use tower::{Layer, Service};

/// Strip `base` from the front of `path` at a segment boundary.
///
/// Returns the remainder with a leading slash (`/maps/ping` -> `/ping`,
/// `/maps` and `/maps/` -> `/`), or `None` when `path` does not actually sit
/// under `base` (e.g. `/mapsx/ping` is not under `/maps`), in which case the
/// request is left untouched.
#[must_use]
pub fn strip_base_prefix(path: &str, base: &str) -> Option<String> {
    // An empty base means no subfolder stripping (root or proxy-strip mode):
    // a true no-op so the request URI is left exactly as received.
    if base.is_empty() {
        return None;
    }
    let rest = path.strip_prefix(base)?;
    if rest.is_empty() {
        Some("/".to_owned())
    } else if rest.starts_with('/') {
        Some(rest.to_owned())
    } else {
        None
    }
}

/// Tower layer applying [`BasePathStrip`] for a fixed base path.
#[derive(Clone, Debug)]
pub struct BasePathStripLayer {
    base: Arc<str>,
}

impl BasePathStripLayer {
    /// Create a layer that strips `base` (a normalized subfolder such as
    /// `/maps`, with a leading slash and no trailing slash) from every request.
    #[must_use]
    pub fn new(base: impl Into<Arc<str>>) -> Self {
        Self { base: base.into() }
    }
}

impl<S> Layer<S> for BasePathStripLayer {
    type Service = BasePathStrip<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BasePathStrip {
            inner,
            base: Arc::clone(&self.base),
        }
    }
}

/// Service that strips the configured base-path prefix from a request's URI
/// before routing. Must wrap the router from the OUTSIDE so it runs first.
#[derive(Clone, Debug)]
pub struct BasePathStrip<S> {
    inner: S,
    base: Arc<str>,
}

impl<S> Service<Request> for BasePathStrip<S>
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
        if let Some(stripped) = strip_base_prefix(req.uri().path(), &self.base) {
            rewrite_request_path(&mut req, &stripped);
        }
        self.inner.call(req)
    }
}

/// Rewrite `req`'s URI path to `new_path`, preserving the query string. A
/// malformed rebuild leaves the request untouched, so this never drops a
/// request.
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

    // ---- pure: strip_base_prefix ----------------------------------------

    #[test]
    fn strips_prefix_at_segment_boundary() {
        assert_eq!(
            strip_base_prefix("/maps/ping", "/maps").as_deref(),
            Some("/ping")
        );
        assert_eq!(
            strip_base_prefix("/maps/_nuxt/x.js", "/maps").as_deref(),
            Some("/_nuxt/x.js")
        );
    }

    #[test]
    fn bare_prefix_becomes_root() {
        assert_eq!(strip_base_prefix("/maps", "/maps").as_deref(), Some("/"));
        assert_eq!(strip_base_prefix("/maps/", "/maps").as_deref(), Some("/"));
    }

    #[test]
    fn non_matching_prefix_is_left_untouched() {
        // `/mapsx` is NOT under `/maps` — must not be mangled into `x/...`.
        assert_eq!(strip_base_prefix("/mapsx/ping", "/maps"), None);
        assert_eq!(strip_base_prefix("/other/ping", "/maps"), None);
    }

    #[test]
    fn nested_base_path_strips_correctly() {
        assert_eq!(
            strip_base_prefix("/a/b/ping", "/a/b").as_deref(),
            Some("/ping")
        );
    }

    #[test]
    fn empty_base_is_a_noop() {
        // Root / proxy-strip mode: the layer must leave every path untouched.
        assert_eq!(strip_base_prefix("/ping", ""), None);
        assert_eq!(strip_base_prefix("/", ""), None);
        assert_eq!(strip_base_prefix("/_nuxt/x.js", ""), None);
    }

    // ---- service: BasePathStrip -----------------------------------------

    /// Echo the (possibly rewritten) request URI back so tests can assert what
    /// the inner service received.
    async fn echo_uri(base: &str, path: &str) -> String {
        let svc = BasePathStripLayer::new(base).layer(service_fn(|req: Request| async move {
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
    async fn service_strips_api_path() {
        assert_eq!(echo_uri("/maps", "/maps/ping").await, "/ping");
    }

    #[tokio::test]
    async fn service_maps_bare_prefix_to_root() {
        assert_eq!(echo_uri("/maps", "/maps").await, "/");
        assert_eq!(echo_uri("/maps", "/maps/").await, "/");
    }

    #[tokio::test]
    async fn service_preserves_query_string() {
        assert_eq!(
            echo_uri("/maps", "/maps/styles/x/?raster").await,
            "/styles/x/?raster"
        );
    }

    #[tokio::test]
    async fn service_leaves_non_matching_path_untouched() {
        assert_eq!(echo_uri("/maps", "/other/ping").await, "/other/ping");
    }
}
