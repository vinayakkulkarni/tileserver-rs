//! Native MapLibre GL rendering using FFI bindings
//!
//! This module provides safe Rust wrappers around the MapLibre Native C API.
//! It is designed for server-side rendering of map tiles and static images.

use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::Once;

use image::ImageEncoder;

use mbgl_sys::{
    MLNCameraOptions, MLNErrorCode, MLNHeadlessFrontend, MLNImageData, MLNMap, MLNMapMode,
    MLNRenderOptions, MLNSize, mln_get_last_error, mln_headless_frontend_create,
    mln_headless_frontend_destroy, mln_image_free, mln_init, mln_map_create, mln_map_destroy,
    mln_map_load_style, mln_map_render_still, mln_map_set_size,
};

use crate::error::{Result, TileServerError};

static INIT: Once = Once::new();

/// Initialize the MapLibre Native library.
/// This is called automatically when needed but can be called explicitly.
///
/// # Errors
///
/// [`TileServerError::RenderError`] when MapLibre Native fails to
/// initialise.
pub fn init() -> Result<()> {
    let mut result = Ok(());

    INIT.call_once(|| {
        // SAFETY: mln_init() is a one-time initialization function safe to call once
        let code = unsafe { mln_init() };
        if code != MLNErrorCode::MLN_OK {
            result = Err(TileServerError::RenderError(format!(
                "failed to initialize MapLibre Native: {:?}",
                get_last_error()
            )));
        }
    });

    result
}

/// Get the last error message from MapLibre Native.
fn get_last_error() -> Option<String> {
    // SAFETY: mln_get_last_error() returns a valid C string pointer or null, safe to check and convert
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
#[non_exhaustive]
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

    /// Convert to PNG format with proper compression
    ///
    /// Uses zlib level 6 (Default) with adaptive row filtering for a good
    /// balance between compression ratio and encoding speed. This produces
    /// tiles comparable in size to tileserver-gl (~30-40 KB for a typical
    /// 512×512 map tile vs ~134 KB with the `image` crate's default Fast
    /// compression).
    ///
    /// # Errors
    ///
    /// [`TileServerError::RenderError`] when the pixels cannot be encoded.
    pub fn to_png(&self) -> Result<Vec<u8>> {
        use image::codecs::png::{CompressionType, FilterType, PngEncoder};

        let estimated = (self.width * self.height) as usize;
        let mut buffer = Vec::with_capacity(estimated);

        let encoder = PngEncoder::new_with_quality(
            &mut buffer,
            CompressionType::Default, // zlib level 6 (balanced speed/size)
            FilterType::Adaptive,     // tries all 5 filter types per row
        );

        encoder
            .write_image(
                &self.data, // no clone — write_image takes a slice
                self.width,
                self.height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| TileServerError::RenderError(format!("PNG encoding failed: {}", e)))?;

        Ok(buffer)
    }

    /// Convert to JPEG format
    ///
    /// # Errors
    ///
    /// [`TileServerError::RenderError`] when the pixels cannot be encoded.
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
                TileServerError::RenderError("failed to create image buffer".to_string())
            })?;

        // Encode to JPEG
        let estimated = (self.width * self.height) as usize;
        let mut buffer = Vec::with_capacity(estimated);
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
    ///
    /// # Errors
    ///
    /// [`TileServerError::RenderError`] when the pixels cannot be encoded.
    pub fn to_webp(&self, _quality: u8) -> Result<Vec<u8>> {
        use image::codecs::webp::WebPEncoder;

        let estimated = (self.width * self.height) as usize;
        let mut buffer = Vec::with_capacity(estimated);

        let encoder = WebPEncoder::new_lossless(&mut buffer);
        encoder
            .write_image(
                &self.data,
                self.width,
                self.height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| TileServerError::RenderError(format!("WebP encoding failed: {}", e)))?;

        Ok(buffer)
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
    ///
    /// # Errors
    ///
    /// [`TileServerError::RenderError`] when the native frontend cannot be
    /// created.
    pub fn new(size: Size, pixel_ratio: f32) -> Result<Self> {
        init()?;

        // SAFETY: mln_headless_frontend_create() is a valid FFI function that returns a pointer or null
        let ptr = unsafe { mln_headless_frontend_create(size.into(), pixel_ratio) };

        if ptr.is_null() {
            return Err(TileServerError::RenderError(
                get_last_error().unwrap_or_else(|| "Failed to create frontend".to_string()),
            ));
        }

        Ok(Self { ptr })
    }

    /// Get the raw pointer (for internal use)
    pub(crate) fn as_ptr(&self) -> *mut MLNHeadlessFrontend {
        self.ptr
    }
}

impl Drop for HeadlessFrontend {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: self.ptr is a valid non-null pointer created in HeadlessFrontend::new()
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
    ///
    /// # Errors
    ///
    /// [`TileServerError::RenderError`] when the native map cannot be
    /// created.
    pub fn new(size: Size, pixel_ratio: f32, mode: MapMode) -> Result<Self> {
        let frontend = HeadlessFrontend::new(size, pixel_ratio)?;

        // SAFETY: frontend.as_ptr() is a valid non-null pointer from HeadlessFrontend::new()
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

    /// Load a style JSON
    ///
    /// # Errors
    ///
    /// [`TileServerError::RenderError`] when the style JSON contains a null
    /// byte or MapLibre rejects it.
    pub fn load_style(&mut self, style_json: &str) -> Result<()> {
        let c_style = CString::new(style_json).map_err(|_| {
            TileServerError::RenderError("style JSON contains null bytes".to_string())
        })?;

        // SAFETY: self.ptr is a valid non-null map pointer, c_style.as_ptr() is a valid C string
        let code = unsafe { mln_map_load_style(self.ptr, c_style.as_ptr()) };

        if code != MLNErrorCode::MLN_OK {
            return Err(TileServerError::RenderError(
                get_last_error().unwrap_or_else(|| format!("Failed to load style: {:?}", code)),
            ));
        }

        Ok(())
    }

    pub fn set_size(&mut self, size: Size) {
        // SAFETY: self.ptr is a valid non-null map pointer created in NativeMap::new()
        unsafe {
            mln_map_set_size(self.ptr, size.into());
        }
    }

    /// Render a still image synchronously
    ///
    /// # Errors
    ///
    /// [`TileServerError::RenderError`] when MapLibre Native fails to render.
    pub fn render(&mut self, options: Option<RenderOptions>) -> Result<RenderedImage> {
        let mut image_data = MLNImageData::default();

        let c_options = options.map(|o| o.into_native());

        // SAFETY: self.ptr is a valid non-null map pointer, c_options is a valid reference or null, image_data is a valid mutable reference
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
            // SAFETY: image_data.data is a valid pointer from mln_map_render_still(), image_data.data_len is the correct size
            let slice = unsafe { std::slice::from_raw_parts(image_data.data, image_data.data_len) };
            let data = slice.to_vec();

            // SAFETY: image_data is a valid mutable reference to data returned from mln_map_render_still()
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
    ///
    /// # Errors
    ///
    /// [`TileServerError::RenderError`] when MapLibre Native fails to render.
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
            // SAFETY: self.ptr is a valid non-null map pointer created in NativeMap::new()
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
            debug: mbgl_sys::MLNDebugOptions::MLN_DEBUG_NONE,
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
