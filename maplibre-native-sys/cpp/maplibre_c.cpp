/**
 * MapLibre Native C API Implementation
 *
 * This is a C wrapper around the MapLibre GL Native C++ library
 * to enable FFI bindings from Rust.
 */

#include "maplibre_c.h"

#include <mbgl/gfx/headless_frontend.hpp>
#include <mbgl/map/map.hpp>
#include <mbgl/map/map_options.hpp>
#include <mbgl/storage/resource_options.hpp>
#include <mbgl/style/style.hpp>
#include <mbgl/util/image.hpp>
#include <mbgl/util/run_loop.hpp>
#include <mbgl/util/premultiply.hpp>
#include <mbgl/util/logging.hpp>

#include <cstring>
#include <memory>
#include <string>
#include <thread>
#include <mutex>

/* Thread-local error message */
static thread_local char last_error[1024] = {0};

/* Thread-local RunLoop - each thread gets its own */
static thread_local std::unique_ptr<mbgl::util::RunLoop> threadRunLoop;

/* Global initialization state */
static bool initialized = false;
static std::mutex initMutex;

/* Silent log observer that suppresses all MapLibre logs */
class SilentLogObserver : public mbgl::Log::Observer {
public:
    bool onRecord(mbgl::EventSeverity, mbgl::Event, int64_t, const std::string&) override {
        return true; // Consume all messages
    }
};

/* Flag to track if logging is suppressed */
static bool loggingSuppressed = false;

/* Ensure the current thread has a RunLoop */
static void ensureRunLoop() {
    if (!threadRunLoop) {
        threadRunLoop = std::make_unique<mbgl::util::RunLoop>();
    }
}

/* Internal structures wrapping MapLibre Native objects */
struct MLNHeadlessFrontend {
    std::unique_ptr<mbgl::HeadlessFrontend> frontend;
    float pixelRatio;
    mbgl::Size size;
};

struct MLNMap {
    MLNHeadlessFrontend* frontend;
    std::unique_ptr<mbgl::Map> map;
    float pixelRatio;
    MLNMapMode mode;
    bool styleLoaded;
};

