//! Tileserver-rs library
//!
//! This module exposes the core functionality for testing and embedding.

pub mod admin;
pub mod autodetect;
pub mod cache_control;
pub mod config;
pub mod error;
pub mod openapi;
pub mod reload;
pub mod render;
pub mod sources;
pub mod startup;
pub mod styles;
#[cfg(feature = "mlt")]
pub mod transcode;
pub mod upload;
pub mod wmts;

pub use config::Config;
pub use error::{Result, TileServerError};
pub use reload::AppState;
pub use sources::{
    detect_mlt_format, SourceManager, TileCompression, TileData, TileFormat, TileJson, TileSource,
};
pub use styles::{rewrite_style_for_api, Style, StyleInfo, StyleManager, UrlQueryParams};
#[cfg(feature = "mlt")]
pub use transcode::{transcode_tile, MvtProto};

#[cfg(feature = "postgres")]
pub use config::{PostgresConfig, PostgresFunctionConfig};
#[cfg(feature = "postgres")]
pub use sources::postgres::{PoolSettings, PostgresFunctionSource, PostgresPool};

// Re-export render types for testing
pub use render::overlay;
pub use render::{ImageFormat, StaticType};
