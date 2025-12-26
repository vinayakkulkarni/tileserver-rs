//! Low-level FFI bindings to MapLibre GL Native
//!
//! This crate provides unsafe Rust bindings to the MapLibre GL Native C API.
//! It is intended to be used by higher-level safe wrappers.
//!
//! # Safety
//!
//! All functions in this crate are unsafe and require careful handling of:
//! - Pointer validity
//! - Memory ownership
//! - Thread safety
//!
//! Users should prefer the safe `maplibre-native` wrapper crate instead.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use libc::{c_char, c_double, c_float, c_uchar, c_uint, c_void, size_t};

/// Opaque type for MLNMap
#[repr(C)]
pub struct MLNMap {
    _private: [u8; 0],
}

/// Opaque type for MLNHeadlessFrontend
#[repr(C)]
pub struct MLNHeadlessFrontend {
    _private: [u8; 0],
}

/// Error codes returned by the API
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MLNErrorCode {
    MLN_OK = 0,
    MLN_ERROR_INVALID_ARGUMENT = 1,
    MLN_ERROR_STYLE_PARSE = 2,
    MLN_ERROR_RENDER_FAILED = 3,
    MLN_ERROR_NOT_LOADED = 4,
    MLN_ERROR_TIMEOUT = 5,
    MLN_ERROR_UNKNOWN = 99,
}

/// Map rendering mode
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MLNMapMode {
    MLN_MAP_MODE_STATIC = 0,
    MLN_MAP_MODE_TILE = 1,
}

/// Debug options (bitflags)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MLNDebugOptions {
    MLN_DEBUG_NONE = 0,
    MLN_DEBUG_TILE_BORDERS = 1,
    MLN_DEBUG_PARSE_STATUS = 2,
    MLN_DEBUG_TIMESTAMPS = 4,
    MLN_DEBUG_COLLISION = 8,
    MLN_DEBUG_OVERDRAW = 16,
}

/// Size structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MLNSize {
    pub width: c_uint,
    pub height: c_uint,
}

impl MLNSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Camera options for rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MLNCameraOptions {
    pub latitude: c_double,
    pub longitude: c_double,
    pub zoom: c_double,
    pub bearing: c_double,
    pub pitch: c_double,
}

