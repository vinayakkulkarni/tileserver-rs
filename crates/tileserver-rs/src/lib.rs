//! Tileserver-rs library
//!
//! This module exposes the core functionality for testing and embedding.

pub mod admin;
pub mod autodetect;
pub mod cache;
pub mod cache_control;
pub mod config;
pub mod error;
pub mod metrics;
pub mod openapi;
#[cfg(feature = "raster")]
pub mod raster;
pub mod reload;
pub mod render;
pub mod routes;
pub mod sources;
pub mod startup;
pub mod styles;
#[cfg(feature = "mlt")]
pub mod transcode;
pub mod upload;
pub mod wmts;

pub use cache::TileCache;
pub use config::{CacheConfig, Config};
pub use error::{Result, TileServerError};
pub use reload::AppState;
pub use sources::{
    SourceManager, TileCompression, TileData, TileFormat, TileJson, TileSource, detect_mlt_format,
};
pub use styles::{Style, StyleInfo, StyleManager, UrlQueryParams, rewrite_style_for_api};
#[cfg(feature = "mlt")]
pub use transcode::{MvtProto, transcode_tile};

#[cfg(feature = "postgres")]
pub use config::{PostgresConfig, PostgresFunctionConfig};
#[cfg(feature = "postgres")]
pub use sources::postgres::{
    PoolSettings, PostgresFunctionSource, PostgresPool, PostgresTableSource, TableInfo,
};

// Re-export render types for testing
pub use render::overlay;
pub use render::{ImageFormat, StaticType};