extern "C" {

MLNErrorCode mln_init(void) {
    std::lock_guard<std::mutex> lock(initMutex);
    
    if (initialized) {
        return MLN_OK;
    }
    
    try {
        // Ensure the calling thread has a RunLoop
        ensureRunLoop();
        initialized = true;
        return MLN_OK;
    } catch (const std::exception& e) {
        snprintf(last_error, sizeof(last_error), "Failed to initialize: %s", e.what());
        return MLN_ERROR_UNKNOWN;
    }
}

void mln_cleanup(void) {
    std::lock_guard<std::mutex> lock(initMutex);
    if (initialized) {
        // Note: thread-local RunLoops are cleaned up when threads exit
        initialized = false;
    }
}

MLNHeadlessFrontend* mln_headless_frontend_create(MLNSize size, float pixel_ratio) {
    if (!initialized) {
        snprintf(last_error, sizeof(last_error), "Library not initialized");
        return nullptr;
    }
    
    try {
        // Ensure this thread has a RunLoop
        ensureRunLoop();
        
        auto frontend = new MLNHeadlessFrontend();
        frontend->pixelRatio = pixel_ratio;
        frontend->size = mbgl::Size{size.width, size.height};
        frontend->frontend = std::make_unique<mbgl::HeadlessFrontend>(
            frontend->size,
            pixel_ratio
        );
        return frontend;
    } catch (const std::exception& e) {
        snprintf(last_error, sizeof(last_error), "Failed to create frontend: %s", e.what());
        return nullptr;
    }
}

void mln_headless_frontend_destroy(MLNHeadlessFrontend* frontend) {
    delete frontend;
}

void mln_headless_frontend_set_size(MLNHeadlessFrontend* frontend, MLNSize size) {
    if (frontend && frontend->frontend) {
        frontend->size = mbgl::Size{size.width, size.height};
        frontend->frontend->setSize(frontend->size);
    }
}

MLNSize mln_headless_frontend_get_size(MLNHeadlessFrontend* frontend) {
    if (frontend) {
        return MLNSize{frontend->size.width, frontend->size.height};
    }
    return MLNSize{0, 0};
}

MLNMap* mln_map_create(MLNHeadlessFrontend* frontend, float pixel_ratio, MLNMapMode mode) {
    return mln_map_create_with_loader(frontend, pixel_ratio, mode, nullptr, nullptr);
}

MLNMap* mln_map_create_with_loader(
    MLNHeadlessFrontend* frontend,
    float pixel_ratio,
    MLNMapMode mode,
    MLNResourceCallback request_callback,
    void* user_data
) {
    if (!initialized) {
        snprintf(last_error, sizeof(last_error), "Library not initialized");
        return nullptr;
    }
    
    if (!frontend || !frontend->frontend) {
        snprintf(last_error, sizeof(last_error), "Invalid frontend");
        return nullptr;
    }
    
    try {
        auto map = new MLNMap();
        map->frontend = frontend;
        map->pixelRatio = pixel_ratio;
        map->mode = mode;
        map->styleLoaded = false;
        
        // Map mode
        mbgl::MapMode mapMode = (mode == MLN_MAP_MODE_TILE) 
            ? mbgl::MapMode::Tile 
            : mbgl::MapMode::Static;
        
        // Create map options
        mbgl::MapOptions mapOptions;
        mapOptions.withSize(frontend->size)
                  .withPixelRatio(pixel_ratio)
                  .withMapMode(mapMode);
        
        // Resource options (using default file sources)
        mbgl::ResourceOptions resourceOptions;
        
        // TODO: Handle custom resource callback if provided
        (void)request_callback;
        (void)user_data;
        
        // Create the map
        map->map = std::make_unique<mbgl::Map>(
            *frontend->frontend,
            mbgl::MapObserver::nullObserver(),
            mapOptions,
            resourceOptions
        );
        
        return map;
    } catch (const std::exception& e) {
        snprintf(last_error, sizeof(last_error), "Failed to create map: %s", e.what());
        return nullptr;
    }
}

void mln_map_destroy(MLNMap* map) {
    delete map;
}

MLNErrorCode mln_map_load_style(MLNMap* map, const char* style_json) {
    if (!map || !map->map) {
        snprintf(last_error, sizeof(last_error), "Invalid map");
        return MLN_ERROR_INVALID_ARGUMENT;
    }
    
    if (!style_json) {
        snprintf(last_error, sizeof(last_error), "Style JSON is null");
        return MLN_ERROR_INVALID_ARGUMENT;
    }
    
    // Ensure this thread has a RunLoop
    ensureRunLoop();
    
    try {
        map->map->getStyle().loadJSON(style_json);
        map->styleLoaded = true;
        return MLN_OK;
    } catch (const std::exception& e) {
        snprintf(last_error, sizeof(last_error), "Style parse error: %s", e.what());
        return MLN_ERROR_STYLE_PARSE;
    }
}

MLNErrorCode mln_map_load_style_url(MLNMap* map, const char* url) {
    if (!map || !map->map) {
        snprintf(last_error, sizeof(last_error), "Invalid map");
        return MLN_ERROR_INVALID_ARGUMENT;
    }
    
    if (!url) {
        snprintf(last_error, sizeof(last_error), "URL is null");
        return MLN_ERROR_INVALID_ARGUMENT;
    }
    
    try {
        map->map->getStyle().loadURL(url);
        map->styleLoaded = true;
        return MLN_OK;
    } catch (const std::exception& e) {
        snprintf(last_error, sizeof(last_error), "Failed to load style from URL: %s", e.what());
        return MLN_ERROR_UNKNOWN;
    }
}

bool mln_map_is_fully_loaded(MLNMap* map) {
    if (!map || !map->map) {
        return false;
    }
    return map->map->isFullyLoaded();
}

void mln_map_set_camera(MLNMap* map, const MLNCameraOptions* camera) {
    if (!map || !map->map || !camera) {
        return;
    }
    
    mbgl::CameraOptions cameraOptions;
    cameraOptions.center = mbgl::LatLng{camera->latitude, camera->longitude};
    cameraOptions.zoom = camera->zoom;
    cameraOptions.bearing = camera->bearing;
    cameraOptions.pitch = camera->pitch;
    
    map->map->jumpTo(cameraOptions);
}

MLNCameraOptions mln_map_get_camera(MLNMap* map) {
    MLNCameraOptions result = {0, 0, 0, 0, 0};
    
    if (!map || !map->map) {
        return result;
    }
    
    auto camera = map->map->getCameraOptions();
    if (camera.center) {
        result.latitude = camera.center->latitude();
        result.longitude = camera.center->longitude();
    }
    if (camera.zoom) {
        result.zoom = *camera.zoom;
    }
    if (camera.bearing) {
        result.bearing = *camera.bearing;
    }
    if (camera.pitch) {
        result.pitch = *camera.pitch;
    }
    
    return result;
}

void mln_map_set_size(MLNMap* map, MLNSize size) {
    if (!map || !map->map || !map->frontend) {
        return;
    }
    
    mbgl::Size newSize{size.width, size.height};
    map->frontend->size = newSize;
    map->frontend->frontend->setSize(newSize);
    map->map->setSize(newSize);
}

void mln_map_set_debug(MLNMap* map, MLNDebugOptions options) {
    if (!map || !map->map) {
        return;
    }
    
    mbgl::MapDebugOptions debugOptions = mbgl::MapDebugOptions::NoDebug;
    
    if (options & MLN_DEBUG_TILE_BORDERS) {
        debugOptions = debugOptions | mbgl::MapDebugOptions::TileBorders;
    }
    if (options & MLN_DEBUG_PARSE_STATUS) {
        debugOptions = debugOptions | mbgl::MapDebugOptions::ParseStatus;
    }
    if (options & MLN_DEBUG_TIMESTAMPS) {
        debugOptions = debugOptions | mbgl::MapDebugOptions::Timestamps;
    }
    if (options & MLN_DEBUG_COLLISION) {
        debugOptions = debugOptions | mbgl::MapDebugOptions::Collision;
    }
    if (options & MLN_DEBUG_OVERDRAW) {
        debugOptions = debugOptions | mbgl::MapDebugOptions::Overdraw;
    }
    
    map->map->setDebug(debugOptions);
}

MLNErrorCode mln_map_render_still(MLNMap* map, const MLNRenderOptions* options, MLNImageData* image) {
    if (!map || !map->map || !map->frontend || !map->frontend->frontend) {
        snprintf(last_error, sizeof(last_error), "Invalid map");
        return MLN_ERROR_INVALID_ARGUMENT;
    }
    
    if (!image) {
        snprintf(last_error, sizeof(last_error), "Image output is null");
        return MLN_ERROR_INVALID_ARGUMENT;
    }
    
    if (!map->styleLoaded) {
        snprintf(last_error, sizeof(last_error), "Style not loaded");
        return MLN_ERROR_NOT_LOADED;
    }
    
    // Ensure this thread has a RunLoop for async operations during render
    ensureRunLoop();
    
    try {
        // Apply render options if provided
        if (options) {
            // Set size
            if (options->size.width > 0 && options->size.height > 0) {
                mln_map_set_size(map, options->size);
            }
            
            // Set camera
            mln_map_set_camera(map, &options->camera);
            
            // Set debug options
            mln_map_set_debug(map, options->debug);
        }
        
        // Render
        auto renderResult = map->frontend->frontend->render(*map->map);
        
        // Copy image data
        auto& premultiplied = renderResult.image;
        size_t dataLen = premultiplied.bytes();
        
        if (dataLen == 0) {
            snprintf(last_error, sizeof(last_error), "Render produced empty image");
            return MLN_ERROR_RENDER_FAILED;
        }
        
        // Allocate and copy data
        uint8_t* data = (uint8_t*)malloc(dataLen);
        if (!data) {
            snprintf(last_error, sizeof(last_error), "Failed to allocate image buffer");
            return MLN_ERROR_UNKNOWN;
        }
        
        memcpy(data, premultiplied.data.get(), dataLen);
        
        image->data = data;
        image->data_len = dataLen;
        image->width = premultiplied.size.width;
        image->height = premultiplied.size.height;
        
        return MLN_OK;
    } catch (const std::exception& e) {
        snprintf(last_error, sizeof(last_error), "Render failed: %s", e.what());
        return MLN_ERROR_RENDER_FAILED;
    }
}

void mln_map_render_still_async(
    MLNMap* map,
    const MLNRenderOptions* options,
    MLNRenderCallback callback,
    void* user_data
) {
    if (!callback) {
        return;
    }
    
    // For now, just do synchronous rendering and call the callback
    MLNImageData image = {nullptr, 0, 0, 0};
    MLNErrorCode error = mln_map_render_still(map, options, &image);
    
    callback(error, error == MLN_OK ? &image : nullptr, user_data);
}

void mln_image_free(MLNImageData* image) {
    if (image && image->data) {
        free(image->data);
        image->data = nullptr;
        image->data_len = 0;
        image->width = 0;
        image->height = 0;
    }
}

const char* mln_get_last_error(void) {
    return last_error[0] ? last_error : nullptr;
}

MLNErrorCode mln_map_add_image(
    MLNMap* map,
    const char* id,
    const uint8_t* data,
    uint32_t width,
    uint32_t height,
    float pixel_ratio,
    bool sdf
) {
    if (!map || !map->map || !id || !data) {
        snprintf(last_error, sizeof(last_error), "Invalid arguments");
        return MLN_ERROR_INVALID_ARGUMENT;
    }
    
    try {
        // Create an unassociated image and premultiply
        size_t dataLen = width * height * 4;
        auto imageData = std::make_unique<uint8_t[]>(dataLen);
        memcpy(imageData.get(), data, dataLen);
        
        mbgl::UnassociatedImage unassocImage({width, height}, std::move(imageData));
        mbgl::PremultipliedImage premultImage = mbgl::util::premultiply(std::move(unassocImage));
        
        map->map->getStyle().addImage(
            std::make_unique<mbgl::style::Image>(
                id,
                std::move(premultImage),
                pixel_ratio,
                sdf
            )
        );
        
        return MLN_OK;
    } catch (const std::exception& e) {
        snprintf(last_error, sizeof(last_error), "Failed to add image: %s", e.what());
        return MLN_ERROR_UNKNOWN;
    }
}

MLNErrorCode mln_map_remove_image(MLNMap* map, const char* id) {
    if (!map || !map->map || !id) {
        return MLN_ERROR_INVALID_ARGUMENT;
    }
    
    try {
        map->map->getStyle().removeImage(id);
        return MLN_OK;
    } catch (const std::exception& e) {
        snprintf(last_error, sizeof(last_error), "Failed to remove image: %s", e.what());
        return MLN_ERROR_UNKNOWN;
    }
}

static char base_path[4096] = {0};
static char api_key[256] = {0};

void mln_set_base_path(const char* path) {
    if (path) {
        strncpy(base_path, path, sizeof(base_path) - 1);
    }
}

void mln_set_api_key(const char* key) {
    if (key) {
        strncpy(api_key, key, sizeof(api_key) - 1);
    }
}

} // extern "C"
