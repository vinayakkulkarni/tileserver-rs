//! OpenAPI 3.1 specification for tileserver-rs API
//!
//! This module provides the OpenAPI specification using utoipa derive macros
//! for seamless integration with utoipa-swagger-ui.
//!
//! The structs and functions in this module are used solely for documentation
//! generation and are not called directly at runtime.

#![allow(dead_code)]

use utoipa::OpenApi;

/// OpenAPI documentation for tileserver-rs
#[derive(OpenApi)]
#[openapi(
    info(
        title = "tileserver-rs API",
        description = "High-performance vector and raster tile server with native MapLibre rendering",
        version = "0.2.1",
        license(name = "MIT", url = "https://github.com/vinayakkulkarni/tileserver-rs/blob/main/LICENSE"),
        contact(name = "Vinayak Kulkarni", url = "https://github.com/vinayakkulkarni/tileserver-rs")
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Data", description = "Vector tile data sources (PMTiles, MBTiles)"),
        (name = "Styles", description = "Map styles and raster tile rendering"),
        (name = "Fonts", description = "Font glyphs for map labels"),
        (name = "Files", description = "Static file serving")
    ),
    paths(
        health_check,
        get_index,
        list_data_sources,
        get_data_source,
        get_tile,
        list_styles,
        get_style_tilejson,
        get_style_json,
        get_raster_tile,
        get_raster_tile_with_size,
        get_static_image,
        get_sprite,
        get_wmts_capabilities,
        list_fonts,
        get_font_glyphs,
        get_static_file,
    ),
    components(schemas(
        TileJSON,
        VectorLayer,
        StyleInfo,
        GeoJSON,
        ApiError,
    ))
)]
pub struct ApiDoc;

// ============================================================
// Schema definitions
// ============================================================

/// TileJSON 3.0 metadata
#[derive(utoipa::ToSchema)]
#[schema(example = json!({
    "tilejson": "3.0.0",
    "tiles": ["http://localhost:8080/data/source/{z}/{x}/{y}.pbf"],
    "name": "OpenMapTiles",
    "minzoom": 0,
    "maxzoom": 14
}))]
pub struct TileJSON {
    /// TileJSON version (always "3.0.0")
    pub tilejson: String,
    /// Source identifier
    #[schema(nullable)]
    pub id: Option<String>,
    /// Human-readable name
    #[schema(nullable)]
    pub name: Option<String>,
    /// Description of the tileset
    #[schema(nullable)]
    pub description: Option<String>,
    /// Tile URL templates
    pub tiles: Vec<String>,
    /// Minimum zoom level
    #[schema(minimum = 0, maximum = 22)]
    pub minzoom: u8,
    /// Maximum zoom level
    #[schema(minimum = 0, maximum = 22)]
    pub maxzoom: u8,
    /// Bounding box [west, south, east, north]
    #[schema(nullable)]
    pub bounds: Option<Vec<f64>>,
    /// Center point [longitude, latitude, zoom]
    #[schema(nullable)]
    pub center: Option<Vec<f64>>,
    /// Attribution HTML
    #[schema(nullable)]
    pub attribution: Option<String>,
    /// Vector layer definitions
    #[schema(nullable)]
    pub vector_layers: Option<Vec<VectorLayer>>,
}

/// Vector layer metadata
#[derive(utoipa::ToSchema)]
pub struct VectorLayer {
    /// Layer identifier
    pub id: String,
    /// Layer description
    #[schema(nullable)]
    pub description: Option<String>,
    /// Minimum zoom level
    #[schema(nullable)]
    pub minzoom: Option<u8>,
    /// Maximum zoom level
    #[schema(nullable)]
    pub maxzoom: Option<u8>,
    /// Field names and types
    #[schema(nullable)]
    pub fields: Option<std::collections::HashMap<String, String>>,
}

