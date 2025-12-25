use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::config::StyleConfig;
use crate::error::{Result, TileServerError};

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
