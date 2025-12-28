//! Tileserver-rs library
//!
//! This module exposes the core functionality for testing and embedding.

pub mod cache_control;
pub mod config;
pub mod error;
pub mod render;
pub mod sources;
pub mod styles;

// Re-export key types for convenience
pub use config::Config;
pub use error::{Result, TileServerError};
pub use sources::{SourceManager, TileJson};
pub use styles::{StyleInfo, StyleManager};

// Re-export render types for testing
pub use render::overlay;
