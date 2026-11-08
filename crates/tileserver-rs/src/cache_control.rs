//! HTTP cache-control header helpers for tile and static asset responses.

use axum::http::HeaderValue;

/// Set cache headers for tile responses
#[must_use]
pub fn tile_cache_headers() -> HeaderValue {
    HeaderValue::from_static("public, max-age=86400, stale-while-revalidate=604800")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_cache_headers_is_public() {
        let hv = tile_cache_headers();
        let s = hv.to_str().unwrap();
        assert!(s.contains("public"));
    }

    #[test]
    fn test_tile_cache_headers_max_age() {
        let hv = tile_cache_headers();
        let s = hv.to_str().unwrap();
        assert!(s.contains("max-age=86400"));
    }

    #[test]
    fn test_tile_cache_headers_stale_while_revalidate() {
        let hv = tile_cache_headers();
        let s = hv.to_str().unwrap();
        assert!(s.contains("stale-while-revalidate=604800"));
    }
}
