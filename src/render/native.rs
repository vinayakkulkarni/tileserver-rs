//! Native MapLibre GL rendering using FFI bindings
//!
//! This module provides safe Rust wrappers around the MapLibre Native C API.
//! It is designed for server-side rendering of map tiles and static images.

use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::Once;

use maplibre_native_sys::{
    mln_cleanup, mln_get_last_error, mln_headless_frontend_create, mln_headless_frontend_destroy,
    mln_headless_frontend_set_size, mln_image_free, mln_init, mln_map_create,
    mln_map_create_with_loader, mln_map_destroy, mln_map_is_fully_loaded, mln_map_load_style,
    mln_map_render_still, mln_map_set_camera, mln_map_set_size, MLNCameraOptions, MLNErrorCode,
    MLNHeadlessFrontend, MLNImageData, MLNMap, MLNMapMode, MLNRenderOptions, MLNResourceCallback,
    MLNSize,
};

use crate::error::{Result, TileServerError};

static INIT: Once = Once::new();
static mut INITIALIZED: bool = false;

/// Initialize the MapLibre Native library.
/// This is called automatically when needed but can be called explicitly.
pub fn init() -> Result<()> {
    let mut result = Ok(());

    INIT.call_once(|| {
        let code = unsafe { mln_init() };
        if code != MLNErrorCode::MLN_OK {
            result = Err(TileServerError::RenderError(format!(
                "Failed to initialize MapLibre Native: {:?}",
                get_last_error()
            )));
        } else {
            unsafe { INITIALIZED = true };
        }
    });

    result
}

/// Cleanup the MapLibre Native library.
/// Should be called when shutting down the application.
#[allow(dead_code)]
pub fn cleanup() {
    unsafe {
        if INITIALIZED {
            mln_cleanup();
            INITIALIZED = false;
        }
    }
}

/// Get the last error message from MapLibre Native.
fn get_last_error() -> Option<String> {
    unsafe {
        let ptr = mln_get_last_error();
        if ptr.is_null() {
            None
        } else {
            Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
        }
    }
}

/// Size of a render target
#[derive(Debug, Clone, Copy, Default)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl From<Size> for MLNSize {
    fn from(size: Size) -> Self {
        MLNSize {
            width: size.width,
            height: size.height,
        }
    }
}

impl From<MLNSize> for Size {
    fn from(size: MLNSize) -> Self {
        Size {
            width: size.width,
            height: size.height,
        }
    }
}

/// Camera options for rendering
#[derive(Debug, Clone, Copy, Default)]
pub struct CameraOptions {
    pub latitude: f64,
    pub longitude: f64,
    pub zoom: f64,
    pub bearing: f64,
    pub pitch: f64,
}

impl CameraOptions {
    pub fn new(latitude: f64, longitude: f64, zoom: f64) -> Self {
        Self {
            latitude,
            longitude,
            zoom,
            bearing: 0.0,
            pitch: 0.0,
        }
    }

    pub fn with_bearing(mut self, bearing: f64) -> Self {
        self.bearing = bearing;
        self
    }

    pub fn with_pitch(mut self, pitch: f64) -> Self {
        self.pitch = pitch;
        self
    }
}

impl From<CameraOptions> for MLNCameraOptions {
    fn from(camera: CameraOptions) -> Self {
        MLNCameraOptions {
            latitude: camera.latitude,
            longitude: camera.longitude,
            zoom: camera.zoom,
            bearing: camera.bearing,
            pitch: camera.pitch,
        }
    }
}

impl From<MLNCameraOptions> for CameraOptions {
    fn from(camera: MLNCameraOptions) -> Self {
        CameraOptions {
            latitude: camera.latitude,
            longitude: camera.longitude,
            zoom: camera.zoom,
            bearing: camera.bearing,
            pitch: camera.pitch,
        }
    }
}

/// Map rendering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MapMode {
    /// Static mode for rendering complete images
    #[default]
    Static,
    /// Tile mode optimized for tile rendering
    Tile,
}

impl From<MapMode> for MLNMapMode {
    fn from(mode: MapMode) -> Self {
        match mode {
            MapMode::Static => MLNMapMode::MLN_MAP_MODE_STATIC,
            MapMode::Tile => MLNMapMode::MLN_MAP_MODE_TILE,
        }
    }
}

/// Rendered image data
pub struct RenderedImage {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl RenderedImage {
    /// Create a new RenderedImage from raw RGBA data
    pub fn from_rgba(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self {
            data,
            width,
            height,
        }
    }

    /// Get the raw RGBA pixel data (premultiplied alpha)
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Take ownership of the raw data
    pub fn take_data(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.data)
    }

