use serde::Deserialize;
use std::str::FromStr;

/// Maximum allowed image dimension (width or height) in pixels
pub const MAX_IMAGE_DIMENSION: u32 = 4096;

/// Maximum allowed scale factor for retina images
pub const MAX_SCALE_FACTOR: u8 = 4;

/// Image format for rendered output
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
}

impl ImageFormat {
    #[must_use]
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
        }
    }
}

impl FromStr for ImageFormat {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "png" => Ok(Self::Png),
            "jpg" | "jpeg" => Ok(Self::Jpeg),
            "webp" => Ok(Self::Webp),
            _ => Err(()),
        }
    }
}

/// Static image type (center, bbox, or auto)
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum StaticType {
    /// Center-based: lon,lat,zoom[@bearing[,pitch]]
    Center {
        lon: f64,
        lat: f64,
        zoom: f64,
        bearing: Option<f64>,
        pitch: Option<f64>,
    },
    /// Bounding box: minx,miny,maxx,maxy
    BoundingBox {
        min_lon: f64,
        min_lat: f64,
        max_lon: f64,
        max_lat: f64,
    },
    /// Auto-fit based on paths/markers
    Auto,
}

impl FromStr for StaticType {
    type Err = String;

    /// Parse static type from path parameter
    /// Examples:
    /// - "-122.4,37.8,12" -> Center
    /// - "-122.4,37.8,12@45" -> Center with bearing
    /// - "-122.4,37.8,12@45,60" -> Center with bearing and pitch
    /// - "-123,37,-122,38" -> BoundingBox
    /// - "auto" -> Auto
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "auto" {
            return Ok(Self::Auto);
        }

        let parts: Vec<&str> = s.split(',').collect();

        // Bounding box: 4 coordinates
        if parts.len() == 4 {
            let min_lon = parts[0].parse().map_err(|_| "Invalid min longitude")?;
            let min_lat = parts[1].parse().map_err(|_| "Invalid min latitude")?;
            let max_lon = parts[2].parse().map_err(|_| "Invalid max longitude")?;
            let max_lat = parts[3].parse().map_err(|_| "Invalid max latitude")?;

            return Ok(Self::BoundingBox {
                min_lon,
                min_lat,
                max_lon,
                max_lat,
            });
        }

        // Center-based: 3+ parts (lon,lat,zoom[@bearing[,pitch]])
        if parts.len() >= 3 {
            let lon = parts[0].parse().map_err(|_| "Invalid longitude")?;
            let lat = parts[1].parse().map_err(|_| "Invalid latitude")?;

            // Check if zoom contains bearing (e.g., "12@45" or "12@45,60")
            let zoom_parts: Vec<&str> = parts[2].split('@').collect();
            let zoom = zoom_parts[0].parse().map_err(|_| "Invalid zoom")?;

            let (bearing, pitch) = if zoom_parts.len() > 1 {
                // Parse bearing and optional pitch from "@45,60"
                let bearing_pitch: Vec<&str> = zoom_parts[1].split(',').collect();
                let bearing = Some(bearing_pitch[0].parse().map_err(|_| "Invalid bearing")?);
                let pitch = if bearing_pitch.len() > 1 {
                    Some(bearing_pitch[1].parse().map_err(|_| "Invalid pitch")?)
                } else {
                    None
                };
                (bearing, pitch)
            } else {
                (None, None)
            };

            return Ok(Self::Center {
                lon,
                lat,
                zoom,
                bearing,
                pitch,
            });
        }

        Err(format!("Invalid static type format: {}", s))
    }
}

/// Query parameters for static image rendering
#[derive(Debug, Clone, Default, Deserialize)]
pub struct StaticQueryParams {
    /// Path overlay (encoded)
    pub path: Option<String>,
    /// Marker overlay (encoded)
    pub marker: Option<String>,
    /// GeoJSON geometry, Feature, or FeatureCollection as URL-encoded JSON string
    pub geojson: Option<String>,
    /// Parse coordinates as lat/lng instead of lng/lat (Google Maps convention)
    #[serde(default)]
    pub latlng: bool,
    /// Padding fraction for auto-fit bounding box (default 0.1)
    pub padding: Option<f64>,
    /// Maximum zoom level for auto-fit
    pub maxzoom: Option<u8>,
}

