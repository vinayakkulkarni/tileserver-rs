/**
 * MapLibre Native C API - Stub Implementation
 *
 * This is a stub implementation for development and testing.
 * It will be replaced with the real MapLibre GL Native bindings.
 */

#include "maplibre_c.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/* Thread-local error message */
static __thread char last_error[1024] = {0};

/* Stub structures */
struct MLNHeadlessFrontend {
    MLNSize size;
    float pixel_ratio;
};

struct MLNMap {
    MLNHeadlessFrontend* frontend;
    float pixel_ratio;
    MLNMapMode mode;
    MLNCameraOptions camera;
    MLNDebugOptions debug;
    char* style_json;
    bool loaded;
    MLNResourceCallback request_callback;
    void* user_data;
};

static bool initialized = false;

MLNErrorCode mln_init(void) {
    if (initialized) {
        return MLN_OK;
    }
    initialized = true;
    fprintf(stderr, "[maplibre-native-sys] Stub implementation initialized\n");
    fprintf(stderr, "[maplibre-native-sys] WARNING: This is not a real MapLibre Native renderer!\n");
    fprintf(stderr, "[maplibre-native-sys] Real rendering requires building with MapLibre GL Native.\n");
    return MLN_OK;
}

void mln_cleanup(void) {
    initialized = false;
}

MLNHeadlessFrontend* mln_headless_frontend_create(MLNSize size, float pixel_ratio) {
    MLNHeadlessFrontend* frontend = (MLNHeadlessFrontend*)calloc(1, sizeof(MLNHeadlessFrontend));
    if (!frontend) {
        snprintf(last_error, sizeof(last_error), "Failed to allocate frontend");
        return NULL;
    }
    frontend->size = size;
    frontend->pixel_ratio = pixel_ratio;
    return frontend;
}

void mln_headless_frontend_destroy(MLNHeadlessFrontend* frontend) {
    if (frontend) {
        free(frontend);
    }
}

void mln_headless_frontend_set_size(MLNHeadlessFrontend* frontend, MLNSize size) {
    if (frontend) {
        frontend->size = size;
    }
}

MLNSize mln_headless_frontend_get_size(MLNHeadlessFrontend* frontend) {
    if (frontend) {
        return frontend->size;
    }
    MLNSize empty = {0, 0};
    return empty;
}

MLNMap* mln_map_create(MLNHeadlessFrontend* frontend, float pixel_ratio, MLNMapMode mode) {
    return mln_map_create_with_loader(frontend, pixel_ratio, mode, NULL, NULL);
}

MLNMap* mln_map_create_with_loader(
    MLNHeadlessFrontend* frontend,
    float pixel_ratio,
    MLNMapMode mode,
    MLNResourceCallback request_callback,
    void* user_data
) {
    if (!frontend) {
        snprintf(last_error, sizeof(last_error), "Frontend is NULL");
        return NULL;
    }

    MLNMap* map = (MLNMap*)calloc(1, sizeof(MLNMap));
    if (!map) {
        snprintf(last_error, sizeof(last_error), "Failed to allocate map");
        return NULL;
    }

    map->frontend = frontend;
    map->pixel_ratio = pixel_ratio;
    map->mode = mode;
    map->camera.latitude = 0;
    map->camera.longitude = 0;
    map->camera.zoom = 0;
    map->camera.bearing = 0;
    map->camera.pitch = 0;
    map->debug = MLN_DEBUG_NONE;
    map->style_json = NULL;
    map->loaded = false;
    map->request_callback = request_callback;
    map->user_data = user_data;

    return map;
}

void mln_map_destroy(MLNMap* map) {
    if (map) {
        if (map->style_json) {
            free(map->style_json);
        }
        free(map);
    }
}

MLNErrorCode mln_map_load_style(MLNMap* map, const char* style_json) {
    if (!map) {
        snprintf(last_error, sizeof(last_error), "Map is NULL");
        return MLN_ERROR_INVALID_ARGUMENT;
    }
    if (!style_json) {
        snprintf(last_error, sizeof(last_error), "Style JSON is NULL");
        return MLN_ERROR_INVALID_ARGUMENT;
    }

    if (map->style_json) {
        free(map->style_json);
    }

    map->style_json = strdup(style_json);
    if (!map->style_json) {
        snprintf(last_error, sizeof(last_error), "Failed to copy style JSON");
        return MLN_ERROR_UNKNOWN;
    }

    map->loaded = true;
    return MLN_OK;
}

