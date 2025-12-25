use serde::Deserialize;

/// Image format for rendered output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
}

impl ImageFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "webp" => Some(Self::Webp),
            _ => None,
        }
    }

    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
        }
    }
}

/// Static image type (center, bbox, or auto)
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

impl StaticType {
    /// Parse static type from path parameter
    /// Examples:
    /// - "-122.4,37.8,12" -> Center
    /// - "-122.4,37.8,12@45" -> Center with bearing
    /// - "-122.4,37.8,12@45,60" -> Center with bearing and pitch
    /// - "-123,37,-122,38" -> BoundingBox
    /// - "auto" -> Auto
    pub fn from_str(s: &str) -> Result<Self, String> {
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
                let bearing = Some(
                    bearing_pitch[0]
                        .parse()
                        .map_err(|_| "Invalid bearing")?,
                );
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
    /// Parse coordinates as lat/lng instead of lng/lat
    #[serde(default)]
    #[allow(dead_code)]
    pub latlng: bool,
    /// Padding for bounding box (default 0.1)
    #[allow(dead_code)]
    pub padding: Option<f64>,
    /// Maximum zoom level for auto-fit
    #[allow(dead_code)]
    pub maxzoom: Option<u8>,
}

/// Options for rendering a map image
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Style ID for navigation
    pub style_id: String,
    /// Style JSON content (kept for future use)
    #[allow(dead_code)]
    pub style_json: String,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Pixel ratio / scale (1-9)
    pub scale: u8,
    /// Center longitude
    pub lon: f64,
    /// Center latitude
    pub lat: f64,
    /// Zoom level
    pub zoom: f64,
    /// Bearing (rotation) in degrees (reserved for future use)
    #[allow(dead_code)]
    pub bearing: f64,
    /// Pitch (tilt) in degrees (reserved for future use)
    #[allow(dead_code)]
    pub pitch: f64,
    /// Output format
    pub format: ImageFormat,
    /// Optional path overlay (reserved for future use)
    #[allow(dead_code)]
    pub path: Option<String>,
    /// Optional marker overlay (reserved for future use)
    #[allow(dead_code)]
    pub marker: Option<String>,
}

impl RenderOptions {
    /// Create options for a raster tile
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
        let lat_rad = ((1.0 - 2.0 * (y as f64) / n) * std::f64::consts::PI).sinh().atan();
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
        }
    }

    /// Create options for a static image
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
        let (lon, lat, zoom, bearing, pitch) = match static_type {
            StaticType::Center {
                lon,
                lat,
                zoom,
                bearing,
                pitch,
            } => (
                lon,
                lat,
                zoom,
                bearing.unwrap_or(0.0),
                pitch.unwrap_or(0.0),
            ),
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
                // For auto mode, we'll need to parse paths/markers
                // For now, default to world view
                // TODO: Implement bbox calculation from paths/markers
                (0.0, 0.0, 1.0, 0.0, 0.0)
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
        })
    }
}
