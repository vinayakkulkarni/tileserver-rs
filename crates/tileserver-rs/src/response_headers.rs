//! User-defined HTTP response header injection.
//!
//! Wires the `[server.extra_response_headers]` config block into the Axum
//! router as a stack of [`tower_http::set_header::SetResponseHeaderLayer`]
//! values. Each configured header becomes one layer; the layers are
//! `overriding` rather than `appending` so a user value wins over any
//! header that an upstream middleware would have otherwise emitted.
//!
//! Validation (RFC 7230 token grammar + reserved-header rejection) happens
//! upstream in [`crate::config::Config::validate`]; this module trusts that
//! the names + values are well-formed `HeaderName` / `HeaderValue`s. Any
//! still-malformed name or value here is logged and skipped so a single
//! bad row never tanks the whole router build.

use axum::Router;
use axum::http::{HeaderName, HeaderValue};
use std::collections::HashMap;
use tower_http::set_header::SetResponseHeaderLayer;

/// Apply user-defined response headers (`[server.extra_response_headers]`)
/// to a router as a stack of overriding `SetResponseHeaderLayer`s.
///
/// `None` or an empty map is a no-op — the router is returned unchanged.
///
/// Per spec, an empty-string value DELETES any pre-existing header of
/// that name from outgoing responses (a `SetResponseHeaderLayer` with an
/// empty `HeaderValue` does this naturally).
pub fn apply_extra_response_headers<S>(
    mut router: Router<S>,
    headers: Option<&HashMap<String, String>>,
) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let Some(headers) = headers else {
        return router;
    };
    if headers.is_empty() {
        return router;
    }

    for (raw_name, raw_value) in headers {
        let name = match HeaderName::try_from(raw_name.as_str()) {
            Ok(n) => n,
            Err(err) => {
                tracing::warn!(
                    header = %raw_name,
                    error = %err,
                    "skipping extra_response_header with invalid name (should have been caught by Config::validate)"
                );
                continue;
            }
        };
        let value = match HeaderValue::try_from(raw_value.as_str()) {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(
                    header = %raw_name,
                    error = %err,
                    "skipping extra_response_header with invalid value"
                );
                continue;
            }
        };
        router = router.layer(SetResponseHeaderLayer::overriding(name, value));
    }
    router
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, routing::get};
    use axum_test::TestServer;

    fn router_with(headers: Option<HashMap<String, String>>) -> Router {
        let r: Router<()> = Router::new().route("/ping", get(|| async { "pong" }));
        apply_extra_response_headers(r, headers.as_ref())
    }

    #[tokio::test]
    async fn no_headers_is_no_op() {
        let server = TestServer::new(router_with(None));
        let resp = server.get("/ping").await;
        resp.assert_status_ok();
        resp.assert_text("pong");
    }

    #[tokio::test]
    async fn empty_map_is_no_op() {
        let server = TestServer::new(router_with(Some(HashMap::new())));
        let resp = server.get("/ping").await;
        resp.assert_status_ok();
    }

    #[tokio::test]
    async fn single_header_appears_on_response() {
        let mut h = HashMap::new();
        h.insert("X-Custom".to_string(), "value1".to_string());
        let server = TestServer::new(router_with(Some(h)));
        let resp = server.get("/ping").await;
        resp.assert_status_ok();
        let v = resp.header("X-Custom");
        assert_eq!(v.to_str().unwrap(), "value1");
    }

    #[tokio::test]
    async fn multiple_headers_all_appear() {
        let mut h = HashMap::new();
        h.insert("X-Foo".to_string(), "foo-val".to_string());
        h.insert("X-Bar".to_string(), "bar-val".to_string());
        h.insert(
            "Cache-Control".to_string(),
            "public, max-age=3600".to_string(),
        );
        let server = TestServer::new(router_with(Some(h)));
        let resp = server.get("/ping").await;
        resp.assert_status_ok();
        assert_eq!(resp.header("X-Foo").to_str().unwrap(), "foo-val");
        assert_eq!(resp.header("X-Bar").to_str().unwrap(), "bar-val");
        assert_eq!(
            resp.header("Cache-Control").to_str().unwrap(),
            "public, max-age=3600"
        );
    }

    #[tokio::test]
    async fn invalid_name_is_skipped_not_panicked() {
        // Config::validate should reject these upstream — but if one
        // slips through, the layer build must not panic the server.
        let mut h = HashMap::new();
        h.insert("Bad Name With Space".to_string(), "value".to_string());
        h.insert("X-Good".to_string(), "ok".to_string());
        let server = TestServer::new(router_with(Some(h)));
        let resp = server.get("/ping").await;
        resp.assert_status_ok();
        assert_eq!(resp.header("X-Good").to_str().unwrap(), "ok");
    }
}
