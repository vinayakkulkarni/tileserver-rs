//! High-level renderer interface
//!
//! This module provides a high-level interface for rendering map tiles
//! and static images using the native MapLibre renderer pool.

use std::sync::Arc;

use super::pool::{PoolConfig, RendererPool};
use super::types::{ImageFormat, RenderOptions};
use crate::error::{Result, TileServerError};

/// High-level renderer that manages the native renderer pool
pub struct Renderer {
    pool: Arc<RendererPool>,
}

impl Renderer {
    /// Create a new renderer with default configuration
    pub fn new() -> Result<Self> {
        Self::with_config(PoolConfig::default(), 3)
    }

    /// Create a new renderer with custom configuration
    pub fn with_config(config: PoolConfig, max_scale: u8) -> Result<Self> {
        let pool = RendererPool::new(config, max_scale)?;
        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Render a map tile
    pub async fn render_tile(
        &self,
        style_json: &str,
        z: u8,
        x: u32,
        y: u32,
        scale: u8,
        format: ImageFormat,
    ) -> Result<Vec<u8>> {
        tracing::debug!(
            "Rendering tile z={}, x={}, y={}, scale={}, format={:?}",
            z,
            x,
            y,
            scale,
            format
        );

        // Get PNG from pool
        let png_data = self.pool.render_tile(style_json, z, x, y, scale).await?;

        // Convert to requested format if needed
        match format {
            ImageFormat::Png => Ok(png_data),
            ImageFormat::Jpeg => self.convert_png_to_jpeg(&png_data, 90),
            ImageFormat::Webp => self.convert_png_to_webp(&png_data, 90),
        }
    }

    /// Render a static map image
    pub async fn render_static(&self, options: RenderOptions) -> Result<Vec<u8>> {
        tracing::debug!(
            "Rendering static image: {}x{} @ {}x, zoom={}, center=[{}, {}]",
            options.width,
            options.height,
            options.scale,
            options.zoom,
            options.lon,
            options.lat
        );

        let native_options = super::native::RenderOptions {
            size: super::native::Size::new(options.width, options.height),
            pixel_ratio: options.scale as f32,
            camera: super::native::CameraOptions::new(options.lat, options.lon, options.zoom)
                .with_bearing(options.bearing)
                .with_pitch(options.pitch),
            mode: super::native::MapMode::Static,
        };

        let image = self
            .pool
            .render_static(&options.style_json, native_options)
            .await?;

        // Convert to requested format
        match options.format {
            ImageFormat::Png => image.to_png(),
            ImageFormat::Jpeg => image.to_jpeg(90),
            ImageFormat::Webp => image.to_webp(90),
        }
    }

    /// Convert PNG data to JPEG
    fn convert_png_to_jpeg(&self, png_data: &[u8], quality: u8) -> Result<Vec<u8>> {
        use image::ImageReader;
        use std::io::Cursor;

        let img = ImageReader::new(Cursor::new(png_data))
            .with_guessed_format()
            .map_err(|e| TileServerError::RenderError(format!("Failed to read PNG: {}", e)))?
            .decode()
            .map_err(|e| TileServerError::RenderError(format!("Failed to decode PNG: {}", e)))?;

        let rgb = img.to_rgb8();

        let mut buffer = Vec::new();
        {
            let mut encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
            encoder
                .encode(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| {
                    TileServerError::RenderError(format!("JPEG encoding failed: {}", e))
                })?;
        }

        Ok(buffer)
    }

    /// Convert PNG data to WebP
    fn convert_png_to_webp(&self, png_data: &[u8], _quality: u8) -> Result<Vec<u8>> {
        use image::ImageReader;
        use std::io::Cursor;

        let img = ImageReader::new(Cursor::new(png_data))
            .with_guessed_format()
            .map_err(|e| TileServerError::RenderError(format!("Failed to read PNG: {}", e)))?
            .decode()
            .map_err(|e| TileServerError::RenderError(format!("Failed to decode PNG: {}", e)))?;

        // Use DynamicImage to write WebP
        let mut buffer = Cursor::new(Vec::new());
        img.write_to(&mut buffer, image::ImageFormat::WebP)
            .map_err(|e| TileServerError::RenderError(format!("WebP encoding failed: {}", e)))?;

        Ok(buffer.into_inner())
    }

    /// Get the underlying pool (for advanced usage)
    pub fn pool(&self) -> Arc<RendererPool> {
        self.pool.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_renderer_creation() {
        let renderer = Renderer::new();
        assert!(renderer.is_ok());
    }
}
