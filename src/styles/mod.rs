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
        StyleInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            url: Some(format!("{}/styles/{}/style.json", base_url, self.id)),
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
        self.styles.values().map(|s| s.to_info(base_url)).collect()
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