/// Style metadata
#[derive(utoipa::ToSchema)]
#[schema(example = json!({
    "id": "osm-bright",
    "name": "OSM Bright",
    "url": "http://localhost:8080/styles/osm-bright/style.json"
}))]
pub struct StyleInfo {
    /// Style identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// URL to style.json
    pub url: String,
}

/// GeoJSON FeatureCollection
#[derive(utoipa::ToSchema)]
pub struct GeoJSON {
    /// Always "FeatureCollection"
    #[schema(example = "FeatureCollection")]
    pub r#type: String,
    /// Array of GeoJSON features
    pub features: Vec<serde_json::Value>,
}

/// API error response
#[derive(utoipa::ToSchema)]
#[schema(example = json!({"error": "Source not found: invalid-source"}))]
pub struct ApiError {
    /// Error message
    pub error: String,
}

// ============================================================
// Path operations (documentation only - actual handlers in main.rs)
// ============================================================

/// Health check
///
/// Returns OK if the server is running
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Server is healthy", body = String, example = "OK")
    )
)]
pub async fn health_check() {}

/// Get all sources and styles
///
/// Returns a combined list of all data sources and styles as TileJSON
#[utoipa::path(
    get,
    path = "/index.json",
    tag = "Data",
    responses(
        (status = 200, description = "Combined list of sources and styles", body = Vec<TileJSON>)
    )
)]
pub async fn get_index() {}

/// List all data sources
///
/// Returns TileJSON metadata for all available tile sources
#[utoipa::path(
    get,
    path = "/data.json",
    tag = "Data",
    responses(
        (status = 200, description = "List of data sources", body = Vec<TileJSON>)
    )
)]
pub async fn list_data_sources() {}

/// Get data source TileJSON
///
/// Returns TileJSON metadata for a specific tile source
#[utoipa::path(
    get,
    path = "/data/{source}",
    tag = "Data",
    params(
        ("source" = String, Path, description = "Source ID (with or without .json extension)")
    ),
    responses(
        (status = 200, description = "TileJSON metadata", body = TileJSON),
        (status = 404, description = "Source not found", body = ApiError)
    )
)]
pub async fn get_data_source() {}

/// Get a vector tile
///
/// Returns a vector tile in the requested format (pbf, mvt, or geojson)
#[utoipa::path(
    get,
    path = "/data/{source}/{z}/{x}/{y}.{format}",
    tag = "Data",
    params(
        ("source" = String, Path, description = "Source ID"),
        ("z" = u8, Path, description = "Zoom level (0-22)"),
        ("x" = u32, Path, description = "Tile X coordinate"),
        ("y" = u32, Path, description = "Tile Y coordinate"),
        ("format" = String, Path, description = "Tile format (pbf, mvt, geojson)")
    ),
    responses(
        (status = 200, description = "Vector tile data", content_type = "application/x-protobuf"),
        (status = 200, description = "GeoJSON tile data", body = GeoJSON, content_type = "application/geo+json"),
        (status = 404, description = "Tile not found")
    )
)]
pub async fn get_tile() {}

/// List all styles
///
/// Returns metadata for all available map styles
#[utoipa::path(
    get,
    path = "/styles.json",
    tag = "Styles",
    responses(
        (status = 200, description = "List of styles", body = Vec<StyleInfo>)
    )
)]
pub async fn list_styles() {}

/// Get style TileJSON
///
/// Returns TileJSON for raster tiles rendered from this style
#[utoipa::path(
    get,
    path = "/styles/{style}.json",
    tag = "Styles",
    params(
        ("style" = String, Path, description = "Style ID")
    ),
    responses(
        (status = 200, description = "TileJSON for raster tiles", body = TileJSON),
        (status = 404, description = "Style not found", body = ApiError)
    )
)]
pub async fn get_style_tilejson() {}

/// Get MapLibre style JSON
///
/// Returns the full MapLibre GL style specification
#[utoipa::path(
    get,
    path = "/styles/{style}/style.json",
    tag = "Styles",
    params(
        ("style" = String, Path, description = "Style ID")
    ),
    responses(
        (status = 200, description = "MapLibre style specification", content_type = "application/json"),
        (status = 404, description = "Style not found", body = ApiError)
    )
)]
pub async fn get_style_json() {}

