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
    /// Create a renderer with the default pool configuration.
    ///
    /// # Errors
    ///
    /// The errors of [`RendererPool::new`].
    pub fn new() -> Result<Self> {
        Self::with_config(PoolConfig::default(), 3)
    }

    /// Create a renderer with `config` and the maximum scale factor.
    ///
    /// # Errors
    ///
    /// The errors of [`RendererPool::new`].
    pub fn with_config(config: PoolConfig, max_scale: u8) -> Result<Self> {
        let pool = RendererPool::new(config, max_scale)?;
        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Render a map tile
    ///
    /// # Errors
    ///
    /// The errors of [`RendererPool::render_tile`], and
    /// [`TileServerError::RenderError`] when the image cannot be re-encoded.
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

        Self::encode_tile(png_data, format).await
    }

    /// Re-encode a rendered PNG tile to `format` off the async runtime.
    async fn encode_tile(png_data: Vec<u8>, format: ImageFormat) -> Result<Vec<u8>> {
        if format == ImageFormat::Png {
            return Ok(png_data);
        }
        // Decode and re-encode are CPU work: keep them off the runtime.
        run_blocking(move || match format {
            ImageFormat::Jpeg => Self::convert_png_to_jpeg(&png_data, 90),
            ImageFormat::Webp => Self::convert_png_to_webp(&png_data, 90),
            ImageFormat::Png => Ok(png_data),
        })
        .await
    }

    /// Render a static map image
    ///
    /// # Errors
    ///
    /// The errors of [`RendererPool::render_static`], and
    /// [`TileServerError::RenderError`] when the overlays cannot be drawn or
    /// the image cannot be encoded.
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

        let rendered_image = self
            .pool
            .render_static(&options.style_json, native_options)
            .await?;

        Self::finish_static(rendered_image, options).await
    }

    /// Draw overlays on a rendered static image and encode it, off the
    /// async runtime.
    async fn finish_static(
        rendered_image: super::native::RenderedImage,
        options: RenderOptions,
    ) -> Result<Vec<u8>> {
        // Overlay drawing and encoding are CPU work: keep them off the runtime.
        run_blocking(move || {
            let final_image = Self::apply_overlays(rendered_image, &options)?;
            match options.format {
                ImageFormat::Png => final_image.to_png(),
                ImageFormat::Jpeg => final_image.to_jpeg(90),
                ImageFormat::Webp => final_image.to_webp(90),
            }
        })
        .await
    }

    /// Apply path and marker overlays to a rendered image
    fn apply_overlays(
        mut image: super::native::RenderedImage,
        options: &RenderOptions,
    ) -> Result<super::native::RenderedImage> {
        let mut paths = Vec::new();
        let mut markers = Vec::new();

        if let Some(ref path_str) = options.path {
            for path_part in path_str.split('~') {
                if let Some(path) = super::overlay::parse_path(path_part, false) {
                    paths.push(path);
                }
            }
        }

        if let Some(ref marker_str) = options.marker {
            for marker_part in marker_str.split('~') {
                if let Some(marker) = super::overlay::parse_marker(marker_part) {
                    markers.push(marker);
                }
            }
        }

        if let Some(ref geojson_str) = options.geojson {
            let (geojson_paths, geojson_markers) = super::overlay::parse_geojson(geojson_str);
            paths.extend(geojson_paths);
            markers.extend(geojson_markers);
        }

        if paths.is_empty() && markers.is_empty() {
            return Ok(image);
        }

        // Convert to image::RgbaImage for drawing
        let actual_width = options.width * options.scale as u32;
        let actual_height = options.height * options.scale as u32;

        let mut rgba_image =
            image::RgbaImage::from_raw(actual_width, actual_height, image.take_data()).ok_or_else(
                || TileServerError::RenderError("failed to create image buffer".to_string()),
            )?;

        // Draw overlays
        super::overlay::draw_overlays(
            &mut rgba_image,
            &paths,
            &markers,
            options.lon,
            options.lat,
            options.zoom,
            options.scale as f32,
        );

        // Convert back to native RenderedImage
        Ok(super::native::RenderedImage::from_rgba(
            actual_width,
            actual_height,
            rgba_image.into_raw(),
        ))
    }

    /// Convert PNG data to JPEG
    fn convert_png_to_jpeg(png_data: &[u8], quality: u8) -> Result<Vec<u8>> {
        use image::ImageReader;
        use std::io::Cursor;

        let img = ImageReader::new(Cursor::new(png_data))
            .with_guessed_format()
            .map_err(|e| TileServerError::RenderError(format!("failed to read PNG: {}", e)))?
            .decode()
            .map_err(|e| TileServerError::RenderError(format!("failed to decode PNG: {}", e)))?;

        let rgb = img.to_rgb8();

        let estimated = (rgb.width() * rgb.height()) as usize;
        let mut buffer = Vec::with_capacity(estimated);
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
    fn convert_png_to_webp(png_data: &[u8], _quality: u8) -> Result<Vec<u8>> {
        use image::ImageReader;
        use std::io::Cursor;

        let img = ImageReader::new(Cursor::new(png_data))
            .with_guessed_format()
            .map_err(|e| TileServerError::RenderError(format!("failed to read PNG: {}", e)))?
            .decode()
            .map_err(|e| TileServerError::RenderError(format!("failed to decode PNG: {}", e)))?;

        // Use DynamicImage to write WebP
        let estimated = (img.width() * img.height()) as usize;
        let mut buffer = Cursor::new(Vec::with_capacity(estimated));
        img.write_to(&mut buffer, image::ImageFormat::WebP)
            .map_err(|e| TileServerError::RenderError(format!("WebP encoding failed: {}", e)))?;

        Ok(buffer.into_inner())
    }

    /// Get the underlying pool (for advanced usage)
    #[must_use]
    pub fn pool(&self) -> Arc<RendererPool> {
        self.pool.clone()
    }
}

