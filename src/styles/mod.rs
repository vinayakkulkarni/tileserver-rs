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
    pub fn to_info(&self, base_url: &str) -> StyleInfo {
        self.to_info_with_key(base_url, None)
    }

    /// Convert to StyleInfo for API response with optional API key
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
    pub fn get(&self, id: &str) -> Option<&Style> {
        self.styles.get(id)
    }

    /// Get all style infos for API response
    pub fn all_infos(&self, base_url: &str) -> Vec<StyleInfo> {
        self.all_infos_with_key(base_url, None)
    }

    /// Get all style infos for API response with optional API key
    pub fn all_infos_with_key(&self, base_url: &str, key: Option<&str>) -> Vec<StyleInfo> {
        self.styles
            .values()
            .map(|s| s.to_info_with_key(base_url, key))
            .collect()
    }

    /// Get all styles
    pub fn all(&self) -> Vec<&Style> {
        self.styles.values().collect()
    }

    /// Get the number of styles
    pub fn len(&self) -> usize {
        self.styles.len()
    }

    /// Check if there are no styles
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
    pub fn with_key(key: Option<String>) -> Self {
        Self {
            key,
            extra: Vec::new(),
        }
    }

    /// Build query string to append to URLs
    /// Returns empty string if no params, otherwise "?key=value&..."
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
pub fn rewrite_style_for_api(
    style_json: &serde_json::Value,
    base_url: &str,
    query_params: &UrlQueryParams,
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
    if let Some(style_sources) = style.get_mut("sources") {
        if let Some(sources_obj) = style_sources.as_object_mut() {
            for (_source_id, source_config) in sources_obj.iter_mut() {
                if let Some(source_obj) = source_config.as_object_mut() {
                    // Rewrite "url" field if relative
                    if let Some(url) = source_obj.get_mut("url") {
                        if let Some(url_str) = url.as_str() {
                            *url = serde_json::Value::String(rewrite_url(url_str));
                        }
                    }

                    // Rewrite "tiles" array if relative
                    if let Some(tiles) = source_obj.get_mut("tiles") {
                        if let Some(tiles_arr) = tiles.as_array_mut() {
                            for tile in tiles_arr.iter_mut() {
                                if let Some(tile_str) = tile.as_str() {
                                    *tile = serde_json::Value::String(rewrite_url(tile_str));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Rewrite glyphs URL if relative
    if let Some(glyphs) = style.get_mut("glyphs") {
        if let Some(glyphs_str) = glyphs.as_str() {
            *glyphs = serde_json::Value::String(rewrite_url(glyphs_str));
        }
    }

    // Rewrite sprite URL if relative
    if let Some(sprite) = style.get_mut("sprite") {
        if let Some(sprite_str) = sprite.as_str() {
            *sprite = serde_json::Value::String(rewrite_url(sprite_str));
        }
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
    if let Some(style_sources) = style.get_mut("sources") {
        if let Some(sources_obj) = style_sources.as_object_mut() {
            for (source_id, source_config) in sources_obj.iter_mut() {
                rewrite_source(source_id, source_config, base_url, sources);
            }
        }
    }

    // Rewrite glyphs URL if it's relative
    if let Some(glyphs) = style.get_mut("glyphs") {
        if let Some(glyphs_str) = glyphs.as_str() {
            if glyphs_str.starts_with('/') {
                let absolute_url = format!("{}{}", base_url, glyphs_str);
                tracing::debug!("Rewriting glyphs URL: {} -> {}", glyphs_str, absolute_url);
                *glyphs = serde_json::Value::String(absolute_url);
            }
        }
    }

    // Rewrite sprite URL if it's relative
    if let Some(sprite) = style.get_mut("sprite") {
        if let Some(sprite_str) = sprite.as_str() {
            if sprite_str.starts_with('/') {
                let absolute_url = format!("{}{}", base_url, sprite_str);
                tracing::debug!("Rewriting sprite URL: {} -> {}", sprite_str, absolute_url);
                *sprite = serde_json::Value::String(absolute_url);
            }
        }
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
    // e.g., "/data/protomaps.json" or "http://localhost:8080/data/protomaps.json"
    let data_source_id = if let Some(rest) = url.strip_prefix("/data/") {
        rest.strip_suffix(".json")
    } else if url.contains("/data/") && url.ends_with(".json") {
        url.rsplit("/data/")
            .next()
            .and_then(|s| s.strip_suffix(".json"))
    } else {
        None
    };

    let data_source_id = match data_source_id {
        Some(id) => id,
        None => return, // Not a reference to our data endpoint
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

    // Build the tile URL template
    let tile_url = format!(
        "{}/data/{}/{{z}}/{{x}}/{{y}}.{}",
        base_url,
        data_source_id,
        metadata.format.extension()
    );

    tracing::debug!(
        "Rewriting source '{}' from URL '{}' to tiles ['{}']",
        source_id,
        url,
        tile_url
    );

    // Remove the URL and add tiles array
    source_obj.remove("url");
    source_obj.insert("tiles".to_string(), serde_json::json!([tile_url]));

    // Add additional metadata if not already present
    if !source_obj.contains_key("minzoom") {
        source_obj.insert("minzoom".to_string(), serde_json::json!(metadata.minzoom));
    }
    if !source_obj.contains_key("maxzoom") {
        source_obj.insert("maxzoom".to_string(), serde_json::json!(metadata.maxzoom));
    }
    if !source_obj.contains_key("bounds") {
        if let Some(bounds) = &metadata.bounds {
            source_obj.insert("bounds".to_string(), serde_json::json!(bounds));
        }
    }
    if !source_obj.contains_key("attribution") {
        if let Some(attribution) = &metadata.attribution {
            source_obj.insert("attribution".to_string(), serde_json::json!(attribution));
        }
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
        let result = rewrite_style_for_api(&style, "http://tiles.example.com", &params);

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
        let result = rewrite_style_for_api(&style, "http://tiles.example.com", &params);

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
        let result = rewrite_style_for_api(&style, "http://tiles.example.com", &params);

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
        let result = rewrite_style_for_api(&style, "http://localhost:8080", &params);

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
        let result = rewrite_style_for_api(&style, "http://localhost", &params);

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
}
