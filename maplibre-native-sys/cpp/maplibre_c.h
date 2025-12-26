/**
 * MapLibre Native C API
 *
 * This is a C wrapper around the MapLibre GL Native C++ library
 * to enable FFI bindings from Rust.
 *
 * The API is designed to be minimal and focused on server-side
 * tile rendering (headless rendering).
 */

#ifndef MAPLIBRE_C_H
#define MAPLIBRE_C_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque types */
typedef struct MLNMap MLNMap;
typedef struct MLNHeadlessFrontend MLNHeadlessFrontend;
typedef struct MLNRenderedImage MLNRenderedImage;
typedef struct MLNFileSource MLNFileSource;
typedef struct MLNResourceLoader MLNResourceLoader;

/* Error codes */
typedef enum {
    MLN_OK = 0,
    MLN_ERROR_INVALID_ARGUMENT = 1,
    MLN_ERROR_STYLE_PARSE = 2,
    MLN_ERROR_RENDER_FAILED = 3,
    MLN_ERROR_NOT_LOADED = 4,
    MLN_ERROR_TIMEOUT = 5,
    MLN_ERROR_UNKNOWN = 99,
} MLNErrorCode;

/* Map mode */
typedef enum {
    MLN_MAP_MODE_STATIC = 0,
    MLN_MAP_MODE_TILE = 1,
} MLNMapMode;

/* Debug options (bitflags) */
typedef enum {
    MLN_DEBUG_NONE = 0,
    MLN_DEBUG_TILE_BORDERS = 1 << 0,
    MLN_DEBUG_PARSE_STATUS = 1 << 1,
    MLN_DEBUG_TIMESTAMPS = 1 << 2,
    MLN_DEBUG_COLLISION = 1 << 3,
    MLN_DEBUG_OVERDRAW = 1 << 4,
} MLNDebugOptions;

/* Size structure */
typedef struct {
    uint32_t width;
    uint32_t height;
} MLNSize;

/* Camera options for rendering */
typedef struct {
    double latitude;
    double longitude;
    double zoom;
    double bearing;
    double pitch;
} MLNCameraOptions;

/* Render options */
typedef struct {
    MLNSize size;
    float pixel_ratio;
    MLNCameraOptions camera;
    MLNMapMode mode;
    MLNDebugOptions debug;
} MLNRenderOptions;

/* Rendered image data */
typedef struct {
    uint8_t* data;           /* RGBA pixel data (premultiplied alpha) */
    size_t data_len;         /* Length in bytes (width * height * 4) */
    uint32_t width;          /* Image width in pixels */
    uint32_t height;         /* Image height in pixels */
} MLNImageData;

/* Resource request (for custom file source) */
typedef struct {
    const char* url;
    uint8_t kind;  /* 0=Unknown, 1=Style, 2=Source, 3=Tile, 4=Glyphs, 5=SpriteImage, 6=SpriteJSON */
} MLNResourceRequest;

/* Resource response */
typedef struct {
    const uint8_t* data;
    size_t data_len;
    const char* error;      /* NULL if no error */
    bool not_found;         /* true if 404 */
} MLNResourceResponse;

/* Callback types */
typedef void (*MLNRenderCallback)(MLNErrorCode error, MLNImageData* image, void* user_data);
typedef void (*MLNResourceCallback)(const MLNResourceRequest* request,
                                    MLNResourceResponse* response,
                                    void* user_data);

/**
 * Initialize the MapLibre Native library.
 * Must be called once before using any other functions.
 * Returns MLN_OK on success.
 */
MLNErrorCode mln_init(void);

/**
 * Cleanup the MapLibre Native library.
 * Should be called when done using the library.
 */
void mln_cleanup(void);

/**
 * Create a new headless frontend for rendering.
 * @param size Initial size of the render target
 * @param pixel_ratio Pixel ratio (1.0 for standard, 2.0 for retina)
 * @return Pointer to frontend or NULL on error
 */
MLNHeadlessFrontend* mln_headless_frontend_create(MLNSize size, float pixel_ratio);

/**
 * Destroy a headless frontend.
 */
void mln_headless_frontend_destroy(MLNHeadlessFrontend* frontend);

