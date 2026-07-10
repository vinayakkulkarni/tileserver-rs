//! Style management, URL rewriting for native rendering, and style JSON processing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::StyleConfig;
use crate::error::{Result, TileServerError};
use crate::sources::SourceManager;

/// Style metadata returned by /styles.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleInfo {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A loaded map style
#[derive(Debug, Clone)]
pub struct Style {
    pub id: String,
    pub name: String,
    pub style_json: serde_json::Value,
    /// Path to the style.json file (used to locate sprites)
    pub path: PathBuf,
}

impl Style {
    /// Load a style from a file path
    pub fn from_file(config: &StyleConfig) -> Result<Self> {
        let path = Path::new(&config.path);

        if !path.exists() {
            return Err(TileServerError::StyleNotFound(config.id.clone()));
        }

        let content = std::fs::read_to_string(path).map_err(TileServerError::FileError)?;

        let style_json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| TileServerError::MetadataError(format!("Invalid style JSON: {}", e)))?;

        let name = config
            .name
            .clone()
            .or_else(|| {
                style_json
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| config.id.clone());

        Ok(Self {
            id: config.id.clone(),
            name,
            style_json,
            path: config.path.clone(),
        })
    }

    /// Convert to StyleInfo for API response
    #[must_use]
    pub fn to_info(&self, base_url: &str) -> StyleInfo {
        self.to_info_with_key(base_url, None)
    }

    /// Convert to StyleInfo for API response with optional API key
    #[must_use]
    pub fn to_info_with_key(&self, base_url: &str, key: Option<&str>) -> StyleInfo {
        let key_query = key
            .map(|k| format!("?key={}", urlencoding::encode(k)))
            .unwrap_or_default();

        StyleInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            url: Some(format!(
                "{}/styles/{}/style.json{}",
                base_url, self.id, key_query
            )),
        }
    }
}

/// Manages all map styles
pub struct StyleManager {
    styles: HashMap<String, Style>,
}

impl StyleManager {
    /// Create a new empty style manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            styles: HashMap::new(),
        }
    }

    /// Load styles from configuration
    pub fn from_configs(configs: &[StyleConfig]) -> Result<Self> {
        let mut manager = Self::new();

        for config in configs {
            match Style::from_file(config) {
                Ok(style) => {
                    tracing::info!("Loaded style: {} ({})", config.id, config.path.display());
                    manager.styles.insert(config.id.clone(), style);
                }
                Err(e) => {
                    tracing::error!("Failed to load style {}: {}", config.id, e);
                    // Continue loading other styles
                }
            }
        }

        Ok(manager)
    }

    /// Get a style by ID
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Style> {
        self.styles.get(id)
    }

    /// Get all style infos for API response
    #[must_use]
    pub fn all_infos(&self, base_url: &str) -> Vec<StyleInfo> {
        self.all_infos_with_key(base_url, None)
    }

    /// Get all style infos for API response with optional API key
    #[must_use]
    pub fn all_infos_with_key(&self, base_url: &str, key: Option<&str>) -> Vec<StyleInfo> {
        self.styles
            .values()
            .map(|s| s.to_info_with_key(base_url, key))
            .collect()
    }

    /// Get all styles
    #[must_use]
    pub fn all(&self) -> Vec<&Style> {
        self.styles.values().collect()
    }

    /// Get the number of styles
    #[must_use]
    pub fn len(&self) -> usize {
        self.styles.len()
    }

    /// Check if there are no styles
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }
}

impl Default for StyleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Query parameters to forward to rewritten URLs (like API keys)
#[derive(Debug, Clone, Default)]
pub struct UrlQueryParams {
    /// API key parameter (e.g., `key=abc123`)
    pub key: Option<String>,
    /// Additional query parameters to forward
    pub extra: Vec<(String, String)>,
}

impl UrlQueryParams {
    /// Create new query params with just a key
    #[must_use]
    pub fn with_key(key: Option<String>) -> Self {
        Self {
            key,
            extra: Vec::new(),
        }
    }

    /// Build query string to append to URLs
    /// Returns empty string if no params, otherwise "?key=value&..."
    #[must_use]
    pub fn to_query_string(&self) -> String {
        let mut params = Vec::new();

        if let Some(ref key) = self.key {
            params.push(format!("key={}", urlencoding::encode(key)));
        }

        for (k, v) in &self.extra {
            params.push(format!(
                "{}={}",
                urlencoding::encode(k),
                urlencoding::encode(v)
            ));
        }

        if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        }
    }
}