/// Get a raster tile
///
/// Returns a raster tile rendered from the style. Supports retina with @2x suffix.
#[utoipa::path(
    get,
    path = "/styles/{style}/{z}/{x}/{y}.{format}",
    tag = "Styles",
    params(
        ("style" = String, Path, description = "Style ID"),
        ("z" = u8, Path, description = "Zoom level (0-22)"),
        ("x" = u32, Path, description = "Tile X coordinate"),
        ("y" = String, Path, description = "Tile Y coordinate (optionally with @2x for retina)", example = "123"),
        ("format" = String, Path, description = "Image format (png, jpg, jpeg, webp)")
    ),
    responses(
        (status = 200, description = "Raster tile image", content_type = "image/png"),
        (status = 404, description = "Style not found", body = ApiError)
    )
)]
pub async fn get_raster_tile() {}

/// Get a raster tile with custom size
///
/// Returns a raster tile with specified tile size (256 or 512 pixels)
#[utoipa::path(
    get,
    path = "/styles/{style}/{tileSize}/{z}/{x}/{y}.{format}",
    tag = "Styles",
    params(
        ("style" = String, Path, description = "Style ID"),
        ("tileSize" = u16, Path, description = "Tile size in pixels (256 or 512)"),
        ("z" = u8, Path, description = "Zoom level"),
        ("x" = u32, Path, description = "Tile X coordinate"),
        ("y" = String, Path, description = "Tile Y coordinate"),
        ("format" = String, Path, description = "Image format (png, jpg, jpeg, webp)")
    ),
    responses(
        (status = 200, description = "Raster tile image", content_type = "image/png")
    )
)]
pub async fn get_raster_tile_with_size() {}

/// Get a static map image
///
/// Renders a static map image centered at the specified location
#[utoipa::path(
    get,
    path = "/styles/{style}/static/{center}/{size}.{format}",
    tag = "Styles",
    params(
        ("style" = String, Path, description = "Style ID"),
        ("center" = String, Path, description = "Center point as lon,lat,zoom or 'auto'", example = "-122.4194,37.7749,12"),
        ("size" = String, Path, description = "Image size as WIDTHxHEIGHT, optionally with @2x", example = "800x600"),
        ("format" = String, Path, description = "Image format (png, jpg, jpeg, webp)"),
        ("bearing" = Option<f64>, Query, description = "Map bearing in degrees"),
        ("pitch" = Option<f64>, Query, description = "Map pitch in degrees"),
        ("markers" = Option<String>, Query, description = "Markers to add (format: pin-s+color(lon,lat))"),
        ("path" = Option<String>, Query, description = "Path to draw (format: path-width+color(lon,lat|lon,lat))")
    ),
    responses(
        (status = 200, description = "Static map image", content_type = "image/png")
    )
)]
pub async fn get_static_image() {}

/// Get sprite image or JSON
///
/// Returns sprite image (PNG) or metadata (JSON) for the style
#[utoipa::path(
    get,
    path = "/styles/{style}/sprite.{ext}",
    tag = "Styles",
    params(
        ("style" = String, Path, description = "Style ID"),
        ("ext" = String, Path, description = "File extension (png or json, optionally with @2x)", example = "png")
    ),
    responses(
        (status = 200, description = "Sprite image", content_type = "image/png"),
        (status = 200, description = "Sprite metadata", content_type = "application/json"),
        (status = 404, description = "Sprite not found", body = ApiError)
    )
)]
pub async fn get_sprite() {}

/// Get WMTS capabilities
///
/// Returns OGC WMTS GetCapabilities document for the style
#[utoipa::path(
    get,
    path = "/styles/{style}/wmts.xml",
    tag = "Styles",
    params(
        ("style" = String, Path, description = "Style ID")
    ),
    responses(
        (status = 200, description = "WMTS capabilities XML", content_type = "application/xml")
    )
)]
pub async fn get_wmts_capabilities() {}

