use chromiumoxide::browser::Browser;
use chromiumoxide::cdp::browser_protocol::page::{CaptureScreenshotFormat, CaptureScreenshotParams};
use std::sync::Arc;
use std::time::Duration;

use super::types::{ImageFormat, RenderOptions};
use crate::error::{Result, TileServerError};

pub struct Renderer {
    browser: Arc<Browser>,
}

impl Renderer {
    pub fn new(browser: Arc<Browser>) -> Self {
        Self { browser }
    }

    /// Render a map to an image buffer
    pub async fn render(&self, options: RenderOptions) -> Result<Vec<u8>> {
        tracing::debug!(
            "Rendering map: {}x{} @ {}x, zoom={}, center=[{}, {}]",
            options.width,
            options.height,
            options.scale,
            options.zoom,
            options.lon,
            options.lat
        );

        // Set viewport size (accounting for pixel ratio)
        let viewport_width = options.width * options.scale as u32;
        let viewport_height = options.height * options.scale as u32;

        // Build URL to existing Nuxt viewer page with screenshot mode
        // Format: http://localhost:8080/styles/{style}?screenshot#zoom/lat/lon
        let url = format!(
            "http://localhost:8080/styles/{}?screenshot#{}/{}/{}",
            options.style_id, options.zoom, options.lat, options.lon
        );

        tracing::debug!("Navigating to: {}", url);

        // Create a new page
        let page = self
            .browser
            .new_page(&url)
            .await
            .map_err(|e| TileServerError::RenderError(format!("Failed to create page: {}", e)))?;

        // Set viewport using emulation (chromiumoxide 0.8 API)
        use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;

        page.execute(
            SetDeviceMetricsOverrideParams::builder()
                .width(viewport_width as i64)
                .height(viewport_height as i64)
                .device_scale_factor(options.scale as f64)
                .mobile(false)
                .build()
                .map_err(|e| TileServerError::RenderError(format!("Failed to build viewport params: {}", e)))?
        )
        .await
        .map_err(|e| TileServerError::RenderError(format!("Failed to set viewport: {}", e)))?;

        // Wait for page to load and map to be ready
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Take screenshot
        let screenshot_format = match options.format {
            ImageFormat::Png => CaptureScreenshotFormat::Png,
            ImageFormat::Jpeg => CaptureScreenshotFormat::Jpeg,
            ImageFormat::Webp => CaptureScreenshotFormat::Webp,
        };

        let mut params = CaptureScreenshotParams::builder()
            .format(screenshot_format)
            .build();

        // Set quality for JPEG/WebP
        if matches!(options.format, ImageFormat::Jpeg | ImageFormat::Webp) {
            params.quality = Some(90);
        }

        let screenshot = page
            .screenshot(params)
            .await
            .map_err(|e| TileServerError::RenderError(format!("Failed to capture screenshot: {}", e)))?;

        // Close the page
        let _ = page.close().await;

        tracing::debug!("Render complete, image size: {} bytes", screenshot.len());

        Ok(screenshot)
    }
}