/**
 * Set the size of the headless frontend.
 */
void mln_headless_frontend_set_size(MLNHeadlessFrontend* frontend, MLNSize size);

/**
 * Get the size of the headless frontend.
 */
MLNSize mln_headless_frontend_get_size(MLNHeadlessFrontend* frontend);

/**
 * Create a new map instance.
 * @param frontend Headless frontend to use for rendering
 * @param pixel_ratio Pixel ratio
 * @param mode Map mode (static or tile)
 * @return Pointer to map or NULL on error
 */
MLNMap* mln_map_create(MLNHeadlessFrontend* frontend, float pixel_ratio, MLNMapMode mode);

/**
 * Create a new map instance with custom resource loader.
 * @param frontend Headless frontend to use for rendering
 * @param pixel_ratio Pixel ratio
 * @param mode Map mode
 * @param request_callback Callback for resource requests
 * @param user_data User data passed to callbacks
 * @return Pointer to map or NULL on error
 */
MLNMap* mln_map_create_with_loader(
    MLNHeadlessFrontend* frontend,
    float pixel_ratio,
    MLNMapMode mode,
    MLNResourceCallback request_callback,
    void* user_data
);

/**
 * Destroy a map instance.
 */
void mln_map_destroy(MLNMap* map);

/**
 * Load a style JSON into the map.
 * @param map The map instance
 * @param style_json JSON string containing the style
 * @return Error code
 */
MLNErrorCode mln_map_load_style(MLNMap* map, const char* style_json);

/**
 * Load a style from a URL.
 * @param map The map instance
 * @param url URL to the style JSON
 * @return Error code
 */
MLNErrorCode mln_map_load_style_url(MLNMap* map, const char* url);

/**
 * Check if the style is fully loaded.
 */
bool mln_map_is_fully_loaded(MLNMap* map);

/**
 * Set camera options.
 */
void mln_map_set_camera(MLNMap* map, const MLNCameraOptions* camera);

/**
 * Get current camera options.
 */
MLNCameraOptions mln_map_get_camera(MLNMap* map);

/**
 * Set map size.
 */
void mln_map_set_size(MLNMap* map, MLNSize size);

/**
 * Set debug options.
 */
void mln_map_set_debug(MLNMap* map, MLNDebugOptions options);

/**
 * Render a still image synchronously.
 * @param map The map instance
 * @param options Render options (can be NULL to use current state)
 * @param image Output image data (caller must free with mln_image_free)
 * @return Error code
 */
MLNErrorCode mln_map_render_still(MLNMap* map, const MLNRenderOptions* options, MLNImageData* image);

/**
 * Render a still image asynchronously.
 * @param map The map instance
 * @param options Render options
 * @param callback Callback when rendering is complete
 * @param user_data User data passed to callback
 */
void mln_map_render_still_async(
    MLNMap* map,
    const MLNRenderOptions* options,
    MLNRenderCallback callback,
    void* user_data
);

/**
 * Free image data returned by mln_map_render_still.
 */
void mln_image_free(MLNImageData* image);

/**
 * Get the last error message.
 * @return Static string describing the last error, or NULL if no error
 */
const char* mln_get_last_error(void);

/**
 * Add a custom image to the map's style.
 * @param map The map instance
 * @param id Image ID
 * @param data RGBA pixel data
 * @param width Image width
 * @param height Image height
 * @param pixel_ratio Pixel ratio of the image
 * @param sdf True if this is an SDF image
 * @return Error code
 */
MLNErrorCode mln_map_add_image(
    MLNMap* map,
    const char* id,
    const uint8_t* data,
    uint32_t width,
    uint32_t height,
    float pixel_ratio,
    bool sdf
);

/**
 * Remove an image from the map's style.
 */
MLNErrorCode mln_map_remove_image(MLNMap* map, const char* id);

/**
 * Set the base path for local file resources.
 */
void mln_set_base_path(const char* path);

/**
 * Set the API key for MapTiler/Mapbox style URLs.
 */
void mln_set_api_key(const char* key);

#ifdef __cplusplus
}
#endif

#endif /* MAPLIBRE_C_H */