/// Extract a tileserver data source id from a style source `url`.
///
/// Accepts the relative (`/data/protomaps-mlt`, `/data/protomaps.json`) and
/// absolute (`http://host/data/protomaps`) forms, ignoring any trailing query
/// string. Returns `None` for URLs that do not reference our `/data/` endpoint.
fn data_source_id_from_url(url: &str) -> Option<&str> {
    let without_query = url.split('?').next().unwrap_or(url);
    let after_data = if let Some(rest) = without_query.strip_prefix("/data/") {
        rest
    } else if without_query.contains("/data/") {
        without_query.rsplit("/data/").next()?
    } else {
        return None;
    };
    let id = after_data.strip_suffix(".json").unwrap_or(after_data);
    (!id.is_empty()).then_some(id)
}

/// Rewrite a style JSON to use absolute URLs for the public API.
///
/// This function replaces relative URLs (like `/data/protomaps.json`)
/// with absolute URLs (like `http://localhost:8080/data/protomaps.json?key=API_KEY`).
///
/// This is essential for:
/// 1. External clients that need absolute URLs
/// 2. API key support via query parameters (forwarded from the original request)
/// 3. Cross-origin usage
///
/// Similar to tileserver-gl's `fixUrl()` function, this:
/// - Converts relative URLs to absolute
/// - Preserves and forwards query parameters (like `?key=...`)
#[must_use]
pub fn rewrite_style_for_api(
    style_json: &serde_json::Value,
    base_url: &str,
    query_params: &UrlQueryParams,
    sources: &SourceManager,
) -> serde_json::Value {
    let mut style = style_json.clone();
    let query_string = query_params.to_query_string();

    // Helper to rewrite a relative URL to absolute with query params
    let rewrite_url = |url_str: &str| -> String {
        if url_str.starts_with('/') {
            format!("{}{}{}", base_url, url_str, query_string)
        } else {
            url_str.to_string()
        }
    };

    // Rewrite sources - convert relative URLs to absolute
    if let Some(style_sources) = style.get_mut("sources")
        && let Some(sources_obj) = style_sources.as_object_mut()
    {
        for (_source_id, source_config) in sources_obj.iter_mut() {
            if let Some(source_obj) = source_config.as_object_mut() {
                // Resolve whether the referenced source serves MLT before we
                // rewrite its url. maplibre-gl-js cannot render our MLT tiles:
                // its bundled `@maplibre/mlt` decoder rejects the geometry
                // encodings mlt-core emits ("the specified geometry type is
                // currently not supported"), confirmed against every
                // `EncoderConfig` variant. So MLT sources are pointed at the
                // `.pbf` transcode endpoint below instead of the `.mlt` tiles
                // their TileJSON advertises.
                let mlt_source_id = source_obj
                    .get("url")
                    .and_then(|v| v.as_str())
                    .and_then(data_source_id_from_url)
                    .filter(|id| {
                        sources
                            .get(id)
                            .is_some_and(|s| s.metadata().format == crate::sources::TileFormat::Mlt)
                    })
                    .map(str::to_string);

                // Rewrite "url" field if relative
                if let Some(url) = source_obj.get_mut("url")
                    && let Some(url_str) = url.as_str()
                {
                    *url = serde_json::Value::String(rewrite_url(url_str));
                }

                // Rewrite "tiles" array if relative
                if let Some(tiles) = source_obj.get_mut("tiles")
                    && let Some(tiles_arr) = tiles.as_array_mut()
                {
                    for tile in tiles_arr.iter_mut() {
                        if let Some(tile_str) = tile.as_str() {
                            *tile = serde_json::Value::String(rewrite_url(tile_str));
                        }
                    }
                }

                // Point MLT sources at the `.pbf` transcode endpoint so the
                // viewer receives standard MVT it can render. The data handler
                // converts MLT->MVT on the fly; the raw `.mlt` endpoint stays
                // available to direct API clients. Mirrors the native-render
                // rewrite in `rewrite_source`.
                if let Some(id) = mlt_source_id {
                    let tile_url = format!(
                        "{}/data/{}/{{z}}/{{x}}/{{y}}.pbf{}",
                        base_url, id, query_string
                    );
                    source_obj.remove("url");
                    source_obj.insert("tiles".to_string(), serde_json::json!([tile_url]));

                    if let Some(source) = sources.get(&id) {
                        let metadata = source.metadata();
                        if !source_obj.contains_key("minzoom") {
                            source_obj
                                .insert("minzoom".to_string(), serde_json::json!(metadata.minzoom));
                        }
                        if !source_obj.contains_key("maxzoom") {
                            source_obj
                                .insert("maxzoom".to_string(), serde_json::json!(metadata.maxzoom));
                        }
                        if !source_obj.contains_key("bounds")
                            && let Some(bounds) = &metadata.bounds
                        {
                            source_obj.insert("bounds".to_string(), serde_json::json!(bounds));
                        }
                    }
                }
            }
        }
    }

    // Rewrite glyphs URL if relative
    if let Some(glyphs) = style.get_mut("glyphs")
        && let Some(glyphs_str) = glyphs.as_str()
    {
        *glyphs = serde_json::Value::String(rewrite_url(glyphs_str));
    }

    // Rewrite sprite URL if relative
    if let Some(sprite) = style.get_mut("sprite")
        && let Some(sprite_str) = sprite.as_str()
    {
        *sprite = serde_json::Value::String(rewrite_url(sprite_str));
    }

    style
}