    /// Get the image width in pixels
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the image height in pixels
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Convert to PNG format
    pub fn to_png(&self) -> Result<Vec<u8>> {
        use image::{ImageBuffer, Rgba};
        use std::io::Cursor;

        // Create image from raw RGBA data
        let img: ImageBuffer<Rgba<u8>, _> =
            ImageBuffer::from_raw(self.width, self.height, self.data.clone()).ok_or_else(|| {
                TileServerError::RenderError("Failed to create image buffer".to_string())
            })?;

        // Encode to PNG using the DynamicImage interface
        let mut buffer = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buffer, image::ImageFormat::Png)
            .map_err(|e| TileServerError::RenderError(format!("PNG encoding failed: {}", e)))?;

        Ok(buffer.into_inner())
    }

    /// Convert to JPEG format
    pub fn to_jpeg(&self, quality: u8) -> Result<Vec<u8>> {
        use image::{ImageBuffer, Rgb};

        // Convert RGBA to RGB (JPEG doesn't support alpha)
        let mut rgb_data = Vec::with_capacity((self.width * self.height * 3) as usize);
        for chunk in self.data.chunks(4) {
            rgb_data.push(chunk[0]);
            rgb_data.push(chunk[1]);
            rgb_data.push(chunk[2]);
        }

        let img: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_raw(self.width, self.height, rgb_data)
            .ok_or_else(|| {
                TileServerError::RenderError("Failed to create image buffer".to_string())
            })?;

        // Encode to JPEG
        let mut buffer = Vec::new();
        {
            let mut encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
            encoder
                .encode(
                    img.as_raw(),
                    self.width,
                    self.height,
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(|e| {
                    TileServerError::RenderError(format!("JPEG encoding failed: {}", e))
                })?;
        }

        Ok(buffer)
    }

    /// Convert to WebP format
    pub fn to_webp(&self, _quality: u8) -> Result<Vec<u8>> {
        use image::{ImageBuffer, Rgba};
        use std::io::Cursor;

        let img: ImageBuffer<Rgba<u8>, _> =
            ImageBuffer::from_raw(self.width, self.height, self.data.clone()).ok_or_else(|| {
                TileServerError::RenderError("Failed to create image buffer".to_string())
            })?;

        // Encode to WebP using DynamicImage interface
        let mut buffer = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut buffer, image::ImageFormat::WebP)
            .map_err(|e| TileServerError::RenderError(format!("WebP encoding failed: {}", e)))?;

        Ok(buffer.into_inner())
    }
}

/// Headless frontend for map rendering
pub struct HeadlessFrontend {
    ptr: *mut MLNHeadlessFrontend,
}

// Safety: HeadlessFrontend is thread-safe as long as each instance is only
// used from one thread at a time. The MapLibre Native library handles
// internal synchronization.
unsafe impl Send for HeadlessFrontend {}

impl HeadlessFrontend {
    /// Create a new headless frontend
    pub fn new(size: Size, pixel_ratio: f32) -> Result<Self> {
        init()?;

        let ptr = unsafe { mln_headless_frontend_create(size.into(), pixel_ratio) };

        if ptr.is_null() {
            return Err(TileServerError::RenderError(
                get_last_error().unwrap_or_else(|| "Failed to create frontend".to_string()),
            ));
        }

        Ok(Self { ptr })
    }

    /// Set the size of the render target
    #[allow(dead_code)]
    pub fn set_size(&mut self, size: Size) {
        unsafe {
            mln_headless_frontend_set_size(self.ptr, size.into());
        }
    }

    /// Get the raw pointer (for internal use)
    pub(crate) fn as_ptr(&self) -> *mut MLNHeadlessFrontend {
        self.ptr
    }
}

impl Drop for HeadlessFrontend {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                mln_headless_frontend_destroy(self.ptr);
            }
        }
    }
}

/// A MapLibre map instance for rendering
pub struct NativeMap {
    ptr: *mut MLNMap,
    _frontend: HeadlessFrontend, // Keep frontend alive
}

// Safety: Same as HeadlessFrontend
unsafe impl Send for NativeMap {}

impl NativeMap {
    /// Create a new map instance
    pub fn new(size: Size, pixel_ratio: f32, mode: MapMode) -> Result<Self> {
        let frontend = HeadlessFrontend::new(size, pixel_ratio)?;

        let ptr = unsafe { mln_map_create(frontend.as_ptr(), pixel_ratio, mode.into()) };

        if ptr.is_null() {
            return Err(TileServerError::RenderError(
                get_last_error().unwrap_or_else(|| "Failed to create map".to_string()),
            ));
        }

        Ok(Self {
            ptr,
            _frontend: frontend,
        })
    }

    /// Create a new map instance with a custom resource loader callback
    #[allow(dead_code)]
    pub fn with_resource_loader(
        size: Size,
        pixel_ratio: f32,
        mode: MapMode,
        callback: MLNResourceCallback,
        user_data: *mut std::ffi::c_void,
    ) -> Result<Self> {
        let frontend = HeadlessFrontend::new(size, pixel_ratio)?;

        let ptr = unsafe {
            mln_map_create_with_loader(
                frontend.as_ptr(),
                pixel_ratio,
                mode.into(),
                callback,
                user_data,
            )
        };

        if ptr.is_null() {
            return Err(TileServerError::RenderError(
                get_last_error().unwrap_or_else(|| "Failed to create map with loader".to_string()),
            ));
        }

        Ok(Self {
            ptr,
            _frontend: frontend,
        })
    }