impl MLNCameraOptions {
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

/// Render options
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MLNRenderOptions {
    pub size: MLNSize,
    pub pixel_ratio: c_float,
    pub camera: MLNCameraOptions,
    pub mode: MLNMapMode,
    pub debug: MLNDebugOptions,
}

impl Default for MLNRenderOptions {
    fn default() -> Self {
        Self {
            size: MLNSize::new(512, 512),
            pixel_ratio: 1.0,
            camera: MLNCameraOptions::default(),
            mode: MLNMapMode::MLN_MAP_MODE_TILE,
            debug: MLNDebugOptions::MLN_DEBUG_NONE,
        }
    }
}

/// Rendered image data
#[repr(C)]
#[derive(Debug)]
pub struct MLNImageData {
    /// RGBA pixel data (premultiplied alpha)
    pub data: *mut c_uchar,
    /// Length in bytes (width * height * 4)
    pub data_len: size_t,
    /// Image width in pixels
    pub width: c_uint,
    /// Image height in pixels
    pub height: c_uint,
}

impl Default for MLNImageData {
    fn default() -> Self {
        Self {
            data: std::ptr::null_mut(),
            data_len: 0,
            width: 0,
            height: 0,
        }
    }
}

/// Resource request (for custom file source)
#[repr(C)]
#[derive(Debug)]
pub struct MLNResourceRequest {
    pub url: *const c_char,
    /// 0=Unknown, 1=Style, 2=Source, 3=Tile, 4=Glyphs, 5=SpriteImage, 6=SpriteJSON
    pub kind: c_uchar,
}

/// Resource response
#[repr(C)]
#[derive(Debug)]
pub struct MLNResourceResponse {
    pub data: *const c_uchar,
    pub data_len: size_t,
    /// NULL if no error
    pub error: *const c_char,
    /// true if 404
    pub not_found: bool,
}

impl Default for MLNResourceResponse {
    fn default() -> Self {
        Self {
            data: std::ptr::null(),
            data_len: 0,
            error: std::ptr::null(),
            not_found: false,
        }
    }
}

/// Callback type for async rendering
pub type MLNRenderCallback = Option<
    unsafe extern "C" fn(error: MLNErrorCode, image: *mut MLNImageData, user_data: *mut c_void),
>;

/// Callback type for resource requests
pub type MLNResourceCallback = Option<
    unsafe extern "C" fn(
        request: *const MLNResourceRequest,
        response: *mut MLNResourceResponse,
        user_data: *mut c_void,
    ),
>;

// External C functions - these are stubbed for now
// In a real implementation, these would link to libmaplibre-native

extern "C" {
    /// Initialize the MapLibre Native library.
    pub fn mln_init() -> MLNErrorCode;

    /// Cleanup the MapLibre Native library.
    pub fn mln_cleanup();

    /// Create a new headless frontend for rendering.
    pub fn mln_headless_frontend_create(
        size: MLNSize,
        pixel_ratio: c_float,
    ) -> *mut MLNHeadlessFrontend;

    /// Destroy a headless frontend.
    pub fn mln_headless_frontend_destroy(frontend: *mut MLNHeadlessFrontend);

    /// Set the size of the headless frontend.
    pub fn mln_headless_frontend_set_size(frontend: *mut MLNHeadlessFrontend, size: MLNSize);

    /// Get the size of the headless frontend.
    pub fn mln_headless_frontend_get_size(frontend: *mut MLNHeadlessFrontend) -> MLNSize;

    /// Create a new map instance.
    pub fn mln_map_create(
        frontend: *mut MLNHeadlessFrontend,
        pixel_ratio: c_float,
        mode: MLNMapMode,
    ) -> *mut MLNMap;

    /// Create a new map instance with custom resource loader.
    pub fn mln_map_create_with_loader(
        frontend: *mut MLNHeadlessFrontend,
        pixel_ratio: c_float,
        mode: MLNMapMode,
        request_callback: MLNResourceCallback,
        user_data: *mut c_void,
    ) -> *mut MLNMap;

    /// Destroy a map instance.
    pub fn mln_map_destroy(map: *mut MLNMap);

    /// Load a style JSON into the map.
    pub fn mln_map_load_style(map: *mut MLNMap, style_json: *const c_char) -> MLNErrorCode;

    /// Load a style from a URL.
    pub fn mln_map_load_style_url(map: *mut MLNMap, url: *const c_char) -> MLNErrorCode;

    /// Check if the style is fully loaded.
    pub fn mln_map_is_fully_loaded(map: *mut MLNMap) -> bool;

    /// Set camera options.
    pub fn mln_map_set_camera(map: *mut MLNMap, camera: *const MLNCameraOptions);

    /// Get current camera options.
    pub fn mln_map_get_camera(map: *mut MLNMap) -> MLNCameraOptions;

    /// Set map size.
    pub fn mln_map_set_size(map: *mut MLNMap, size: MLNSize);

    /// Set debug options.
    pub fn mln_map_set_debug(map: *mut MLNMap, options: MLNDebugOptions);

    /// Render a still image synchronously.
    pub fn mln_map_render_still(
        map: *mut MLNMap,
        options: *const MLNRenderOptions,
        image: *mut MLNImageData,
    ) -> MLNErrorCode;

    /// Render a still image asynchronously.
    pub fn mln_map_render_still_async(
        map: *mut MLNMap,
        options: *const MLNRenderOptions,
        callback: MLNRenderCallback,
        user_data: *mut c_void,
    );

    /// Free image data returned by mln_map_render_still.
    pub fn mln_image_free(image: *mut MLNImageData);

    /// Get the last error message.
    pub fn mln_get_last_error() -> *const c_char;

    /// Add a custom image to the map's style.
    pub fn mln_map_add_image(
        map: *mut MLNMap,
        id: *const c_char,
        data: *const c_uchar,
        width: c_uint,
        height: c_uint,
        pixel_ratio: c_float,
        sdf: bool,
    ) -> MLNErrorCode;

    /// Remove an image from the map's style.
    pub fn mln_map_remove_image(map: *mut MLNMap, id: *const c_char) -> MLNErrorCode;

    /// Set the base path for local file resources.
    pub fn mln_set_base_path(path: *const c_char);

    /// Set the API key for MapTiler/Mapbox style URLs.
    pub fn mln_set_api_key(key: *const c_char);
}

/// Resource kind constants
pub mod resource_kind {
    pub const UNKNOWN: u8 = 0;
    pub const STYLE: u8 = 1;
    pub const SOURCE: u8 = 2;
    pub const TILE: u8 = 3;
    pub const GLYPHS: u8 = 4;
    pub const SPRITE_IMAGE: u8 = 5;
    pub const SPRITE_JSON: u8 = 6;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_layout() {
        assert_eq!(std::mem::size_of::<MLNSize>(), 8);
    }

    #[test]
    fn test_camera_options() {
        let camera = MLNCameraOptions::new(37.8, -122.4, 12.0)
            .with_bearing(45.0)
            .with_pitch(30.0);

        assert_eq!(camera.latitude, 37.8);
        assert_eq!(camera.longitude, -122.4);
        assert_eq!(camera.zoom, 12.0);
        assert_eq!(camera.bearing, 45.0);
        assert_eq!(camera.pitch, 30.0);
    }
}
