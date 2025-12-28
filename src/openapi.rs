//! OpenAPI 3.1 specification for tileserver-rs API
//!
//! This module provides the OpenAPI specification as a static JSON document.

use serde_json::{json, Value};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_openapi_spec() {
        let spec = generate_openapi_spec("http://localhost:8080", "0.2.1");

        // Check basic structure
        assert_eq!(spec["openapi"], "3.1.0");
        assert_eq!(spec["info"]["title"], "tileserver-rs API");
        assert_eq!(spec["info"]["version"], "0.2.1");

        // Check server URL
        assert_eq!(spec["servers"][0]["url"], "http://localhost:8080");

        // Check that paths exist
        assert!(spec["paths"]["/health"].is_object());
        assert!(spec["paths"]["/data.json"].is_object());
        assert!(spec["paths"]["/styles.json"].is_object());
        assert!(spec["paths"]["/fonts.json"].is_object());

        // Check components
        assert!(spec["components"]["schemas"]["TileJSON"].is_object());
        assert!(spec["components"]["schemas"]["StyleInfo"].is_object());
    }

    #[test]
    fn test_openapi_spec_has_all_endpoints() {
        let spec = generate_openapi_spec("http://localhost:8080", "0.2.1");
        let paths = spec["paths"].as_object().unwrap();

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
}