/// List available fonts
///
/// Returns a list of available font families
#[utoipa::path(
    get,
    path = "/fonts.json",
    tag = "Fonts",
    responses(
        (status = 200, description = "List of font names", body = Vec<String>,
            example = json!(["Noto Sans Regular", "Noto Sans Bold"]))
    )
)]
pub async fn list_fonts() {}

/// Get font glyphs
///
/// Returns PBF-encoded font glyphs for a character range
#[utoipa::path(
    get,
    path = "/fonts/{fontstack}/{range}",
    tag = "Fonts",
    params(
        ("fontstack" = String, Path, description = "Font stack (comma-separated font names)", example = "Noto Sans Regular"),
        ("range" = String, Path, description = "Character range (e.g., 0-255.pbf)", example = "0-255.pbf")
    ),
    responses(
        (status = 200, description = "Font glyph data", content_type = "application/x-protobuf"),
        (status = 404, description = "Font not found", body = ApiError)
    )
)]
pub async fn get_font_glyphs() {}

/// Get static file
///
/// Serves static files from the configured files directory
#[utoipa::path(
    get,
    path = "/files/{filepath}",
    tag = "Files",
    params(
        ("filepath" = String, Path, description = "Path to the file")
    ),
    responses(
        (status = 200, description = "File content"),
        (status = 404, description = "File not found", body = ApiError)
    )
)]
pub async fn get_static_file() {}

#[cfg(test)]
mod tests {
    use super::*;
    use utoipa::OpenApi;

    #[test]
    fn test_openapi_spec_generates() {
        let spec = ApiDoc::openapi();

        // Check basic structure
        assert_eq!(spec.info.title, "tileserver-rs API");
        assert_eq!(spec.info.version, "0.2.1");

        // Check that paths exist
        let paths = spec.paths.paths;
        assert!(paths.contains_key("/health"));
        assert!(paths.contains_key("/data.json"));
        assert!(paths.contains_key("/styles.json"));
        assert!(paths.contains_key("/fonts.json"));
    }

    #[test]
    fn test_openapi_spec_has_all_endpoints() {
        let spec = ApiDoc::openapi();
        let paths = spec.paths.paths;

        // All expected endpoints
        let expected_paths = [
            "/health",
            "/index.json",
            "/data.json",
            "/data/{source}",
            "/data/{source}/{z}/{x}/{y}.{format}",
            "/styles.json",
            "/styles/{style}.json",
            "/styles/{style}/style.json",
            "/styles/{style}/{z}/{x}/{y}.{format}",
            "/styles/{style}/{tileSize}/{z}/{x}/{y}.{format}",
            "/styles/{style}/static/{center}/{size}.{format}",
            "/styles/{style}/sprite.{ext}",
            "/styles/{style}/wmts.xml",
            "/fonts.json",
            "/fonts/{fontstack}/{range}",
            "/files/{filepath}",
        ];

        for path in expected_paths {
            assert!(
                paths.contains_key(path),
                "Missing path in OpenAPI spec: {}",
                path
            );
        }
    }

    #[test]
    fn test_openapi_has_tags() {
        let spec = ApiDoc::openapi();
        // Verify tags are present
        assert!(spec.tags.is_some(), "Tags should be defined");
        assert_eq!(
            spec.tags.as_ref().unwrap().len(),
            5,
            "Should have 5 tags defined"
        );
    }

    #[test]
    fn test_openapi_has_schemas() {
        let spec = ApiDoc::openapi();
        let schemas = &spec.components.as_ref().unwrap().schemas;

        assert!(schemas.contains_key("TileJSON"));
        assert!(schemas.contains_key("StyleInfo"));
        assert!(schemas.contains_key("VectorLayer"));
        assert!(schemas.contains_key("GeoJSON"));
        assert!(schemas.contains_key("ApiError"));
    }
}