/// Rewrite a style JSON to inline tile URLs for native rendering.
///
/// This function replaces relative source URLs (like `/data/protomaps.json`)
/// with inline tile URL templates that MapLibre Native can use directly.
///
/// The native renderer cannot fetch TileJSON from our server (same process),
/// so we need to embed the tile URLs directly in the style.
/// This also rewrites relative glyphs and sprite URLs to absolute URLs.
pub fn rewrite_style_for_native(
    style_json: &serde_json::Value,
    base_url: &str,
    sources: &SourceManager,
) -> serde_json::Value {
    let mut style = style_json.clone();

    // Rewrite sources - inline tile URLs
    if let Some(style_sources) = style.get_mut("sources")
        && let Some(sources_obj) = style_sources.as_object_mut()
    {
        for (source_id, source_config) in sources_obj.iter_mut() {
            rewrite_source(source_id, source_config, base_url, sources);
        }
    }

    // Rewrite glyphs URL if it's relative
    if let Some(glyphs) = style.get_mut("glyphs")
        && let Some(glyphs_str) = glyphs.as_str()
        && glyphs_str.starts_with('/')
    {
        let absolute_url = format!("{}{}", base_url, glyphs_str);
        tracing::debug!("Rewriting glyphs URL: {} -> {}", glyphs_str, absolute_url);
        *glyphs = serde_json::Value::String(absolute_url);
    }

    // Rewrite sprite URL if it's relative
    if let Some(sprite) = style.get_mut("sprite")
        && let Some(sprite_str) = sprite.as_str()
        && sprite_str.starts_with('/')
    {
        let absolute_url = format!("{}{}", base_url, sprite_str);
        tracing::debug!("Rewriting sprite URL: {} -> {}", sprite_str, absolute_url);
        *sprite = serde_json::Value::String(absolute_url);
    }

    style
}