/// Options for rendering a map image
#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub style_id: String,
    pub style_json: String,
    pub width: u32,
    pub height: u32,
    /// Pixel ratio / scale (1-9)
    pub scale: u8,
    pub lon: f64,
    pub lat: f64,
    pub zoom: f64,
    /// Camera bearing (rotation) in degrees
    pub bearing: f64,
    /// Camera pitch (tilt) in degrees
    pub pitch: f64,
    pub format: ImageFormat,
    pub path: Option<String>,
    pub marker: Option<String>,
    pub geojson: Option<String>,
}

impl RenderOptions {
    /// Create options for a raster tile
    #[must_use]
    pub fn for_tile(
        style_id: String,
        style_json: String,
        z: u8,
        x: u32,
        y: u32,
        scale: u8,
        format: ImageFormat,
    ) -> Self {
        // Calculate center of tile
        let n = 2_f64.powi(z as i32);
        let lon = (x as f64) / n * 360.0 - 180.0;
        let lat_rad = ((1.0 - 2.0 * (y as f64) / n) * std::f64::consts::PI)
            .sinh()
            .atan();
        let lat = lat_rad.to_degrees();

        // Tile size is 512px at scale 1
        let tile_size = 512;

        Self {
            style_id,
            style_json,
            width: tile_size,
            height: tile_size,
            scale,
            lon,
            lat,
            zoom: z as f64,
            bearing: 0.0,
            pitch: 0.0,
            format,
            path: None,
            marker: None,
            geojson: None,
        }
    }

