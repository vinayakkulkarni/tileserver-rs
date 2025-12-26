//! Renderer pool for efficient tile rendering
//!
//! This module provides a pool of native MapLibre renderers.
//!
//! IMPORTANT: MapLibre Native is NOT thread-safe for concurrent style loading.
//! We use a global mutex to serialize all render operations, but run them in
//! spawn_blocking to avoid blocking the async runtime (MapLibre fetches tiles
//! from our server during rendering).

use std::sync::{Mutex, OnceLock};

use super::native::{MapMode, NativeMap, RenderOptions, RenderedImage, Size};
use crate::error::{Result, TileServerError};

/// Global mutex to serialize all MapLibre Native operations
/// This is necessary because MapLibre Native has shared state that isn't thread-safe
static RENDER_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

fn get_render_mutex() -> &'static Mutex<()> {
    RENDER_MUTEX.get_or_init(|| Mutex::new(()))
}

/// Configuration for a renderer pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Default tile size
    pub tile_size: u32,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self { tile_size: 512 }
    }
}

/// Pool of native MapLibre renderers
///
/// Currently uses a global mutex to serialize all render operations.
/// Each render creates a fresh renderer to avoid thread-safety issues
/// with MapLibre Native's shared state.
pub struct RendererPool {
    /// Configuration
    config: PoolConfig,
    /// Maximum scale factor
    max_scale: u8,
}

impl RendererPool {
    /// Create a new renderer pool
    pub fn new(config: PoolConfig, max_scale: u8) -> Result<Self> {
        // Initialize MapLibre Native
        super::native::init()?;

        tracing::info!(
            "Renderer pool initialized (tile_size={}, max_scale={})",
            config.tile_size,
            max_scale
        );

        Ok(Self { config, max_scale })
    }

    /// Render a tile
    pub async fn render_tile(
        &self,
        style_json: &str,
        z: u8,
        x: u32,
        y: u32,
        scale: u8,
    ) -> Result<Vec<u8>> {
        let scale = scale.min(self.max_scale).max(1);
        let tile_size = self.config.tile_size;
        let style_json = style_json.to_string();

        // Use spawn_blocking to avoid deadlock (MapLibre fetches tiles from our server)
        tokio::task::spawn_blocking(move || {
            // Acquire global render lock to serialize all MapLibre operations
            let _global_lock = get_render_mutex().lock().map_err(|e| {
                TileServerError::RenderError(format!("Failed to acquire render lock: {}", e))
            })?;

            // Create a fresh renderer for each request
            // This avoids issues with MapLibre Native's shared state across threads
            let mut map =
                NativeMap::new(Size::new(tile_size, tile_size), scale as f32, MapMode::Tile)?;

            map.load_style(&style_json)?;
            let image = map.render_tile(z, x, y, tile_size, scale as f32)?;
            image.to_png()
        })
        .await
        .map_err(|e| TileServerError::RenderError(format!("Render task panicked: {}", e)))?
    }

    /// Render a static image
    pub async fn render_static(
        &self,
        style_json: &str,
        options: RenderOptions,
    ) -> Result<RenderedImage> {
        let style_json = style_json.to_string();

        tokio::task::spawn_blocking(move || {
            // Acquire global render lock to serialize all MapLibre operations
            let _global_lock = get_render_mutex().lock().map_err(|e| {
                TileServerError::RenderError(format!("Failed to acquire render lock: {}", e))
            })?;

            let mut map = NativeMap::new(options.size, options.pixel_ratio, MapMode::Static)?;
            map.load_style(&style_json)?;
            map.render(Some(options))
        })
        .await
        .map_err(|e| TileServerError::RenderError(format!("Render task panicked: {}", e)))?
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            max_scale: self.max_scale,
        }
    }
}

impl Drop for RendererPool {
    fn drop(&mut self) {
        tracing::info!("Renderer pool shutting down");
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub max_scale: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_creation() {
        let config = PoolConfig::default();
        let pool = RendererPool::new(config, 3);
        assert!(pool.is_ok());
    }
}
