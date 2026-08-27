//! Runtime rebasing of the embedded single-page application for subfolder
//! deployments.
//!
//! The GUI is built once with a root (`/`) base and embedded in the binary at
//! compile time. When the server is deployed under a URL subfolder (derived
//! from `server.public_url`, e.g. `https://example.com/maps`), the browser
//! must request subfolder-prefixed asset and API URLs. Rather than rebuild the
//! SPA per deployment, the server rewrites the embedded text assets once at
//! startup so every root-absolute reference is prefixed with the base path.
//!
//! The rewrite targets, all of which appear verbatim in the Nuxt/Vite build
//! output, are:
//! - `/_nuxt/` — the build assets directory (hardcoded in `index.html` and in
//!   the chunk graph, so it must be rebased across every text asset).
//! - `/fonts/` — `@nuxt/fonts` preload and `@font-face` `url()` references.
//! - `/favicon.ico` and the inlined `app.baseURL` in `index.html`. Rewriting
//!   `baseURL` makes the Vue Router history base and
//!   `useRuntimeConfig().app.baseURL` (read by the API-URL helper and
//!   `<NuxtLink>`) subfolder-aware at runtime with no client rebuild.
//!
//! For a root deployment (empty base path) every function here is a no-op, so
//! the common case serves byte-identical embedded assets.

/// Text asset extensions whose root-absolute references are rewritten. Binary
/// assets (fonts, images, the favicon itself) are served unchanged.
const TEXT_EXTENSIONS: [&str; 6] = ["html", "js", "css", "json", "xml", "svg"];

/// Returns `true` if the asset at `path` is a text asset whose contents may
/// carry root-absolute references that need rebasing.
#[must_use]
pub fn is_text_asset(path: &str) -> bool {
    path.rsplit('.')
        .next()
        .is_some_and(|ext| TEXT_EXTENSIONS.contains(&ext))
}

/// Rewrite the root-absolute references in a single embedded text asset so
/// they are prefixed with `base_path` (e.g. `/maps`).
///
/// Returns `Some(rewritten_bytes)` when the asset is a text asset and
/// `base_path` is non-empty; otherwise returns `None`, signalling the caller
/// to serve the original embedded bytes unchanged. `base_path` must be a
/// normalized subfolder (leading slash, no trailing slash) as produced by
/// [`crate::config::derive_base_path`]; an empty `base_path` (root deployment)
/// always yields `None`.
#[must_use]
pub fn rewrite_asset(path: &str, content: &[u8], base_path: &str) -> Option<Vec<u8>> {
    if base_path.is_empty() || !is_text_asset(path) {
        return None;
    }
    // Text assets in a Nuxt/Vite build are UTF-8. If a text-extension asset is
    // somehow not valid UTF-8, leave it untouched rather than corrupt it.
    let text = std::str::from_utf8(content).ok()?;

    let mut out = text
        .replace("/_nuxt/", &format!("{base_path}/_nuxt/"))
        .replace("/fonts/", &format!("{base_path}/fonts/"));

    // `index.html` carries two references the prefix rules above do not cover:
    // the favicon link and the inlined runtime `app.baseURL`. Rewriting
    // `baseURL` is what makes the router + API helper subfolder-aware.
    if path == "index.html" {
        out = out
            .replace(
                "href=\"/favicon.ico\"",
                &format!("href=\"{base_path}/favicon.ico\""),
            )
            .replace("baseURL:\"/\"", &format!("baseURL:\"{base_path}/\""));
    }

    Some(out.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_text_asset_matches_known_text_extensions() {
        assert!(is_text_asset("index.html"));
        assert!(is_text_asset("_nuxt/entry.abc.js"));
        assert!(is_text_asset("_nuxt/entry.abc.css"));
        assert!(is_text_asset("wmts.xml"));
        assert!(!is_text_asset("fonts/switzer-300.woff2"));
        assert!(!is_text_asset("favicon.ico"));
        assert!(!is_text_asset("no_extension"));
    }

    #[test]
    fn rewrite_is_noop_for_root_deployment() {
        let html = br#"<script src="/_nuxt/entry.js"></script>"#;
        assert_eq!(rewrite_asset("index.html", html, ""), None);
    }

    #[test]
    fn rewrite_is_noop_for_binary_assets() {
        assert_eq!(
            rewrite_asset("fonts/switzer-300.woff2", &[0u8, 1, 2], "/maps"),
            None
        );
    }

    #[test]
    fn rewrite_prefixes_nuxt_and_fonts_in_js() {
        let js = br#"import"/_nuxt/chunk.js";fetch("/fonts/x.woff2")"#;
        let out = rewrite_asset("_nuxt/entry.js", js, "/maps").expect("text asset rewritten");
        let out = String::from_utf8(out).expect("valid utf8");
        assert_eq!(
            out,
            r#"import"/maps/_nuxt/chunk.js";fetch("/maps/fonts/x.woff2")"#
        );
    }

    #[test]
    fn rewrite_index_html_rebases_base_url_and_favicon() {
        let html = br#"<link rel="icon" href="/favicon.ico"><script>window.__NUXT__.config={app:{baseURL:"/",buildAssetsDir:"/_nuxt/"}}</script><script src="/_nuxt/e.js"></script>"#;
        let out = rewrite_asset("index.html", html, "/maps").expect("index rewritten");
        let out = String::from_utf8(out).expect("valid utf8");
        assert!(out.contains(r#"href="/maps/favicon.ico""#));
        assert!(out.contains(r#"baseURL:"/maps/""#));
        assert!(out.contains(r#"buildAssetsDir:"/maps/_nuxt/""#));
        assert!(out.contains(r#"src="/maps/_nuxt/e.js""#));
        // The original root-absolute forms must be fully gone.
        assert!(!out.contains(r#"href="/favicon.ico""#));
        assert!(!out.contains(r#"baseURL:"/""#));
    }

    #[test]
    fn rewrite_leaves_non_index_favicon_and_baseurl_untouched() {
        // A JS chunk that happens to contain a `baseURL:"/"` literal is NOT the
        // runtime config; only index.html carries the authoritative one, so the
        // baseURL/favicon specials apply to index.html alone.
        let js = br#"const x={baseURL:"/"};load("/_nuxt/a.js")"#;
        let out = rewrite_asset("_nuxt/x.js", js, "/maps").expect("rewritten");
        let out = String::from_utf8(out).expect("valid utf8");
        assert_eq!(out, r#"const x={baseURL:"/"};load("/maps/_nuxt/a.js")"#);
    }
}
