//! Tileserver-rs library
//!
//! This module exposes the core functionality for testing and embedding.

pub mod cache_control;
pub mod config;
pub mod error;
pub mod openapi;
pub mod render;
pub mod sources;
pub mod styles;
pub mod wmts;

pub use config::Config;
pub use error::{Result, TileServerError};
pub use sources::{SourceManager, TileCompression, TileData, TileFormat, TileJson, TileSource};
pub use styles::{rewrite_style_for_api, Style, StyleInfo, StyleManager, UrlQueryParams};

#[cfg(feature = "postgres")]
pub use config::{PostgresConfig, PostgresFunctionConfig};
#[cfg(feature = "postgres")]
pub use sources::postgres::{PostgresFunctionSource, PostgresPool};

// Re-export render types for testing
pub use render::overlay;
pub use render::{ImageFormat, StaticType};