/// Generate the OpenAPI specification
pub fn generate_openapi_spec(base_url: &str, version: &str) -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "tileserver-rs API",
            "description": "High-performance vector and raster tile server with native MapLibre rendering",
            "version": version,
            "license": {
                "name": "MIT",
                "url": "https://github.com/vinayakkulkarni/tileserver-rs/blob/main/LICENSE"
            },
            "contact": {
                "name": "Vinayak Kulkarni",
                "url": "https://github.com/vinayakkulkarni/tileserver-rs"
            }
        },
        "servers": [
            {
                "url": base_url,
                "description": "Current server"
            }
        ],
        "tags": [
            {
                "name": "Health",
                "description": "Health check endpoints"
            },
            {
                "name": "Data",
                "description": "Vector tile data sources (PMTiles, MBTiles)"
            },
            {
                "name": "Styles",
                "description": "Map styles and raster tile rendering"
            },
            {
                "name": "Fonts",
                "description": "Font glyphs for map labels"
            },
            {
                "name": "Files",
                "description": "Static file serving"
            }
        ],
        "paths": {
            "/health": {
                "get": {
                    "tags": ["Health"],
                    "summary": "Health check",
                    "description": "Returns OK if the server is running",
                    "operationId": "healthCheck",
                    "responses": {
                        "200": {
                            "description": "Server is healthy",
                            "content": {
                                "text/plain": {
                                    "schema": {
                                        "type": "string"
                                    },
                                    "examples": {
                                        "ok": {
                                            "value": "OK"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/index.json": {
                "get": {
                    "tags": ["Data", "Styles"],
                    "summary": "Get all sources and styles",
                    "description": "Returns a combined list of all data sources and styles as TileJSON",
                    "operationId": "getIndex",
                    "responses": {
                        "200": {
                            "description": "Combined list of sources and styles",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "$ref": "#/components/schemas/TileJSON"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/data.json": {
                "get": {
                    "tags": ["Data"],
                    "summary": "List all data sources",
                    "description": "Returns TileJSON metadata for all available tile sources",
                    "operationId": "listDataSources",
                    "responses": {
                        "200": {
                            "description": "List of data sources",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "$ref": "#/components/schemas/TileJSON"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/data/{source}": {
                "get": {
                    "tags": ["Data"],
                    "summary": "Get data source TileJSON",
                    "description": "Returns TileJSON metadata for a specific tile source",
                    "operationId": "getDataSource",
                    "parameters": [
                        {
                            "name": "source",
                            "in": "path",
                            "required": true,
                            "description": "Source ID (with or without .json extension)",
                            "schema": {
                                "type": "string"
                            },
                            "examples": {
                                "withExtension": {
                                    "value": "openmaptiles.json",
                                    "summary": "With .json extension"
                                },
                                "withoutExtension": {
                                    "value": "openmaptiles",
                                    "summary": "Without extension"
                                }
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "TileJSON metadata",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/TileJSON"
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "Source not found",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/Error"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/data/{source}/{z}/{x}/{y}.{format}": {
                "get": {
                    "tags": ["Data"],
                    "summary": "Get a vector tile",
                    "description": "Returns a vector tile in the requested format (pbf, mvt, or geojson)",
                    "operationId": "getTile",
                    "parameters": [
                        {
                            "name": "source",
                            "in": "path",
                            "required": true,
                            "description": "Source ID",
                            "schema": {
                                "type": "string"
                            }
                        },
                        {
                            "name": "z",
                            "in": "path",
                            "required": true,
                            "description": "Zoom level (0-22)",
                            "schema": {
                                "type": "integer",
                                "minimum": 0,
                                "maximum": 22
                            }
                        },
                        {
                            "name": "x",
                            "in": "path",
                            "required": true,
                            "description": "Tile X coordinate",
                            "schema": {
                                "type": "integer",
                                "minimum": 0
                            }
                        },
                        {
                            "name": "y",
                            "in": "path",
                            "required": true,
                            "description": "Tile Y coordinate",
                            "schema": {
                                "type": "integer",
                                "minimum": 0
                            }
                        },
                        {
                            "name": "format",
                            "in": "path",
                            "required": true,
                            "description": "Tile format",
                            "schema": {
                                "type": "string",
                                "enum": ["pbf", "mvt", "geojson"]
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Vector tile data",
                            "headers": {
                                "Content-Encoding": {
                                    "description": "Compression encoding (gzip if compressed)",
                                    "schema": {
                                        "type": "string"
                                    }
                                }
                            },
                            "content": {
                                "application/x-protobuf": {
                                    "schema": {
                                        "type": "string",
                                        "format": "binary"
                                    }
                                },
                                "application/geo+json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/GeoJSON"
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "Tile not found"
                        }
                    }
                }
            },
            "/styles.json": {
                "get": {
                    "tags": ["Styles"],
                    "summary": "List all styles",
                    "description": "Returns metadata for all available map styles",
                    "operationId": "listStyles",
                    "responses": {
                        "200": {
                            "description": "List of styles",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "$ref": "#/components/schemas/StyleInfo"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/styles/{style}.json": {
                "get": {
                    "tags": ["Styles"],
                    "summary": "Get style TileJSON",
                    "description": "Returns TileJSON for raster tiles rendered from this style",
                    "operationId": "getStyleTileJSON",
                    "parameters": [
                        {
                            "name": "style",
                            "in": "path",
                            "required": true,
                            "description": "Style ID",
                            "schema": {
                                "type": "string"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "TileJSON for raster tiles",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/TileJSON"
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "Style not found"
                        }
                    }
                }
            },
            "/styles/{style}/style.json": {
                "get": {
                    "tags": ["Styles"],
                    "summary": "Get MapLibre style JSON",
                    "description": "Returns the full MapLibre GL style specification",
                    "operationId": "getStyleJSON",
                    "parameters": [
                        {
                            "name": "style",
                            "in": "path",
                            "required": true,
                            "description": "Style ID",
                            "schema": {
                                "type": "string"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "MapLibre style specification",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "description": "MapLibre GL Style Specification"
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "Style not found"
                        }
                    }
                }
            },
            "/styles/{style}/{z}/{x}/{y}.{format}": {
                "get": {
                    "tags": ["Styles"],
                    "summary": "Get a raster tile",
                    "description": "Returns a raster tile rendered from the style",
                    "operationId": "getRasterTile",
                    "parameters": [
                        {
                            "name": "style",
                            "in": "path",
                            "required": true,
                            "description": "Style ID",
                            "schema": {
                                "type": "string"
                            }
                        },
                        {
                            "name": "z",
                            "in": "path",
                            "required": true,
                            "description": "Zoom level",
                            "schema": {
                                "type": "integer",
                                "minimum": 0,
                                "maximum": 22
                            }
                        },
                        {
                            "name": "x",
                            "in": "path",
                            "required": true,
                            "description": "Tile X coordinate",
                            "schema": {
                                "type": "integer"
                            }
                        },
                        {
                            "name": "y",
                            "in": "path",
                            "required": true,
                            "description": "Tile Y coordinate (can include @2x for retina)",
                            "schema": {
                                "type": "string"
                            },
                            "examples": {
                                "standard": {
                                    "value": "123",
                                    "summary": "Standard resolution"
                                },
                                "retina": {
                                    "value": "123@2x",
                                    "summary": "2x retina resolution"
                                }
                            }
                        },
                        {
                            "name": "format",
                            "in": "path",
                            "required": true,
                            "description": "Image format",
                            "schema": {
                                "type": "string",
                                "enum": ["png", "jpg", "jpeg", "webp"]
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Raster tile image",
                            "content": {
                                "image/png": {
                                    "schema": {
                                        "type": "string",
                                        "format": "binary"
                                    }
                                },
                                "image/jpeg": {
                                    "schema": {
                                        "type": "string",
                                        "format": "binary"
                                    }
                                },
                                "image/webp": {
                                    "schema": {
                                        "type": "string",
                                        "format": "binary"
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "Style not found"
                        }
                    }
                }
            },
            "/styles/{style}/{tileSize}/{z}/{x}/{y}.{format}": {
                "get": {
                    "tags": ["Styles"],
                    "summary": "Get a raster tile with custom size",
                    "description": "Returns a raster tile with specified tile size (256 or 512)",
                    "operationId": "getRasterTileWithSize",
                    "parameters": [
                        {
                            "name": "style",
                            "in": "path",
                            "required": true,
                            "schema": {
                                "type": "string"
                            }
                        },
                        {
                            "name": "tileSize",
                            "in": "path",
                            "required": true,
                            "description": "Tile size in pixels",
                            "schema": {
                                "type": "integer",
                                "enum": [256, 512]
                            }
                        },
                        {
                            "name": "z",
                            "in": "path",
                            "required": true,
                            "schema": {
                                "type": "integer"
                            }
                        },
                        {
                            "name": "x",
                            "in": "path",
                            "required": true,
                            "schema": {
                                "type": "integer"
                            }
                        },
                        {
                            "name": "y",
                            "in": "path",
                            "required": true,
                            "schema": {
                                "type": "string"
                            }
                        },
                        {
                            "name": "format",
                            "in": "path",
                            "required": true,
                            "schema": {
                                "type": "string",
                                "enum": ["png", "jpg", "jpeg", "webp"]
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Raster tile image",
                            "content": {
                                "image/png": {},
                                "image/jpeg": {},
                                "image/webp": {}
                            }
                        }
                    }
                }
            },
            "/styles/{style}/static/{center}/{size}.{format}": {
                "get": {
                    "tags": ["Styles"],
                    "summary": "Get a static map image",
                    "description": "Renders a static map image centered at the specified location",
                    "operationId": "getStaticImage",
                    "parameters": [
                        {
                            "name": "style",
                            "in": "path",
                            "required": true,
                            "schema": {
                                "type": "string"
                            }
                        },
                        {
                            "name": "center",
                            "in": "path",
                            "required": true,
                            "description": "Center point as lon,lat,zoom or auto",
                            "schema": {
                                "type": "string"
                            },
                            "examples": {
                                "coordinates": {
                                    "value": "-122.4194,37.7749,12",
                                    "summary": "San Francisco at zoom 12"
                                },
                                "auto": {
                                    "value": "auto",
                                    "summary": "Auto-fit to markers"
                                }
                            }
                        },
                        {
                            "name": "size",
                            "in": "path",
                            "required": true,
                            "description": "Image size as WIDTHxHEIGHT, optionally with @2x for retina",
                            "schema": {
                                "type": "string"
                            },
                            "examples": {
                                "standard": {
                                    "value": "800x600",
                                    "summary": "800x600 pixels"
                                },
                                "retina": {
                                    "value": "800x600@2x",
                                    "summary": "800x600 at 2x resolution"
                                }
                            }
                        },
                        {
                            "name": "format",
                            "in": "path",
                            "required": true,
                            "schema": {
                                "type": "string",
                                "enum": ["png", "jpg", "jpeg", "webp"]
                            }
                        },
                        {
                            "name": "bearing",
                            "in": "query",
                            "description": "Map bearing in degrees",
                            "schema": {
                                "type": "number",
                                "default": 0
                            }
                        },
                        {
                            "name": "pitch",
                            "in": "query",
                            "description": "Map pitch in degrees",
                            "schema": {
                                "type": "number",
                                "default": 0
                            }
                        },
                        {
                            "name": "markers",
                            "in": "query",
                            "description": "Markers to add (format: lon,lat,color|lon,lat,color)",
                            "schema": {
                                "type": "string"
                            }
                        },
                        {
                            "name": "path",
                            "in": "query",
                            "description": "Path to draw (format: stroke:color|width|lon,lat|lon,lat...)",
                            "schema": {
                                "type": "string"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Static map image",
                            "content": {
                                "image/png": {},
                                "image/jpeg": {},
                                "image/webp": {}
                            }
                        }
                    }
                }
            },
            "/styles/{style}/sprite.{ext}": {
                "get": {
                    "tags": ["Styles"],
                    "summary": "Get sprite image or JSON",
                    "description": "Returns sprite image (PNG) or metadata (JSON) for the style",
                    "operationId": "getSprite",
                    "parameters": [
                        {
                            "name": "style",
                            "in": "path",
                            "required": true,
                            "schema": {
                                "type": "string"
                            }
                        },
                        {
                            "name": "ext",
                            "in": "path",
                            "required": true,
                            "description": "File extension (png or json, optionally with @2x)",
                            "schema": {
                                "type": "string"
                            },
                            "examples": {
                                "image": {
                                    "value": "png"
                                },
                                "json": {
                                    "value": "json"
                                },
                                "retina": {
                                    "value": "@2x.png"
                                }
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Sprite data",
                            "content": {
                                "image/png": {},
                                "application/json": {}
                            }
                        },
                        "404": {
                            "description": "Sprite not found"
                        }
                    }
                }
            },
            "/styles/{style}/wmts.xml": {
                "get": {
                    "tags": ["Styles"],
                    "summary": "Get WMTS capabilities",
                    "description": "Returns OGC WMTS GetCapabilities document for the style",
                    "operationId": "getWMTSCapabilities",
                    "parameters": [
                        {
                            "name": "style",
                            "in": "path",
                            "required": true,
                            "schema": {
                                "type": "string"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "WMTS capabilities XML",
                            "content": {
                                "application/xml": {
                                    "schema": {
                                        "type": "string"
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/fonts.json": {
                "get": {
                    "tags": ["Fonts"],
                    "summary": "List available fonts",
                    "description": "Returns a list of available font families",
                    "operationId": "listFonts",
                    "responses": {
                        "200": {
                            "description": "List of font names",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "type": "string"
                                        }
                                    },
                                    "examples": {
                                        "fonts": {
                                            "value": ["Noto Sans Regular", "Noto Sans Bold"]
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "/fonts/{fontstack}/{range}": {
                "get": {
                    "tags": ["Fonts"],
                    "summary": "Get font glyphs",
                    "description": "Returns PBF-encoded font glyphs for a character range",
                    "operationId": "getFontGlyphs",
                    "parameters": [
                        {
                            "name": "fontstack",
                            "in": "path",
                            "required": true,
                            "description": "Font stack (comma-separated font names)",
                            "schema": {
                                "type": "string"
                            },
                            "examples": {
                                "single": {
                                    "value": "Noto Sans Regular",
                                    "summary": "Single font"
                                },
                                "fallback": {
                                    "value": "Noto Sans Bold,Noto Sans Regular",
                                    "summary": "Font with fallback"
                                }
                            }
                        },
                        {
                            "name": "range",
                            "in": "path",
                            "required": true,
                            "description": "Character range (e.g., 0-255.pbf)",
                            "schema": {
                                "type": "string",
                                "pattern": "^\\d+-\\d+\\.pbf$"
                            },
                            "examples": {
                                "basic": {
                                    "value": "0-255.pbf",
                                    "summary": "Basic Latin characters"
                                },
                                "extended": {
                                    "value": "256-511.pbf",
                                    "summary": "Latin Extended"
                                }
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Font glyph data",
                            "content": {
                                "application/x-protobuf": {
                                    "schema": {
                                        "type": "string",
                                        "format": "binary"
                                    }
                                }
                            }
                        },
                        "404": {
                            "description": "Font not found"
                        }
                    }
                }
            },
            "/files/{filepath}": {
                "get": {
                    "tags": ["Files"],
                    "summary": "Get static file",
                    "description": "Serves static files from the configured files directory",
                    "operationId": "getStaticFile",
                    "parameters": [
                        {
                            "name": "filepath",
                            "in": "path",
                            "required": true,
                            "description": "Path to the file",
                            "schema": {
                                "type": "string"
                            }
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "File content"
                        },
                        "404": {
                            "description": "File not found"
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "TileJSON": {
                    "type": "object",
                    "required": ["tilejson", "tiles"],
                    "properties": {
                        "tilejson": {
                            "type": "string",
                            "description": "TileJSON version",
                            "example": "3.0.0"
                        },
                        "id": {
                            "type": "string",
                            "description": "Source identifier"
                        },
                        "name": {
                            "type": "string",
                            "description": "Human-readable name"
                        },
                        "description": {
                            "type": "string",
                            "description": "Description of the tileset"
                        },
                        "tiles": {
                            "type": "array",
                            "items": {
                                "type": "string"
                            },
                            "description": "Tile URL templates"
                        },
                        "minzoom": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 22
                        },
                        "maxzoom": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 22
                        },
                        "bounds": {
                            "type": "array",
                            "items": {
                                "type": "number"
                            },
                            "minItems": 4,
                            "maxItems": 4,
                            "description": "[west, south, east, north]"
                        },
                        "center": {
                            "type": "array",
                            "items": {
                                "type": "number"
                            },
                            "minItems": 3,
                            "maxItems": 3,
                            "description": "[longitude, latitude, zoom]"
                        },
                        "attribution": {
                            "type": "string",
                            "description": "Attribution HTML"
                        },
                        "vector_layers": {
                            "type": "array",
                            "items": {
                                "$ref": "#/components/schemas/VectorLayer"
                            }
                        }
                    }
                },
                "VectorLayer": {
                    "type": "object",
                    "required": ["id"],
                    "properties": {
                        "id": {
                            "type": "string"
                        },
                        "description": {
                            "type": "string"
                        },
                        "minzoom": {
                            "type": "integer"
                        },
                        "maxzoom": {
                            "type": "integer"
                        },
                        "fields": {
                            "type": "object",
                            "additionalProperties": {
                                "type": "string"
                            }
                        }
                    }
                },
                "StyleInfo": {
                    "type": "object",
                    "required": ["id", "name", "url"],
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Style identifier"
                        },
                        "name": {
                            "type": "string",
                            "description": "Human-readable name"
                        },
                        "url": {
                            "type": "string",
                            "description": "URL to style.json"
                        }
                    }
                },
                "GeoJSON": {
                    "type": "object",
                    "required": ["type"],
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": ["FeatureCollection"]
                        },
                        "features": {
                            "type": "array",
                            "items": {
                                "type": "object"
                            }
                        }
                    }
                },
                "Error": {
                    "type": "object",
                    "required": ["error"],
                    "properties": {
                        "error": {
                            "type": "string",
                            "description": "Error message"
                        }
                    }
                }
            }
        }
    })
}