    /// Load a style JSON
    pub fn load_style(&mut self, style_json: &str) -> Result<()> {
        let c_style = CString::new(style_json).map_err(|_| {
            TileServerError::RenderError("Style JSON contains null bytes".to_string())
        })?;

        let code = unsafe { mln_map_load_style(self.ptr, c_style.as_ptr()) };

        if code != MLNErrorCode::MLN_OK {
            return Err(TileServerError::RenderError(
                get_last_error().unwrap_or_else(|| format!("Failed to load style: {:?}", code)),
            ));
        }

        Ok(())
    }

    /// Check if the map is fully loaded
    #[allow(dead_code)]
    pub fn is_fully_loaded(&self) -> bool {
        unsafe { mln_map_is_fully_loaded(self.ptr) }
    }

    /// Set the camera options
    #[allow(dead_code)]
    pub fn set_camera(&mut self, camera: CameraOptions) {
        let c_camera: MLNCameraOptions = camera.into();
        unsafe {
            mln_map_set_camera(self.ptr, &c_camera);
        }
    }

    /// Set the map size
    #[allow(dead_code)]
    pub fn set_size(&mut self, size: Size) {
        unsafe {
            mln_map_set_size(self.ptr, size.into());
        }
    }

    /// Render a still image synchronously
    pub fn render(&mut self, options: Option<RenderOptions>) -> Result<RenderedImage> {
        let mut image_data = MLNImageData::default();

        let c_options = options.map(|o| o.into_native());

        let code = unsafe {
            mln_map_render_still(
                self.ptr,
                c_options
                    .as_ref()
                    .map(|o| o as *const MLNRenderOptions)
                    .unwrap_or(ptr::null()),
                &mut image_data,
            )
        };

        if code != MLNErrorCode::MLN_OK {
            return Err(TileServerError::RenderError(
                get_last_error().unwrap_or_else(|| format!("Render failed: {:?}", code)),
            ));
        }

        // Save dimensions before freeing (mln_image_free zeros them)
        let width = image_data.width;
        let height = image_data.height;

        // Copy the data and free the native buffer
        let data = if !image_data.data.is_null() && image_data.data_len > 0 {
            let slice = unsafe { std::slice::from_raw_parts(image_data.data, image_data.data_len) };
            let data = slice.to_vec();

            unsafe {
                mln_image_free(&mut image_data);
            }

            data
        } else {
            Vec::new()
        };

        Ok(RenderedImage {
            data,
            width,
            height,
        })
    }

    /// Render a tile at the given coordinates
    pub fn render_tile(
        &mut self,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
        pixel_ratio: f32,
    ) -> Result<RenderedImage> {
        // Calculate center of tile
        let n = 2_f64.powi(z as i32);
        let lon = (x as f64 + 0.5) / n * 360.0 - 180.0;
        let lat_rad = ((1.0 - 2.0 * (y as f64 + 0.5) / n) * std::f64::consts::PI)
            .sinh()
            .atan();
        let lat = lat_rad.to_degrees();

        let options = RenderOptions {
            size: Size::new(tile_size, tile_size),
            pixel_ratio,
            camera: CameraOptions::new(lat, lon, z as f64),
            mode: MapMode::Tile,
        };

        self.render(Some(options))
    }
}

impl Drop for NativeMap {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                mln_map_destroy(self.ptr);
            }
        }
    }
}

/// Render options
#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub size: Size,
    pub pixel_ratio: f32,
    pub camera: CameraOptions,
    pub mode: MapMode,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            size: Size::new(512, 512),
            pixel_ratio: 1.0,
            camera: CameraOptions::default(),
            mode: MapMode::Tile,
        }
    }
}

impl RenderOptions {
    fn into_native(self) -> MLNRenderOptions {
        MLNRenderOptions {
            size: self.size.into(),
            pixel_ratio: self.pixel_ratio,
            camera: self.camera.into(),
            mode: self.mode.into(),
            debug: maplibre_native_sys::MLNDebugOptions::MLN_DEBUG_NONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        assert!(init().is_ok());
    }

    #[test]
    fn test_size_conversion() {
        let size = Size::new(512, 256);
        let native: MLNSize = size.into();
        assert_eq!(native.width, 512);
        assert_eq!(native.height, 256);
    }

    #[test]
    fn test_camera_options() {
        let camera = CameraOptions::new(37.8, -122.4, 12.0)
            .with_bearing(45.0)
            .with_pitch(30.0);

        assert_eq!(camera.latitude, 37.8);
        assert_eq!(camera.longitude, -122.4);
        assert_eq!(camera.zoom, 12.0);
        assert_eq!(camera.bearing, 45.0);
        assert_eq!(camera.pitch, 30.0);
    }
}