/// Run CPU-bound render post-processing on the blocking thread pool.
async fn run_blocking<F>(f: F) -> Result<Vec<u8>>
where
    F: FnOnce() -> Result<Vec<u8>> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| TileServerError::RenderError(format!("render task failed: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_renderer_creation() {
        let renderer = Renderer::new();
        assert!(renderer.is_ok());
    }

    fn png_4x4() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([200, 30, 30, 255]));
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    fn static_options(format: ImageFormat, marker: Option<&str>) -> RenderOptions {
        RenderOptions {
            style_id: "s".to_string(),
            style_json: "{}".to_string(),
            width: 64,
            height: 64,
            scale: 1,
            lon: 0.0,
            lat: 0.0,
            zoom: 2.0,
            bearing: 0.0,
            pitch: 0.0,
            format,
            path: None,
            marker: marker.map(str::to_string),
            geojson: None,
        }
    }

    #[tokio::test]
    async fn encode_tile_keeps_png_and_converts_jpeg_and_webp() {
        let png = png_4x4();
        assert_eq!(
            Renderer::encode_tile(png.clone(), ImageFormat::Png)
                .await
                .unwrap(),
            png
        );
        let jpeg = Renderer::encode_tile(png.clone(), ImageFormat::Jpeg)
            .await
            .unwrap();
        assert_eq!(&jpeg[..3], &[0xFF, 0xD8, 0xFF], "JPEG magic");
        let webp = Renderer::encode_tile(png, ImageFormat::Webp).await.unwrap();
        assert_eq!(&webp[..4], b"RIFF");
        assert_eq!(&webp[8..12], b"WEBP");
    }

    #[tokio::test]
    async fn encode_tile_rejects_bytes_that_are_not_an_image() {
        for format in [ImageFormat::Jpeg, ImageFormat::Webp] {
            let err = Renderer::encode_tile(b"not a png".to_vec(), format)
                .await
                .unwrap_err();
            assert!(matches!(err, TileServerError::RenderError(_)), "{err}");
        }
    }

    #[tokio::test]
    async fn finish_static_draws_a_marker_and_encodes_each_format() {
        let blank =
            || super::super::native::RenderedImage::from_rgba(64, 64, vec![255u8; 64 * 64 * 4]);
        let plain = Renderer::finish_static(blank(), static_options(ImageFormat::Png, None))
            .await
            .unwrap();
        let marked = Renderer::finish_static(
            blank(),
            static_options(ImageFormat::Png, Some("pin-s+f00(0,0)")),
        )
        .await
        .unwrap();
        assert_ne!(plain, marked, "the marker changes the pixels");

        let jpeg = Renderer::finish_static(blank(), static_options(ImageFormat::Jpeg, None))
            .await
            .unwrap();
        assert_eq!(&jpeg[..3], &[0xFF, 0xD8, 0xFF]);
        let webp = Renderer::finish_static(blank(), static_options(ImageFormat::Webp, None))
            .await
            .unwrap();
        assert_eq!(&webp[8..12], b"WEBP");
    }

    #[tokio::test]
    async fn finish_static_rejects_a_buffer_that_does_not_match_the_size() {
        let short = super::super::native::RenderedImage::from_rgba(64, 64, vec![0u8; 16]);
        let err = Renderer::finish_static(short, static_options(ImageFormat::Png, Some("0,0")))
            .await
            .unwrap_err();
        assert!(matches!(err, TileServerError::RenderError(_)), "{err}");
    }
}
