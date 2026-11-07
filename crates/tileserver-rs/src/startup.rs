//! Unified config resolution with auto-detect fallback.
//!
//! Priority order:
//! 1. Explicit `--config path` (fail-fast if missing)
//! 2. Positional `detect_path` (auto-detect from that path)
//! 3. Default config locations (`config.toml`, `/etc/tileserver-rs/config.toml`)
//! 4. CWD auto-detect (scan current directory)

use anyhow::Context;
use axum::{
    Router,
    http::{
        Method, StatusCode,
        header::{ACCEPT, CONTENT_TYPE},
    },
    response::IntoResponse,
    routing::get,
};
use std::{net::SocketAddr, path::PathBuf, time::Duration};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;

use crate::{autodetect::AutoDetectReport, config::Config};

/// Placeholder `admin_bind` meaning "no admin listener".
const ADMIN_DISABLED: &str = "127.0.0.1:0";

/// Hex SHA-256 of the config: the file's own hash when it came from a file,
/// else the hash of the serialized (auto-detected) config.
///
/// # Errors
///
/// Returns an error when the config file cannot be re-read.
pub fn config_hash(path: Option<&PathBuf>, config: &Config) -> anyhow::Result<String> {
    if let Some(path) = path {
        return Ok(Config::load_with_metadata(Some(path.clone()))?.content_hash);
    }
    use sha2::{Digest, Sha256};
    let content = toml::to_string(config).context("failed to serialize the config")?;
    Ok(Sha256::digest(content.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// CORS for the public API: read-only methods, a one-day preflight cache.
///
/// # Errors
///
/// Returns an error when an origin in `cors_origins` is not a valid header.
pub fn cors_layer(cors_origins: &[String]) -> anyhow::Result<CorsLayer> {
    let allow_origin = crate::cors_origin::build_allow_origin(cors_origins)?;
    Ok(CorsLayer::new()
        .allow_headers([ACCEPT, CONTENT_TYPE])
        .max_age(Duration::from_secs(86400))
        .allow_origin(allow_origin)
        .allow_methods([Method::GET, Method::OPTIONS, Method::HEAD]))
}

/// `/openapi.json` plus the self-hosted Scalar reference at `/_openapi`.
///
/// # Errors
///
/// Returns an error when the OpenAPI spec cannot be serialized.
pub fn docs_router<S: Clone + Send + Sync + 'static>() -> anyhow::Result<Router<S>> {
    let mut spec = crate::openapi::ApiDoc::openapi();
    spec.info.version = env!("CARGO_PKG_VERSION").to_string();
    let openapi_json = spec
        .to_pretty_json()
        .context("failed to serialize the OpenAPI spec")?;
    let scalar_html = scalar_api_reference::axum::scalar_response(
        &serde_json::json!({ "url": "/openapi.json", "layout": "classic" }),
        Some("/_openapi/scalar.js"),
    );
    Ok(Router::new()
        .route(
            "/openapi.json",
            get(move || async move { ([(CONTENT_TYPE, "application/json")], openapi_json) }),
        )
        .route("/_openapi", get(move || async move { scalar_html }))
        .route("/_openapi/scalar.js", get(scalar_asset)))
}

async fn scalar_asset() -> axum::response::Response {
    match scalar_api_reference::get_asset_with_mime("scalar.js") {
        Some((mime, content)) => (StatusCode::OK, [(CONTENT_TYPE, mime)], content).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Parse `admin_bind`: `None` when the admin listener is disabled.
///
/// # Errors
///
/// Returns an error when `admin_bind` is not a socket address.
pub fn admin_addr(admin_bind: &str) -> anyhow::Result<Option<SocketAddr>> {
    if admin_bind == ADMIN_DISABLED {
        return Ok(None);
    }
    let addr: SocketAddr = admin_bind
        .parse()
        .with_context(|| format!("invalid admin_bind '{admin_bind}'"))?;
    if !addr.ip().is_loopback() {
        tracing::warn!(
            bind = %addr,
            "admin_bind is NOT a loopback address — destructive admin endpoints (/__admin/reload, /__admin/cache/flush, /__admin/oauth/*) are reachable on ALL interfaces. \
             In production, set [server].admin_bind to '127.0.0.1:<port>' and reverse-proxy with auth. \
             See https://github.com/vinayakkulkarni/tileserver-rs/blob/main/apps/docs/content/4.guides/16.mcp.md#admin-security"
        );
    }
    Ok(Some(addr))
}

/// Resolve the runtime configuration and optional auto-detect report.
///
/// # Errors
///
/// Returns an error if an explicit config path does not exist or the
/// auto-detect target is unreadable.
pub fn load_runtime_config(
    config_path: Option<PathBuf>,
    detect_path: Option<PathBuf>,
) -> anyhow::Result<(Config, Option<AutoDetectReport>)> {
    // 1. Explicit config — fail-fast if missing
    if let Some(path) = config_path {
        if !path.exists() {
            anyhow::bail!("Config file not found: {}", path.display());
        }
        return Ok((Config::load(Some(path))?, None));
    }

    // 2. Positional detect path
    if let Some(path) = detect_path {
        let (config, report) = crate::autodetect::detect_config(path)?;
        return Ok((config, Some(report)));
    }

    // 3. Default config locations
    let default_paths = vec![
        PathBuf::from("config.toml"),
        PathBuf::from("/etc/tileserver-rs/config.toml"),
    ];

    for path in default_paths {
        if path.exists() {
            return Ok((Config::load(Some(path))?, None));
        }
    }

    // 4. CWD auto-detect
    let cwd = std::env::current_dir().context("failed to get current working directory")?;
    let (config, report) = crate::autodetect::detect_config(cwd)?;
    Ok((config, Some(report)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn config_hash_is_stable_hex_and_tracks_content() {
        let a = Config::default();
        let first = config_hash(None, &a).unwrap();
        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(first, config_hash(None, &a).unwrap());
        let mut b = Config::default();
        b.server.port += 1;
        assert_ne!(first, config_hash(None, &b).unwrap());
    }

    #[test]
    fn config_hash_of_a_file_uses_the_file_hash() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("config.toml");
        std::fs::write(&path, "[server]\nport = 9000\n").unwrap();
        let expected = Config::load_with_metadata(Some(path.clone()))
            .unwrap()
            .content_hash;
        assert_eq!(
            config_hash(Some(&path), &Config::default()).unwrap(),
            expected
        );
    }

    #[test]
    fn admin_addr_parses_disabled_loopback_public_and_invalid() {
        assert_eq!(admin_addr(ADMIN_DISABLED).unwrap(), None);
        assert_eq!(
            admin_addr("127.0.0.1:9001").unwrap(),
            Some("127.0.0.1:9001".parse().unwrap())
        );
        assert_eq!(
            admin_addr("0.0.0.0:9001").unwrap(),
            Some("0.0.0.0:9001".parse().unwrap())
        );
        assert!(admin_addr("not-an-address").is_err());
    }

    #[test]
    fn cors_layer_rejects_an_invalid_origin_pattern() {
        assert!(cors_layer(&["*".to_string()]).is_ok());
        assert!(cors_layer(&["/(unclosed/".to_string()]).is_err());
    }

    #[tokio::test]
    async fn docs_router_serves_spec_reference_and_script() {
        let server = axum_test::TestServer::new(docs_router::<()>().unwrap());
        let spec: serde_json::Value = server.get("/openapi.json").await.json();
        assert_eq!(spec["info"]["version"], env!("CARGO_PKG_VERSION"));
        assert!(
            server
                .get("/_openapi")
                .await
                .text()
                .contains("/openapi.json")
        );
        let js = server.get("/_openapi/scalar.js").await;
        js.assert_status_ok();
        assert!(!js.as_bytes().is_empty());
    }

    #[test]
    fn test_explicit_config_takes_precedence_over_auto_detect() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let config_path = root.join("config.toml");
        std::fs::write(
            &config_path,
            r#"
                [[sources]]
                id = "from-config"
                type = "pmtiles"
                path = "/tmp/from-config.pmtiles"
            "#,
        )
        .unwrap();

        std::fs::write(root.join("detected.pmtiles"), b"mock").unwrap();

        let (config, report) =
            load_runtime_config(Some(config_path), Some(root.to_path_buf())).unwrap();

        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].id, "from-config");
        assert!(report.is_none());
    }

    #[test]
    fn test_missing_explicit_config_fails() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("missing-config.toml");

        let result = load_runtime_config(Some(missing.clone()), None);

        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(msg.contains("Config file not found"));
        assert!(msg.contains(missing.to_string_lossy().as_ref()));
    }

    #[test]
    fn test_positional_detect_path_auto_detects() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        std::fs::write(root.join("detected.pmtiles"), b"mock").unwrap();

        let (config, report) =
            load_runtime_config(None, Some(root.to_path_buf())).expect("detect must succeed");

        assert!(
            report.is_some(),
            "positional detect path must produce an auto-detect report"
        );
        // The auto-detect walk registers the mock archive as a source.
        assert_eq!(config.sources.len(), 1);
    }

    #[test]
    fn test_missing_detect_path_fails() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("no-such-dir");

        let result = load_runtime_config(None, Some(missing));

        assert!(
            result.is_err(),
            "auto-detect on an unreadable target must error"
        );
    }
}