    /// Create options for a static image
    #[allow(clippy::too_many_arguments)]
    pub fn for_static(
        style_id: String,
        style_json: String,
        static_type: StaticType,
        width: u32,
        height: u32,
        scale: u8,
        format: ImageFormat,
        query_params: StaticQueryParams,
    ) -> Result<Self, String> {
        // Security: Validate image dimensions to prevent DoS via memory exhaustion
        if width == 0 || height == 0 {
            return Err("Image dimensions must be greater than 0".to_string());
        }
        if width > MAX_IMAGE_DIMENSION {
            return Err(format!(
                "Image width {} exceeds maximum of {}",
                width, MAX_IMAGE_DIMENSION
            ));
        }
        if height > MAX_IMAGE_DIMENSION {
            return Err(format!(
                "Image height {} exceeds maximum of {}",
                height, MAX_IMAGE_DIMENSION
            ));
        }
        if scale == 0 || scale > MAX_SCALE_FACTOR {
            return Err(format!(
                "Scale factor must be between 1 and {}",
                MAX_SCALE_FACTOR
            ));
        }

        let (lon, lat, zoom, bearing, pitch) = match static_type {
            StaticType::Center {
                lon,
                lat,
                zoom,
                bearing,
                pitch,
            } => (lon, lat, zoom, bearing.unwrap_or(0.0), pitch.unwrap_or(0.0)),
            StaticType::BoundingBox {
                min_lon,
                min_lat,
                max_lon,
                max_lat,
            } => {
                // Calculate center and zoom to fit bbox
                let center_lon = (min_lon + max_lon) / 2.0;
                let center_lat = (min_lat + max_lat) / 2.0;

                // Simple zoom calculation (can be improved)
                let lon_diff = (max_lon - min_lon).abs();
                let lat_diff = (max_lat - min_lat).abs();
                let max_diff = lon_diff.max(lat_diff);

                let zoom = if max_diff > 180.0 {
                    0.0
                } else if max_diff > 90.0 {
                    1.0
                } else if max_diff > 45.0 {
                    2.0
                } else if max_diff > 22.5 {
                    3.0
                } else if max_diff > 11.25 {
                    4.0
                } else if max_diff > 5.625 {
                    5.0
                } else {
                    // More precise calculation for higher zooms
                    let zoom_lon = (360.0 / lon_diff).log2();
                    let zoom_lat = (180.0 / lat_diff).log2();
                    zoom_lon.min(zoom_lat).floor()
                };

                (center_lon, center_lat, zoom, 0.0, 0.0)
            }
            StaticType::Auto => {
                let mut paths = Vec::new();
                let mut markers = Vec::new();

                if let Some(ref path_str) = query_params.path {
                    for path_part in path_str.split('~') {
                        if let Some(path) =
                            crate::render::overlay::parse_path(path_part, query_params.latlng)
                        {
                            paths.push(path);
                        }
                    }
                }

                if let Some(ref marker_str) = query_params.marker {
                    for marker_part in marker_str.split('~') {
                        if let Some(marker) = crate::render::overlay::parse_marker(marker_part) {
                            markers.push(marker);
                        }
                    }
                }

                if let Some(ref geojson_str) = query_params.geojson {
                    let (geojson_paths, geojson_markers) =
                        crate::render::overlay::parse_geojson(geojson_str);
                    paths.extend(geojson_paths);
                    markers.extend(geojson_markers);
                }

                if let Some((min_lon, min_lat, max_lon, max_lat)) =
                    crate::render::overlay::calculate_bounds(&paths, &markers)
                {
                    // Calculate center
                    let center_lon = (min_lon + max_lon) / 2.0;
                    let center_lat = (min_lat + max_lat) / 2.0;

                    // Calculate zoom to fit bounds with padding
                    let padding = query_params.padding.unwrap_or(0.1);
                    let lon_diff = (max_lon - min_lon).abs() * (1.0 + padding);
                    let lat_diff = (max_lat - min_lat).abs() * (1.0 + padding);

                    // Account for image aspect ratio
                    let aspect = width as f64 / height as f64;
                    let adjusted_lon_diff = lon_diff.max(lat_diff * aspect);
                    let adjusted_lat_diff = lat_diff.max(lon_diff / aspect);
                    let max_diff = adjusted_lon_diff.max(adjusted_lat_diff);

                    let zoom = if max_diff > 180.0 {
                        0.0
                    } else if max_diff > 0.0 {
                        let zoom_lon = (360.0 / max_diff).log2();
                        let zoom_lat = (180.0 / adjusted_lat_diff).log2();
                        let calculated_zoom = zoom_lon.min(zoom_lat).floor();
                        // Clamp to maxzoom if specified
                        if let Some(max_zoom) = query_params.maxzoom {
                            calculated_zoom.min(max_zoom as f64)
                        } else {
                            calculated_zoom.min(18.0)
                        }
                    } else {
                        // Single point, use a reasonable default zoom
                        query_params.maxzoom.map_or(14.0, |z| z as f64)
                    };

                    (center_lon, center_lat, zoom, 0.0, 0.0)
                } else {
                    // No paths or markers, default to world view
                    (0.0, 0.0, 1.0, 0.0, 0.0)
                }
            }
        };

        Ok(Self {
            style_id,
            style_json,
            width,
            height,
            scale,
            lon,
            lat,
            zoom,
            bearing,
            pitch,
            format,
            path: query_params.path,
            marker: query_params.marker,
            geojson: query_params.geojson,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_image_format_from_str_png() {
        assert_eq!(ImageFormat::from_str("png"), Ok(ImageFormat::Png));
    }

    #[test]
    fn test_image_format_from_str_jpg() {
        assert_eq!(ImageFormat::from_str("jpg"), Ok(ImageFormat::Jpeg));
    }

    #[test]
    fn test_image_format_from_str_jpeg() {
        assert_eq!(ImageFormat::from_str("jpeg"), Ok(ImageFormat::Jpeg));
    }

    #[test]
    fn test_image_format_from_str_webp() {
        assert_eq!(ImageFormat::from_str("webp"), Ok(ImageFormat::Webp));
    }

    #[test]
    fn test_image_format_from_str_case_insensitive() {
        assert_eq!(ImageFormat::from_str("PNG"), Ok(ImageFormat::Png));
        assert_eq!(ImageFormat::from_str("WEBP"), Ok(ImageFormat::Webp));
    }

    #[test]
    fn test_image_format_from_str_invalid() {
        assert!(ImageFormat::from_str("bmp").is_err());
        assert!(ImageFormat::from_str("gif").is_err());
    }

    #[test]
    fn test_image_format_content_type_png() {
        assert_eq!(ImageFormat::Png.content_type(), "image/png");
    }

    #[test]
    fn test_image_format_content_type_jpeg() {
        assert_eq!(ImageFormat::Jpeg.content_type(), "image/jpeg");
    }

    #[test]
    fn test_image_format_content_type_webp() {
        assert_eq!(ImageFormat::Webp.content_type(), "image/webp");
    }

    #[test]
    fn test_static_type_auto() {
        let st = StaticType::from_str("auto").unwrap();
        assert!(matches!(st, StaticType::Auto));
    }

    #[test]
    fn test_static_type_center() {
        let st = StaticType::from_str("-122.4,37.8,12").unwrap();
        match st {
            StaticType::Center {
                lon,
                lat,
                zoom,
                bearing,
                pitch,
            } => {
                assert!((lon - (-122.4)).abs() < 0.001);
                assert!((lat - 37.8).abs() < 0.001);
                assert!((zoom - 12.0).abs() < 0.001);
                assert!(bearing.is_none());
                assert!(pitch.is_none());
            }
            _ => panic!("expected Center"),
        }
    }

    #[test]
    fn test_static_type_center_with_bearing() {
        let st = StaticType::from_str("-122.4,37.8,12@45").unwrap();
        match st {
            StaticType::Center { bearing, pitch, .. } => {
                assert_eq!(bearing, Some(45.0));
                assert!(pitch.is_none());
            }
            _ => panic!("expected Center"),
        }
    }

    #[test]
    fn test_static_type_center_bearing_pitch_ambiguous_with_bbox() {
        // "-122.4,37.8,12@45,60" has 4 comma-separated parts.
        // The parser's 4-part bbox branch runs first and cannot parse
        // "12@45" as f64, so this input returns an error.
        // This documents the current parser limitation.
        let result = StaticType::from_str("-122.4,37.8,12@45,60");
        assert!(result.is_err());
    }

    #[test]
    fn test_static_type_bounding_box() {
        let st = StaticType::from_str("-123,37,-122,38").unwrap();
        match st {
            StaticType::BoundingBox {
                min_lon,
                min_lat,
                max_lon,
                max_lat,
            } => {
                assert!((min_lon - (-123.0)).abs() < 0.001);
                assert!((min_lat - 37.0).abs() < 0.001);
                assert!((max_lon - (-122.0)).abs() < 0.001);
                assert!((max_lat - 38.0).abs() < 0.001);
            }
            _ => panic!("expected BoundingBox"),
        }
    }

    #[test]
    fn test_static_type_invalid() {
        let result = StaticType::from_str("garbage");
        assert!(result.is_err());
    }

    #[test]
    fn test_render_options_for_tile_dimensions() {
        let opts = RenderOptions::for_tile(
            "style1".to_string(),
            "{}".to_string(),
            0,
            0,
            0,
            1,
            ImageFormat::Png,
        );
        assert_eq!(opts.width, 512);
        assert_eq!(opts.height, 512);
        assert_eq!(opts.scale, 1);
        assert_eq!(opts.bearing, 0.0);
        assert_eq!(opts.pitch, 0.0);
        assert!(matches!(opts.format, ImageFormat::Png));
    }

    #[test]
    fn test_render_options_for_tile_z0_center() {
        let opts = RenderOptions::for_tile(
            "s".to_string(),
            "{}".to_string(),
            0,
            0,
            0,
            1,
            ImageFormat::Png,
        );
        assert!((opts.lon - (-180.0)).abs() < 0.01);
    }

    #[test]
    fn test_render_options_for_static_zero_dimension_rejected() {
        let result = RenderOptions::for_static(
            "s".to_string(),
            "{}".to_string(),
            StaticType::Center {
                lon: 0.0,
                lat: 0.0,
                zoom: 1.0,
                bearing: None,
                pitch: None,
            },
            0,
            100,
            1,
            ImageFormat::Png,
            StaticQueryParams::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_render_options_for_static_oversized_rejected() {
        let result = RenderOptions::for_static(
            "s".to_string(),
            "{}".to_string(),
            StaticType::Center {
                lon: 0.0,
                lat: 0.0,
                zoom: 1.0,
                bearing: None,
                pitch: None,
            },
            5000,
            100,
            1,
            ImageFormat::Png,
            StaticQueryParams::default(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("maximum"));
    }

    #[test]
    fn test_render_options_for_static_scale_zero_rejected() {
        let result = RenderOptions::for_static(
            "s".to_string(),
            "{}".to_string(),
            StaticType::Center {
                lon: 0.0,
                lat: 0.0,
                zoom: 1.0,
                bearing: None,
                pitch: None,
            },
            100,
            100,
            0,
            ImageFormat::Png,
            StaticQueryParams::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_render_options_for_static_scale_too_high_rejected() {
        let result = RenderOptions::for_static(
            "s".to_string(),
            "{}".to_string(),
            StaticType::Center {
                lon: 0.0,
                lat: 0.0,
                zoom: 1.0,
                bearing: None,
                pitch: None,
            },
            100,
            100,
            5,
            ImageFormat::Png,
            StaticQueryParams::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_render_options_for_static_valid() {
        let result = RenderOptions::for_static(
            "s".to_string(),
            "{}".to_string(),
            StaticType::Center {
                lon: -122.4,
                lat: 37.8,
                zoom: 10.0,
                bearing: Some(45.0),
                pitch: Some(30.0),
            },
            800,
            600,
            2,
            ImageFormat::Webp,
            StaticQueryParams::default(),
        );
        let opts = result.unwrap();
        assert_eq!(opts.width, 800);
        assert_eq!(opts.height, 600);
        assert_eq!(opts.scale, 2);
        assert!((opts.lon - (-122.4)).abs() < 0.001);
        assert!((opts.lat - 37.8).abs() < 0.001);
        assert!((opts.bearing - 45.0).abs() < 0.001);
        assert!((opts.pitch - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_static_query_params_defaults() {
        let p = StaticQueryParams::default();
        assert!(p.path.is_none());
        assert!(p.marker.is_none());
        assert!(p.geojson.is_none());
        assert!(!p.latlng);
        assert!(p.padding.is_none());
        assert!(p.maxzoom.is_none());
    }

    #[test]
    fn test_max_constants() {
        assert_eq!(MAX_IMAGE_DIMENSION, 4096);
        assert_eq!(MAX_SCALE_FACTOR, 4);
    }

    fn center_origin() -> StaticType {
        StaticType::Center {
            lon: 0.0,
            lat: 0.0,
            zoom: 1.0,
            bearing: None,
            pitch: None,
        }
    }

    fn for_static_center(
        st: StaticType,
        width: u32,
        height: u32,
        scale: u8,
    ) -> Result<RenderOptions, String> {
        RenderOptions::for_static(
            "s".to_string(),
            "{}".to_string(),
            st,
            width,
            height,
            scale,
            ImageFormat::Png,
            StaticQueryParams::default(),
        )
    }

    #[test]
    fn test_render_options_for_static_zero_dimension_message() {
        let err = for_static_center(center_origin(), 0, 100, 1).unwrap_err();
        assert!(err.contains("dimensions"));
    }

    #[test]
    fn test_render_options_for_static_zero_height_rejected() {
        let err = for_static_center(center_origin(), 100, 0, 1).unwrap_err();
        assert!(err.contains("dimensions"));
    }

    #[test]
    fn test_render_options_for_static_width_too_large_message() {
        let err = for_static_center(center_origin(), MAX_IMAGE_DIMENSION + 1, 100, 1).unwrap_err();
        assert!(err.contains("width"));
    }

    #[test]
    fn test_render_options_for_static_height_too_large_message() {
        let err = for_static_center(center_origin(), 100, MAX_IMAGE_DIMENSION + 1, 1).unwrap_err();
        assert!(err.contains("height"));
    }

    #[test]
    fn test_render_options_for_static_scale_zero_message() {
        let err = for_static_center(center_origin(), 100, 100, 0).unwrap_err();
        assert!(err.contains("Scale"));
    }

    #[test]
    fn test_render_options_for_static_scale_too_high_message() {
        let err = for_static_center(center_origin(), 100, 100, MAX_SCALE_FACTOR + 1).unwrap_err();
        assert!(err.contains("Scale"));
    }

    #[test]
    fn test_render_options_for_static_center_defaults_bearing_pitch() {
        let opts = for_static_center(
            StaticType::Center {
                lon: -71.0,
                lat: 42.0,
                zoom: 9.0,
                bearing: None,
                pitch: None,
            },
            256,
            256,
            1,
        )
        .unwrap();
        assert!((opts.lon - (-71.0)).abs() < 1e-9);
        assert!((opts.lat - 42.0).abs() < 1e-9);
        assert!((opts.zoom - 9.0).abs() < 1e-9);
        assert_eq!(opts.bearing, 0.0);
        assert_eq!(opts.pitch, 0.0);
    }

    #[test]
    fn test_render_options_for_static_center_with_bearing_pitch() {
        let opts = for_static_center(
            StaticType::Center {
                lon: 10.0,
                lat: 20.0,
                zoom: 5.0,
                bearing: Some(90.0),
                pitch: Some(45.0),
            },
            256,
            256,
            1,
        )
        .unwrap();
        assert!((opts.bearing - 90.0).abs() < 1e-9);
        assert!((opts.pitch - 45.0).abs() < 1e-9);
    }

    #[test]
    fn test_render_options_for_static_stores_inputs() {
        let opts = RenderOptions::for_static(
            "my-style".to_string(),
            "{\"version\":8}".to_string(),
            center_origin(),
            321,
            123,
            3,
            ImageFormat::Jpeg,
            StaticQueryParams::default(),
        )
        .unwrap();
        assert_eq!(opts.style_id, "my-style");
        assert_eq!(opts.style_json, "{\"version\":8}");
        assert_eq!(opts.width, 321);
        assert_eq!(opts.height, 123);
        assert_eq!(opts.scale, 3);
        assert!(matches!(opts.format, ImageFormat::Jpeg));
    }

    fn bbox_opts(min_lon: f64, min_lat: f64, max_lon: f64, max_lat: f64) -> RenderOptions {
        for_static_center(
            StaticType::BoundingBox {
                min_lon,
                min_lat,
                max_lon,
                max_lat,
            },
            256,
            256,
            1,
        )
        .unwrap()
    }

    #[test]
    fn test_render_options_for_static_bbox_center_midpoints() {
        // lon_diff 4, lat_diff 4 → precise path; assert midpoints regardless.
        let opts = bbox_opts(-2.0, 8.0, 2.0, 12.0);
        assert!((opts.lon - 0.0).abs() < 1e-9);
        assert!((opts.lat - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_render_options_for_static_bbox_zoom_0() {
        // lon_diff = 200 > 180 → zoom 0.0
        let opts = bbox_opts(-100.0, -10.0, 100.0, 10.0);
        assert_eq!(opts.zoom, 0.0);
    }

    #[test]
    fn test_render_options_for_static_bbox_zoom_1() {
        // lon_diff = 100 (>90, <=180) → zoom 1.0
        let opts = bbox_opts(-50.0, -5.0, 50.0, 5.0);
        assert_eq!(opts.zoom, 1.0);
    }

    #[test]
    fn test_render_options_for_static_bbox_zoom_2() {
        // lon_diff = 50 (>45, <=90) → zoom 2.0
        let opts = bbox_opts(-25.0, -5.0, 25.0, 5.0);
        assert_eq!(opts.zoom, 2.0);
    }

    #[test]
    fn test_render_options_for_static_bbox_zoom_3() {
        // lon_diff = 30 (>22.5, <=45) → zoom 3.0
        let opts = bbox_opts(-15.0, -5.0, 15.0, 5.0);
        assert_eq!(opts.zoom, 3.0);
    }

    #[test]
    fn test_render_options_for_static_bbox_zoom_4() {
        // lon_diff = 20 (>11.25, <=22.5) → zoom 4.0
        let opts = bbox_opts(-10.0, -5.0, 10.0, 5.0);
        assert_eq!(opts.zoom, 4.0);
    }

    #[test]
    fn test_render_options_for_static_bbox_zoom_5() {
        // lon_diff = 10 (>5.625, <=11.25) → zoom 5.0
        let opts = bbox_opts(-5.0, -2.0, 5.0, 2.0);
        assert_eq!(opts.zoom, 5.0);
    }

    #[test]
    fn test_render_options_for_static_bbox_zoom_precise_path() {
        // lon_diff = 1, lat_diff = 1 → max_diff <= 5.625 → precise log2 branch.
        // zoom_lon = log2(360/1) ≈ 8.49, zoom_lat = log2(180/1) ≈ 7.49
        // → min().floor() = 7.0
        let opts = bbox_opts(0.0, 0.0, 1.0, 1.0);
        assert_eq!(opts.zoom, 7.0);
        assert!((opts.lon - 0.5).abs() < 1e-9);
        assert!((opts.lat - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_render_options_for_static_auto_empty_world_view() {
        // No path/marker/geojson overlays → calculate_bounds is None → world view.
        let opts = for_static_center(StaticType::Auto, 256, 256, 1).unwrap();
        assert_eq!(opts.lon, 0.0);
        assert_eq!(opts.lat, 0.0);
        assert_eq!(opts.zoom, 1.0);
        assert_eq!(opts.bearing, 0.0);
        assert_eq!(opts.pitch, 0.0);
    }

    fn for_static_auto(params: StaticQueryParams, width: u32, height: u32) -> RenderOptions {
        RenderOptions::for_static(
            "s".to_string(),
            "{}".to_string(),
            StaticType::Auto,
            width,
            height,
            1,
            ImageFormat::Png,
            params,
        )
        .unwrap()
    }

    #[test]
    fn test_render_options_for_static_auto_path_fits_bounds() {
        let params = StaticQueryParams {
            path: Some("-2.0,8.0|2.0,12.0".to_string()),
            ..StaticQueryParams::default()
        };
        let opts = for_static_auto(params, 256, 256);
        assert!((opts.lon - 0.0).abs() < 1e-9);
        assert!((opts.lat - 10.0).abs() < 1e-9);
        assert!(opts.zoom > 0.0 && opts.zoom <= 18.0);
    }

    #[test]
    fn test_render_options_for_static_auto_marker_single_point_default_zoom() {
        // A single marker → bounds collapse to a point → max_diff == 0 → default zoom.
        let params = StaticQueryParams {
            marker: Some("10.0,20.0".to_string()),
            ..StaticQueryParams::default()
        };
        let opts = for_static_auto(params, 256, 256);
        assert!((opts.lon - 10.0).abs() < 1e-9);
        assert!((opts.lat - 20.0).abs() < 1e-9);
        assert_eq!(opts.zoom, 14.0);
    }

    #[test]
    fn test_render_options_for_static_auto_single_point_maxzoom_override() {
        // Single point + explicit maxzoom → default-zoom branch uses maxzoom.
        let params = StaticQueryParams {
            marker: Some("10.0,20.0".to_string()),
            maxzoom: Some(11),
            ..StaticQueryParams::default()
        };
        let opts = for_static_auto(params, 256, 256);
        assert_eq!(opts.zoom, 11.0);
    }

    #[test]
    fn test_render_options_for_static_auto_path_maxzoom_clamps() {
        // Tight bounds want a high zoom; maxzoom clamps it down.
        let params = StaticQueryParams {
            path: Some("0.0,0.0|0.001,0.001".to_string()),
            maxzoom: Some(8),
            ..StaticQueryParams::default()
        };
        let opts = for_static_auto(params, 256, 256);
        assert_eq!(opts.zoom, 8.0);
    }

    #[test]
    fn test_render_options_for_static_auto_latlng_swaps_coords() {
        // latlng=true means the path coords are parsed lat,lng (Google convention).
        let params = StaticQueryParams {
            path: Some("8.0,-2.0|12.0,2.0".to_string()),
            latlng: true,
            ..StaticQueryParams::default()
        };
        let opts = for_static_auto(params, 256, 256);
        assert!((opts.lon - 0.0).abs() < 1e-9);
        assert!((opts.lat - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_render_options_for_static_auto_geojson_linestring_fits() {
        let params = StaticQueryParams {
            geojson: Some(
                r#"{"type":"LineString","coordinates":[[-2.0,8.0],[2.0,12.0]]}"#.to_string(),
            ),
            ..StaticQueryParams::default()
        };
        let opts = for_static_auto(params, 256, 256);
        assert!((opts.lon - 0.0).abs() < 1e-9);
        assert!((opts.lat - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_render_options_for_static_auto_wide_path_zoom_zero() {
        // Bounds wider than 180° of adjusted diff → zoom clamps to 0.0.
        let params = StaticQueryParams {
            path: Some("-100.0,-10.0|100.0,10.0".to_string()),
            ..StaticQueryParams::default()
        };
        let opts = for_static_auto(params, 256, 256);
        assert_eq!(opts.zoom, 0.0);
    }

    #[test]
    fn test_render_options_for_static_auto_padding_widens_bounds() {
        // A large padding fraction must still yield a valid finite zoom.
        let params = StaticQueryParams {
            path: Some("-1.0,-1.0|1.0,1.0".to_string()),
            padding: Some(0.5),
            ..StaticQueryParams::default()
        };
        let opts = for_static_auto(params, 512, 256);
        assert!(opts.zoom.is_finite());
        assert!(opts.zoom >= 0.0);
    }
}