/// Rewrite a single source to inline tile URLs
fn rewrite_source(
    source_id: &str,
    source_config: &mut serde_json::Value,
    base_url: &str,
    sources: &SourceManager,
) {
    let source_obj = match source_config.as_object_mut() {
        Some(obj) => obj,
        None => return,
    };

    // Check if this source has a URL that references our data endpoint
    let url = match source_obj.get("url") {
        Some(serde_json::Value::String(url)) => url.clone(),
        _ => return,
    };

    // Check if this is a reference to our data endpoint
    // Supports both with and without .json suffix:
    //   "/data/protomaps.json" or "/data/protomaps"
    //   "http://localhost:8080/data/protomaps.json" or "http://localhost:8080/data/protomaps"
    let data_source_id = if let Some(rest) = url.strip_prefix("/data/") {
        // "/data/protomaps.json" -> "protomaps" or "/data/protomaps" -> "protomaps"
        Some(rest.strip_suffix(".json").unwrap_or(rest))
    } else if url.contains("/data/") {
        // "http://host/data/protomaps.json" or "http://host/data/protomaps"
        url.rsplit("/data/")
            .next()
            .map(|s| s.strip_suffix(".json").unwrap_or(s))
    } else {
        None
    };

    let data_source_id = match data_source_id {
        Some(id) if !id.is_empty() => id,
        _ => return, // Not a reference to our data endpoint
    };

    // Look up the source metadata
    let tile_source = match sources.get(data_source_id) {
        Some(s) => s,
        None => {
            tracing::warn!(
                "Style references source '{}' via URL '{}', but source not found",
                source_id,
                url
            );
            return;
        }
    };

    let metadata = tile_source.metadata();

    // MapLibre Native has an MLT render path (maplibre-native PR #3246), but it
    // only engages when the source carries `encoding: "mlt"`, and that hint is
    // unreliable upstream (maplibre-native issue #4341). Our rewriter inlines
    // `tiles` without an encoding field, so the renderer would default to the
    // MVT decoder and fail on MLT bytes. Point its tile URL at the .pbf endpoint
    // instead, which transparently transcodes MLT -> MVT on the fly.
    let native_extension = if metadata.format == crate::sources::TileFormat::Mlt {
        "pbf"
    } else {
        metadata.format.extension()
    };

    let tile_url = format!(
        "{}/data/{}/{{z}}/{{x}}/{{y}}.{}",
        base_url, data_source_id, native_extension
    );

    tracing::debug!(
        "Rewriting source '{}' from URL '{}' to tiles ['{}']",
        source_id,
        url,
        tile_url
    );

    source_obj.remove("url");
    source_obj.insert("tiles".to_string(), serde_json::json!([tile_url]));

    // Add additional metadata if not already present
    if !source_obj.contains_key("minzoom") {
        source_obj.insert("minzoom".to_string(), serde_json::json!(metadata.minzoom));
    }
    if !source_obj.contains_key("maxzoom") {
        source_obj.insert("maxzoom".to_string(), serde_json::json!(metadata.maxzoom));
    }
    if !source_obj.contains_key("bounds")
        && let Some(bounds) = &metadata.bounds
    {
        source_obj.insert("bounds".to_string(), serde_json::json!(bounds));
    }
    if !source_obj.contains_key("attribution")
        && let Some(attribution) = &metadata.attribution
    {
        source_obj.insert("attribution".to_string(), serde_json::json!(attribution));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_url_query_params_empty() {
        let params = UrlQueryParams::default();
        assert_eq!(params.to_query_string(), "");
    }

    #[test]
    fn test_url_query_params_with_key() {
        let params = UrlQueryParams::with_key(Some("my_api_key".to_string()));
        assert_eq!(params.to_query_string(), "?key=my_api_key");
    }

    #[test]
    fn test_url_query_params_with_special_chars() {
        let params = UrlQueryParams::with_key(Some("key with spaces & symbols=".to_string()));
        assert_eq!(
            params.to_query_string(),
            "?key=key%20with%20spaces%20%26%20symbols%3D"
        );
    }

    #[test]
    fn test_url_query_params_with_extra() {
        let params = UrlQueryParams {
            key: Some("abc".to_string()),
            extra: vec![("foo".to_string(), "bar".to_string())],
        };
        assert_eq!(params.to_query_string(), "?key=abc&foo=bar");
    }

    #[test]
    fn test_rewrite_style_for_api_no_params() {
        let style = json!({
            "version": 8,
            "sources": {
                "openmaptiles": {
                    "type": "vector",
                    "url": "/data/openmaptiles.json"
                }
            },
            "glyphs": "/fonts/{fontstack}/{range}.pbf",
            "sprite": "/styles/basic/sprite"
        });

        let params = UrlQueryParams::default();
        let result = rewrite_style_for_api(
            &style,
            "http://tiles.example.com",
            &params,
            &SourceManager::new(),
        );

        assert_eq!(
            result["sources"]["openmaptiles"]["url"],
            "http://tiles.example.com/data/openmaptiles.json"
        );
        assert_eq!(
            result["glyphs"],
            "http://tiles.example.com/fonts/{fontstack}/{range}.pbf"
        );
        assert_eq!(
            result["sprite"],
            "http://tiles.example.com/styles/basic/sprite"
        );
    }

    #[test]
    fn test_rewrite_style_for_api_with_key() {
        let style = json!({
            "version": 8,
            "sources": {
                "openmaptiles": {
                    "type": "vector",
                    "url": "/data/openmaptiles.json"
                }
            },
            "glyphs": "/fonts/{fontstack}/{range}.pbf",
            "sprite": "/styles/basic/sprite"
        });

        let params = UrlQueryParams::with_key(Some("my_secret_key".to_string()));
        let result = rewrite_style_for_api(
            &style,
            "http://tiles.example.com",
            &params,
            &SourceManager::new(),
        );

        assert_eq!(
            result["sources"]["openmaptiles"]["url"],
            "http://tiles.example.com/data/openmaptiles.json?key=my_secret_key"
        );
        assert_eq!(
            result["glyphs"],
            "http://tiles.example.com/fonts/{fontstack}/{range}.pbf?key=my_secret_key"
        );
        assert_eq!(
            result["sprite"],
            "http://tiles.example.com/styles/basic/sprite?key=my_secret_key"
        );
    }

    #[test]
    fn test_rewrite_style_for_api_preserves_absolute_urls() {
        let style = json!({
            "version": 8,
            "sources": {
                "external": {
                    "type": "vector",
                    "url": "https://external-tiles.com/tiles.json"
                }
            },
            "glyphs": "https://fonts.example.com/{fontstack}/{range}.pbf"
        });

        let params = UrlQueryParams::with_key(Some("key123".to_string()));
        let result = rewrite_style_for_api(
            &style,
            "http://tiles.example.com",
            &params,
            &SourceManager::new(),
        );

        // External URLs should NOT be modified
        assert_eq!(
            result["sources"]["external"]["url"],
            "https://external-tiles.com/tiles.json"
        );
        assert_eq!(
            result["glyphs"],
            "https://fonts.example.com/{fontstack}/{range}.pbf"
        );
    }

    #[test]
    fn test_rewrite_style_for_api_with_tiles_array() {
        let style = json!({
            "version": 8,
            "sources": {
                "osm": {
                    "type": "vector",
                    "tiles": [
                        "/data/osm/{z}/{x}/{y}.pbf",
                        "/backup/osm/{z}/{x}/{y}.pbf"
                    ]
                }
            }
        });

        let params = UrlQueryParams::with_key(Some("abc".to_string()));
        let result = rewrite_style_for_api(
            &style,
            "http://localhost:8080",
            &params,
            &SourceManager::new(),
        );

        let tiles = result["sources"]["osm"]["tiles"].as_array().unwrap();
        assert_eq!(
            tiles[0],
            "http://localhost:8080/data/osm/{z}/{x}/{y}.pbf?key=abc"
        );
        assert_eq!(
            tiles[1],
            "http://localhost:8080/backup/osm/{z}/{x}/{y}.pbf?key=abc"
        );
    }

    #[test]
    fn test_rewrite_style_for_api_mixed_sources() {
        let style = json!({
            "version": 8,
            "sources": {
                "local": {
                    "type": "vector",
                    "url": "/data/local.json"
                },
                "external": {
                    "type": "raster",
                    "tiles": ["https://external.com/{z}/{x}/{y}.png"]
                }
            }
        });

        let params = UrlQueryParams::with_key(Some("test".to_string()));
        let result =
            rewrite_style_for_api(&style, "http://localhost", &params, &SourceManager::new());

        // Local URL should be rewritten
        assert_eq!(
            result["sources"]["local"]["url"],
            "http://localhost/data/local.json?key=test"
        );
        // External URL should NOT be modified
        assert_eq!(
            result["sources"]["external"]["tiles"][0],
            "https://external.com/{z}/{x}/{y}.png"
        );
    }

    #[test]
    fn test_style_info_to_info() {
        let style = Style {
            id: "my-style".to_string(),
            name: "My Style".to_string(),
            style_json: json!({}),
            path: PathBuf::from("/styles/my-style/style.json"),
        };

        let info = style.to_info("http://localhost:8080");
        assert_eq!(info.id, "my-style");
        assert_eq!(info.name, "My Style");
        assert_eq!(
            info.url,
            Some("http://localhost:8080/styles/my-style/style.json".to_string())
        );
    }

    use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};
    use async_trait::async_trait;
    use bytes::Bytes;
    use std::sync::Arc;

    struct FmtSource(TileMetadata);

    #[async_trait]
    impl TileSource for FmtSource {
        async fn get_tile(&self, _z: u8, _x: u32, _y: u32) -> Result<Option<TileData>> {
            Ok(Some(TileData {
                data: Bytes::from_static(b"x"),
                format: self.0.format,
                compression: TileCompression::None,
            }))
        }
        fn metadata(&self) -> &TileMetadata {
            &self.0
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    fn manager_with_source(id: &str, format: TileFormat) -> SourceManager {
        let meta = TileMetadata {
            id: id.to_string(),
            name: id.to_string(),
            description: None,
            attribution: None,
            format,
            minzoom: 0,
            maxzoom: 14,
            bounds: None,
            center: None,
            vector_layers: None,
        };
        let mut map: HashMap<String, Arc<dyn TileSource>> = HashMap::new();
        map.insert(
            id.to_string(),
            Arc::new(FmtSource(meta)) as Arc<dyn TileSource>,
        );
        SourceManager::from_sources(map)
    }

    #[test]
    fn test_rewrite_style_for_native_mlt_source_uses_pbf_extension() {
        // MapLibre Native cannot decode MLT tiles, so an MLT source must be
        // rewritten to the .pbf (transcoded MVT) endpoint and must NOT carry
        // an `encoding: mlt` hint — the renderer receives standard MVT.
        let mgr = manager_with_source("india-mlt", TileFormat::Mlt);
        let style = json!({
            "version": 8,
            "sources": {
                "india-mlt": { "type": "vector", "url": "/data/india-mlt" }
            }
        });

        let result = rewrite_style_for_native(&style, "http://localhost:8080", &mgr);
        let src = &result["sources"]["india-mlt"];

        let tiles = src["tiles"].as_array().unwrap();
        assert_eq!(
            tiles[0],
            "http://localhost:8080/data/india-mlt/{z}/{x}/{y}.pbf"
        );
        assert!(
            src.get("encoding").is_none(),
            "native MLT rewrite must not emit an encoding hint, got: {:?}",
            src.get("encoding")
        );
    }

    #[test]
    fn test_rewrite_style_for_native_pbf_source_unchanged() {
        let mgr = manager_with_source("osm", TileFormat::Pbf);
        let style = json!({
            "version": 8,
            "sources": {
                "osm": { "type": "vector", "url": "/data/osm" }
            }
        });

        let result = rewrite_style_for_native(&style, "http://localhost:8080", &mgr);
        let src = &result["sources"]["osm"];

        let tiles = src["tiles"].as_array().unwrap();
        assert_eq!(tiles[0], "http://localhost:8080/data/osm/{z}/{x}/{y}.pbf");
        assert!(src.get("encoding").is_none());
    }

    #[test]
    fn test_data_source_id_from_url_forms() {
        assert_eq!(
            data_source_id_from_url("/data/protomaps-mlt"),
            Some("protomaps-mlt")
        );
        assert_eq!(
            data_source_id_from_url("/data/protomaps.json"),
            Some("protomaps")
        );
        assert_eq!(
            data_source_id_from_url("http://host:8080/data/india-mlt?key=abc"),
            Some("india-mlt")
        );
        assert_eq!(
            data_source_id_from_url("https://external.com/tiles.json"),
            None
        );
        assert_eq!(data_source_id_from_url("/data/"), None);
    }

    #[test]
    fn test_rewrite_style_for_api_mlt_source_uses_pbf_tiles() {
        // The demo source URL is `/data/protomaps-mlt` (a TileJSON `url` ref
        // whose `-mlt` is a hyphen suffix, not a `.mlt` extension). maplibre-gl
        // cannot render our MLT, so the source must be rewritten to the `.pbf`
        // transcode endpoint with no `url` and no `encoding: mlt` hint.
        let mgr = manager_with_source("protomaps-mlt", TileFormat::Mlt);
        let style = json!({
            "version": 8,
            "sources": {
                "protomaps": { "type": "vector", "url": "/data/protomaps-mlt" }
            }
        });

        let params = UrlQueryParams::default();
        let result = rewrite_style_for_api(&style, "http://localhost:8080", &params, &mgr);
        let src = &result["sources"]["protomaps"];

        assert!(src.get("url").is_none(), "url must be replaced by tiles");
        assert!(
            src.get("encoding").is_none(),
            "no encoding hint: the viewer receives transcoded MVT, not MLT"
        );
        let tiles = src["tiles"].as_array().unwrap();
        assert_eq!(
            tiles[0],
            "http://localhost:8080/data/protomaps-mlt/{z}/{x}/{y}.pbf"
        );
        assert_eq!(src["minzoom"], 0);
        assert_eq!(src["maxzoom"], 14);
    }

    #[test]
    fn test_rewrite_style_for_api_mlt_pbf_tiles_forward_key() {
        let mgr = manager_with_source("protomaps-mlt", TileFormat::Mlt);
        let style = json!({
            "version": 8,
            "sources": {
                "protomaps": { "type": "vector", "url": "/data/protomaps-mlt" }
            }
        });

        let params = UrlQueryParams::with_key(Some("abc123".to_string()));
        let result = rewrite_style_for_api(&style, "http://localhost:8080", &params, &mgr);
        let tiles = result["sources"]["protomaps"]["tiles"].as_array().unwrap();

        assert_eq!(
            tiles[0],
            "http://localhost:8080/data/protomaps-mlt/{z}/{x}/{y}.pbf?key=abc123"
        );
    }

    #[test]
    fn test_rewrite_style_for_api_pbf_source_keeps_url() {
        let mgr = manager_with_source("protomaps", TileFormat::Pbf);
        let style = json!({
            "version": 8,
            "sources": {
                "protomaps": { "type": "vector", "url": "/data/protomaps" }
            }
        });

        let params = UrlQueryParams::default();
        let result = rewrite_style_for_api(&style, "http://localhost:8080", &params, &mgr);
        let src = &result["sources"]["protomaps"];

        assert_eq!(src["url"], "http://localhost:8080/data/protomaps");
        assert!(src.get("tiles").is_none());
        assert!(src.get("encoding").is_none());
    }

    // ---- Style::from_file + StyleManager coverage -----------------------

    /// Write a style.json into a temp dir and return its path plus the dir
    /// guard (kept alive by the caller so the file survives the test).
    fn temp_style(contents: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("style.json");
        std::fs::write(&path, contents).expect("write style");
        (dir, path)
    }

    #[test]
    fn test_style_from_file_missing_path_errors() {
        let config = StyleConfig {
            id: "ghost".to_string(),
            path: PathBuf::from("/nonexistent/path/to/style.json"),
            name: None,
        };
        let err = Style::from_file(&config).expect_err("missing file must error");
        assert!(matches!(err, TileServerError::StyleNotFound(id) if id == "ghost"));
    }

    #[test]
    fn test_style_from_file_invalid_json_errors() {
        let (_dir, path) = temp_style("{ this is not valid json ]");
        let config = StyleConfig {
            id: "broken".to_string(),
            path,
            name: None,
        };
        let err = Style::from_file(&config).expect_err("invalid JSON must error");
        assert!(matches!(err, TileServerError::MetadataError(_)));
    }

    #[test]
    fn test_style_from_file_name_falls_back_to_json_name() {
        let (_dir, path) = temp_style(r#"{"version": 8, "name": "From JSON"}"#);
        let config = StyleConfig {
            id: "styled".to_string(),
            path,
            name: None,
        };
        let style = Style::from_file(&config).expect("valid style");
        assert_eq!(style.name, "From JSON");
        assert_eq!(style.id, "styled");
    }

    #[test]
    fn test_style_from_file_name_falls_back_to_id() {
        let (_dir, path) = temp_style(r#"{"version": 8}"#);
        let config = StyleConfig {
            id: "only-id".to_string(),
            path,
            name: None,
        };
        let style = Style::from_file(&config).expect("valid style");
        assert_eq!(style.name, "only-id");
    }

    #[test]
    fn test_style_from_file_explicit_name_wins() {
        let (_dir, path) = temp_style(r#"{"version": 8, "name": "JSON Name"}"#);
        let config = StyleConfig {
            id: "id".to_string(),
            path,
            name: Some("Explicit Name".to_string()),
        };
        let style = Style::from_file(&config).expect("valid style");
        assert_eq!(style.name, "Explicit Name");
    }

    #[test]
    fn test_style_manager_from_configs_skips_bad_and_keeps_good() {
        let (_good_dir, good_path) = temp_style(r#"{"version": 8, "name": "Good"}"#);
        let configs = vec![
            StyleConfig {
                id: "good".to_string(),
                path: good_path,
                name: None,
            },
            StyleConfig {
                id: "bad".to_string(),
                path: PathBuf::from("/no/such/style.json"),
                name: None,
            },
        ];
        let manager = StyleManager::from_configs(&configs).expect("from_configs");
        assert_eq!(manager.len(), 1);
        assert!(!manager.is_empty());
        assert!(manager.get("good").is_some());
        assert!(manager.get("bad").is_none());
    }

    #[test]
    fn test_style_manager_empty_getters() {
        let manager = StyleManager::default();
        assert_eq!(manager.len(), 0);
        assert!(manager.is_empty());
        assert!(manager.get("missing").is_none());
        assert!(manager.all().is_empty());
        assert!(manager.all_infos("http://x").is_empty());
    }

    #[test]
    fn test_style_manager_all_and_all_infos() {
        let (_dir, path) = temp_style(r#"{"version": 8, "name": "One"}"#);
        let configs = vec![StyleConfig {
            id: "one".to_string(),
            path,
            name: None,
        }];
        let manager = StyleManager::from_configs(&configs).expect("from_configs");

        let all = manager.all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "one");

        let infos = manager.all_infos("http://tiles.example.com");
        assert_eq!(infos.len(), 1);
        assert_eq!(
            infos[0].url.as_deref(),
            Some("http://tiles.example.com/styles/one/style.json")
        );

        let keyed = manager.all_infos_with_key("http://tiles.example.com", Some("secret"));
        assert_eq!(
            keyed[0].url.as_deref(),
            Some("http://tiles.example.com/styles/one/style.json?key=secret")
        );
    }

    // ---- rewrite_style_for_native coverage ------------------------------

    #[test]
    fn test_rewrite_style_for_native_inlines_source_and_urls() {
        let mgr = manager_with_source("protomaps", TileFormat::Pbf);
        let style = json!({
            "version": 8,
            "sources": {
                "protomaps": { "type": "vector", "url": "/data/protomaps.json" }
            },
            "glyphs": "/fonts/{fontstack}/{range}.pbf",
            "sprite": "/styles/basic/sprite"
        });

        let result = rewrite_style_for_native(&style, "http://localhost:8080", &mgr);
        let src = &result["sources"]["protomaps"];

        assert!(src.get("url").is_none());
        assert_eq!(
            src["tiles"][0],
            "http://localhost:8080/data/protomaps/{z}/{x}/{y}.pbf"
        );
        assert_eq!(src["minzoom"], 0);
        assert_eq!(src["maxzoom"], 14);

        assert_eq!(
            result["glyphs"],
            "http://localhost:8080/fonts/{fontstack}/{range}.pbf"
        );
        assert_eq!(
            result["sprite"],
            "http://localhost:8080/styles/basic/sprite"
        );
    }

    #[test]
    fn test_rewrite_style_for_native_mlt_source_uses_pbf() {
        let mgr = manager_with_source("india", TileFormat::Mlt);
        let style = json!({
            "version": 8,
            "sources": {
                "india": { "type": "vector", "url": "/data/india" }
            }
        });

        let result = rewrite_style_for_native(&style, "http://localhost:8080", &mgr);
        let src = &result["sources"]["india"];
        assert_eq!(
            src["tiles"][0],
            "http://localhost:8080/data/india/{z}/{x}/{y}.pbf"
        );
        assert!(src.get("encoding").is_none());
    }

    #[test]
    fn test_rewrite_style_for_native_absolute_urls_untouched() {
        // Absolute glyphs/sprite (not starting with '/') are left untouched.
        let mgr = SourceManager::new();
        let style = json!({
            "version": 8,
            "sources": {},
            "glyphs": "https://cdn.example.com/fonts/{fontstack}/{range}.pbf",
            "sprite": "https://cdn.example.com/sprite"
        });
        let result = rewrite_style_for_native(&style, "http://localhost:8080", &mgr);
        assert_eq!(
            result["glyphs"],
            "https://cdn.example.com/fonts/{fontstack}/{range}.pbf"
        );
        assert_eq!(result["sprite"], "https://cdn.example.com/sprite");
    }

    #[test]
    fn test_rewrite_style_for_native_unknown_source_kept() {
        // A /data reference to a source we don't know about is left as-is
        // (the warn branch in rewrite_source), retaining its url.
        let mgr = SourceManager::new();
        let style = json!({
            "version": 8,
            "sources": {
                "missing": { "type": "vector", "url": "/data/missing.json" }
            }
        });
        let result = rewrite_style_for_native(&style, "http://localhost:8080", &mgr);
        let src = &result["sources"]["missing"];
        assert_eq!(src["url"], "/data/missing.json");
        assert!(src.get("tiles").is_none());
    }

    #[test]
    fn test_rewrite_style_for_native_non_data_url_kept() {
        // A source url that is not a /data reference triggers the early
        // `return` in rewrite_source and is left untouched.
        let mgr = SourceManager::new();
        let style = json!({
            "version": 8,
            "sources": {
                "remote": { "type": "raster", "url": "https://tiles.example.com/x.json" }
            }
        });
        let result = rewrite_style_for_native(&style, "http://localhost:8080", &mgr);
        assert_eq!(
            result["sources"]["remote"]["url"],
            "https://tiles.example.com/x.json"
        );
    }

    #[test]
    fn test_rewrite_style_for_native_source_without_url_kept() {
        // A source object with no `url` key hits the `_ => return` arm.
        let mgr = SourceManager::new();
        let style = json!({
            "version": 8,
            "sources": {
                "inline": { "type": "vector", "tiles": ["https://x/{z}/{x}/{y}.pbf"] }
            }
        });
        let result = rewrite_style_for_native(&style, "http://localhost:8080", &mgr);
        assert_eq!(
            result["sources"]["inline"]["tiles"][0],
            "https://x/{z}/{x}/{y}.pbf"
        );
    }
}