MLNErrorCode mln_map_load_style_url(MLNMap* map, const char* url) {
    if (!map) {
        snprintf(last_error, sizeof(last_error), "Map is NULL");
        return MLN_ERROR_INVALID_ARGUMENT;
    }
    if (!url) {
        snprintf(last_error, sizeof(last_error), "URL is NULL");
        return MLN_ERROR_INVALID_ARGUMENT;
    }

    /* In a real implementation, this would fetch the URL */
    snprintf(last_error, sizeof(last_error), "URL loading not implemented in stub");
    return MLN_ERROR_NOT_LOADED;
}

bool mln_map_is_fully_loaded(MLNMap* map) {
    return map && map->loaded;
}

void mln_map_set_camera(MLNMap* map, const MLNCameraOptions* camera) {
    if (map && camera) {
        map->camera = *camera;
    }
}

MLNCameraOptions mln_map_get_camera(MLNMap* map) {
    if (map) {
        return map->camera;
    }
    MLNCameraOptions empty = {0};
    return empty;
}

void mln_map_set_size(MLNMap* map, MLNSize size) {
    if (map && map->frontend) {
        map->frontend->size = size;
    }
}

void mln_map_set_debug(MLNMap* map, MLNDebugOptions options) {
    if (map) {
        map->debug = options;
    }
}

/**
 * Render a placeholder image (solid color with text overlay).
 * In a real implementation, this would render using MapLibre GL Native.
 */
MLNErrorCode mln_map_render_still(MLNMap* map, const MLNRenderOptions* options, MLNImageData* image) {
    if (!map) {
        snprintf(last_error, sizeof(last_error), "Map is NULL");
        return MLN_ERROR_INVALID_ARGUMENT;
    }
    if (!image) {
        snprintf(last_error, sizeof(last_error), "Image output is NULL");
        return MLN_ERROR_INVALID_ARGUMENT;
    }
    if (!map->loaded) {
        snprintf(last_error, sizeof(last_error), "Style not loaded");
        return MLN_ERROR_NOT_LOADED;
    }

    /* Get size from options or frontend */
    uint32_t width, height;
    if (options) {
        width = options->size.width;
        height = options->size.height;
    } else if (map->frontend) {
        width = map->frontend->size.width;
        height = map->frontend->size.height;
    } else {
        width = 512;
        height = 512;
    }

    /* Allocate image buffer (RGBA) */
    size_t data_len = (size_t)width * height * 4;
    uint8_t* data = (uint8_t*)malloc(data_len);
    if (!data) {
        snprintf(last_error, sizeof(last_error), "Failed to allocate image buffer");
        return MLN_ERROR_UNKNOWN;
    }

    /* Fill with a gradient pattern to show it's a stub */
    for (uint32_t y = 0; y < height; y++) {
        for (uint32_t x = 0; x < width; x++) {
            size_t idx = ((size_t)y * width + x) * 4;
            /* Create a simple gradient pattern */
            data[idx + 0] = (uint8_t)((x * 255) / width);      /* R */
            data[idx + 1] = (uint8_t)((y * 255) / height);     /* G */
            data[idx + 2] = 128;                                /* B */
            data[idx + 3] = 255;                                /* A */
        }
    }

    /* Draw a simple X pattern to indicate it's a stub */
    for (uint32_t i = 0; i < (width < height ? width : height); i++) {
        /* Top-left to bottom-right */
        size_t idx1 = ((size_t)i * width + i) * 4;
        data[idx1 + 0] = 255;
        data[idx1 + 1] = 0;
        data[idx1 + 2] = 0;

        /* Top-right to bottom-left */
        size_t idx2 = ((size_t)i * width + (width - 1 - i)) * 4;
        data[idx2 + 0] = 255;
        data[idx2 + 1] = 0;
        data[idx2 + 2] = 0;
    }

    image->data = data;
    image->data_len = data_len;
    image->width = width;
    image->height = height;

    return MLN_OK;
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

    MLNImageData image = {0};
    MLNErrorCode error = mln_map_render_still(map, options, &image);

    callback(error, error == MLN_OK ? &image : NULL, user_data);
}

void mln_image_free(MLNImageData* image) {
    if (image && image->data) {
        free(image->data);
        image->data = NULL;
        image->data_len = 0;
        image->width = 0;
        image->height = 0;
    }
}

const char* mln_get_last_error(void) {
    return last_error[0] ? last_error : NULL;
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
    (void)map;
    (void)id;
    (void)data;
    (void)width;
    (void)height;
    (void)pixel_ratio;
    (void)sdf;
    /* Stub: just return OK */
    return MLN_OK;
}

MLNErrorCode mln_map_remove_image(MLNMap* map, const char* id) {
    (void)map;
    (void)id;
    /* Stub: just return OK */
    return MLN_OK;
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
