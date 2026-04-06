//! Overlay drawing for static map images
//!
//! Supports drawing paths (polylines) and markers on rendered map images.

use image::{Rgba, RgbaImage};
use serde_json::Value;

/// A point in geographic coordinates
#[derive(Debug, Clone, Copy)]
pub struct GeoPoint {
    pub lon: f64,
    pub lat: f64,
}

/// A path overlay to draw on the map
#[derive(Debug, Clone)]
pub struct PathOverlay {
    /// Points along the path
    pub points: Vec<GeoPoint>,
    /// Stroke color (RGBA)
    pub stroke_color: Rgba<u8>,
    /// Stroke width in pixels
    pub stroke_width: f32,
    /// Fill color (RGBA) for closed polygons
    pub fill_color: Option<Rgba<u8>>,
}

/// A marker overlay to draw on the map
#[derive(Debug, Clone)]
pub struct MarkerOverlay {
    /// Position of the marker
    pub position: GeoPoint,
    /// Marker color (RGBA)
    pub color: Rgba<u8>,
    /// Optional label text — not yet rendered; reserved for a future text-rendering pass
    #[allow(dead_code)]
    pub label: Option<String>,
    /// Marker size in pixels
    pub size: f32,
}

/// Decode a Google Encoded Polyline string into a vector of GeoPoints
///
/// The Google Polyline Algorithm encodes coordinates as a series of ASCII characters.
/// See: https://developers.google.com/maps/documentation/utilities/polylinealgorithm
///
/// Note: Google's format is (lat, lon) but we return GeoPoint with (lon, lat) for GeoJSON compatibility.
#[must_use]
pub fn decode_polyline(encoded: &str) -> Vec<GeoPoint> {
    let mut points = Vec::new();
    let mut index = 0;
    let mut lat = 0i64;
    let mut lng = 0i64;

    let bytes: Vec<u8> = encoded.bytes().collect();

    while index < bytes.len() {
        // Decode latitude
        let mut shift = 0;
        let mut result = 0i64;
        loop {
            if index >= bytes.len() {
                return points; // Malformed input
            }
            let b = (bytes[index] as i64) - 63;
            index += 1;
            result |= (b & 0x1f) << shift;
            shift += 5;
            if b < 0x20 {
                break;
            }
        }
        let dlat = if (result & 1) != 0 {
            !(result >> 1)
        } else {
            result >> 1
        };
        lat += dlat;

        // Decode longitude
        shift = 0;
        result = 0;
        loop {
            if index >= bytes.len() {
                return points; // Malformed input
            }
            let b = (bytes[index] as i64) - 63;
            index += 1;
            result |= (b & 0x1f) << shift;
            shift += 5;
            if b < 0x20 {
                break;
            }
        }
        let dlng = if (result & 1) != 0 {
            !(result >> 1)
        } else {
            result >> 1
        };
        lng += dlng;

        // Convert to float (divide by 1e5 for standard precision)
        points.push(GeoPoint {
            lon: lng as f64 / 1e5,
            lat: lat as f64 / 1e5,
        });
    }

    points
}

/// Encode a vector of GeoPoints into a Google Encoded Polyline string
///
/// This is useful for generating compact path representations.
#[cfg(test)]
fn encode_polyline(points: &[GeoPoint]) -> String {
    let mut encoded = String::new();
    let mut prev_lat = 0i64;
    let mut prev_lng = 0i64;

    for point in points {
        let lat = (point.lat * 1e5).round() as i64;
        let lng = (point.lon * 1e5).round() as i64;

        encode_value(lat - prev_lat, &mut encoded);
        encode_value(lng - prev_lng, &mut encoded);

        prev_lat = lat;
        prev_lng = lng;
    }

    encoded
}

/// Encode a single value for polyline encoding
#[cfg(test)]
fn encode_value(mut value: i64, output: &mut String) {
    // Left-shift the value by one bit and invert if negative
    value = if value < 0 { !(value << 1) } else { value << 1 };

    // Break into 5-bit chunks and add 63 to each chunk
    while value >= 0x20 {
        output.push(((value & 0x1f) as u8 + 63 + 0x20) as char);
        value >>= 5;
    }
    output.push((value as u8 + 63) as char);
}

/// Check if a string looks like an encoded polyline (vs pipe-separated coordinates)
fn is_encoded_polyline(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    // If it contains a comma followed by digits or minus sign, it's likely coordinates
    // e.g., "0,0" or "-122.4,37.8"
    if s.contains(',') {
        // Check if it looks like coordinate pairs
        for part in s.split('|') {
            if part.split(',').count() >= 2 {
                // If we can parse both parts as numbers, it's coordinates not polyline
                let coords: Vec<&str> = part.split(',').collect();
                if coords.len() >= 2
                    && coords[0].parse::<f64>().is_ok()
                    && coords[1].parse::<f64>().is_ok()
                {
                    return false;
                }
            }
        }
    }

    // Google polyline encoding uses ASCII 63-126 (? to ~)
    // Valid polyline chars: ?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\]^_`abcdefghijklmnopqrstuvwxyz{|}~
    s.chars().all(|c| {
        let code = c as u32;
        (63..=126).contains(&code)
    })
}

/// Parse a path string into a PathOverlay
///
/// Format: `path-{strokeWidth}+{strokeColor}-{fillColor}({coordinates})`
/// Example: `path-5+f00-88f(0,0|10,10|20,0)`
/// Or encoded polyline: `path-5+f00(encodedPolylineString)`
/// Or simple encoded: `enc:_p~iF~ps|U_ulLnnqC_mqNvxq`
#[must_use]
/// Parse a coordinate pair `"a,b"` respecting the `latlng` flag.
///
/// When `latlng` is `true` the string is interpreted as `"lat,lon"` (Google Maps
/// convention) and the values are swapped so the returned point is always
/// `GeoPoint { lon, lat }` (GeoJSON / Web Mercator convention).
fn parse_coord_pair(s: &str, latlng: bool) -> Option<GeoPoint> {
    let xy: Vec<&str> = s.split(',').collect();
    if xy.len() >= 2 {
        let a: f64 = xy[0].parse().ok()?;
        let b: f64 = xy[1].parse().ok()?;
        if latlng {
            Some(GeoPoint { lon: b, lat: a })
        } else {
            Some(GeoPoint { lon: a, lat: b })
        }
    } else {
        None
    }
}

pub fn parse_path(path_str: &str, latlng: bool) -> Option<PathOverlay> {
    let mut stroke_width = 3.0f32;
    let mut stroke_color = Rgba([0, 0, 255, 255]);
    let mut fill_color: Option<Rgba<u8>> = None;
    let mut points = Vec::new();

    let path_str = path_str.trim();

    if let Some(encoded) = path_str.strip_prefix("enc:") {
        points = decode_polyline(encoded);
    } else if let Some(rest) = path_str.strip_prefix("path-") {
        if let Some(paren_idx) = rest.find('(') {
            let style_part = &rest[..paren_idx];
            let coords_part = &rest[paren_idx + 1..rest.len() - 1];

            let parts: Vec<&str> = style_part.split('+').collect();
            if !parts.is_empty() {
                stroke_width = parts[0].parse().unwrap_or(3.0);
            }
            if parts.len() > 1 {
                let color_parts: Vec<&str> = parts[1].split('-').collect();
                stroke_color = parse_hex_color(color_parts[0]).unwrap_or(stroke_color);
                if color_parts.len() > 1 {
                    fill_color = parse_hex_color(color_parts[1]);
                }
            }

            if is_encoded_polyline(coords_part) {
                points = decode_polyline(coords_part);
            } else {
                for coord in coords_part.split('|') {
                    if let Some(p) = parse_coord_pair(coord, latlng) {
                        points.push(p);
                    }
                }
            }
        }
    } else if is_encoded_polyline(path_str) {
        points = decode_polyline(path_str);
    } else {
        for coord in path_str.split('|') {
            if let Some(p) = parse_coord_pair(coord, latlng) {
                points.push(p);
            }
        }
    }

    if points.len() >= 2 {
        Some(PathOverlay {
            points,
            stroke_color,
            stroke_width,
            fill_color,
        })
    } else {
        None
    }
}

/// Parse a marker string into a MarkerOverlay
///
/// Format: `{icon}-{label}+{color}({lon},{lat})`
/// Example: `pin-s+f00(-122.4,37.8)`
/// Or simple: `{lon},{lat}`
#[must_use]
pub fn parse_marker(marker_str: &str) -> Option<MarkerOverlay> {
    let marker_str = marker_str.trim();

    // Default values
    let mut color = Rgba([255, 0, 0, 255]); // Red
    let mut label: Option<String> = None;
    let mut size = 24.0f32;

    // Try to parse pin-{size}-{label}+{color}({lon},{lat}) format
    if marker_str.starts_with("pin-") {
        if let Some(paren_idx) = marker_str.find('(') {
            let style_part = &marker_str[4..paren_idx]; // Skip "pin-"
            let coords_part = &marker_str[paren_idx + 1..marker_str.len() - 1];

            // Parse style: s, m, l for size, optional label, + color
            let parts: Vec<&str> = style_part.split('+').collect();
            if !parts.is_empty() {
                let size_label: Vec<&str> = parts[0].split('-').collect();
                size = match size_label[0] {
                    "s" => 20.0,
                    "m" => 28.0,
                    "l" => 36.0,
                    _ => 24.0,
                };
                if size_label.len() > 1 {
                    label = Some(size_label[1].to_string());
                }
            }
            if parts.len() > 1 {
                color = parse_hex_color(parts[1]).unwrap_or(color);
            }

            // Parse coordinates
            let xy: Vec<&str> = coords_part.split(',').collect();
            if xy.len() >= 2
                && let (Ok(lon), Ok(lat)) = (xy[0].parse(), xy[1].parse())
            {
                return Some(MarkerOverlay {
                    position: GeoPoint { lon, lat },
                    color,
                    label,
                    size,
                });
            }
        }
    } else {
        // Simple format: lon,lat
        let xy: Vec<&str> = marker_str.split(',').collect();
        if xy.len() >= 2
            && let (Ok(lon), Ok(lat)) = (xy[0].parse(), xy[1].parse())
        {
            return Some(MarkerOverlay {
                position: GeoPoint { lon, lat },
                color,
                label: None,
                size,
            });
        }
    }

    None
}

/// Parse a hex color string (3 or 6 digits, with optional alpha)
fn parse_hex_color(hex: &str) -> Option<Rgba<u8>> {
    let hex = hex.trim_start_matches('#');

    match hex.len() {
        3 => {
            // Short format: RGB -> RRGGBB
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(Rgba([r, g, b, 255]))
        }
        4 => {
            // Short format with alpha: RGBA -> RRGGBBAA
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            let a = u8::from_str_radix(&hex[3..4], 16).ok()? * 17;
            Some(Rgba([r, g, b, a]))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Rgba([r, g, b, 255]))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(Rgba([r, g, b, a]))
        }
        _ => None,
    }
}

/// Convert geographic coordinates to pixel coordinates
fn geo_to_pixel(
    point: &GeoPoint,
    center_lon: f64,
    center_lat: f64,
    zoom: f64,
    width: u32,
    height: u32,
    scale: f32,
) -> (f32, f32) {
    // Web Mercator projection
    let tile_size = 512.0 * scale as f64;
    let scale_factor = tile_size * 2.0_f64.powf(zoom) / 360.0;

    // Convert lon/lat to pixels relative to center
    let dx = (point.lon - center_lon) * scale_factor;

    // Mercator Y transformation
    let center_y = (std::f64::consts::PI / 4.0 + center_lat.to_radians() / 2.0)
        .tan()
        .ln();
    let point_y = (std::f64::consts::PI / 4.0 + point.lat.to_radians() / 2.0)
        .tan()
        .ln();
    let dy = -(point_y - center_y) * scale_factor * 180.0 / std::f64::consts::PI;

    // Convert to image coordinates (center of image is center of map)
    let px = (width as f64 / 2.0 + dx) as f32;
    let py = (height as f64 / 2.0 + dy) as f32;

    (px, py)
}

/// Draw overlays on an image
pub fn draw_overlays(
    image: &mut RgbaImage,
    paths: &[PathOverlay],
    markers: &[MarkerOverlay],
    center_lon: f64,
    center_lat: f64,
    zoom: f64,
    scale: f32,
) {
    let width = image.width();
    let height = image.height();

    // Draw paths first (underneath markers)
    for path in paths {
        draw_path(
            image, path, center_lon, center_lat, zoom, width, height, scale,
        );
    }

    // Draw markers on top
    for marker in markers {
        draw_marker(
            image, marker, center_lon, center_lat, zoom, width, height, scale,
        );
    }
}

/// Draw a path on the image
#[allow(clippy::too_many_arguments)]
fn draw_path(
    image: &mut RgbaImage,
    path: &PathOverlay,
    center_lon: f64,
    center_lat: f64,
    zoom: f64,
    width: u32,
    height: u32,
    scale: f32,
) {
    if path.points.len() < 2 {
        return;
    }

    let pixels: Vec<(f32, f32)> = path
        .points
        .iter()
        .map(|p| geo_to_pixel(p, center_lon, center_lat, zoom, width, height, scale))
        .collect();

    if let Some(fill) = path.fill_color {
        fill_polygon(image, &pixels, fill);
    }

    let stroke_width = path.stroke_width * scale;
    for pair in pixels.windows(2) {
        draw_line(
            image,
            pair[0].0,
            pair[0].1,
            pair[1].0,
            pair[1].1,
            path.stroke_color,
            stroke_width,
        );
    }
}

/// Draw a line segment with thickness using Bresenham's algorithm
fn draw_line(
    image: &mut RgbaImage,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: Rgba<u8>,
    thickness: f32,
) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let length = (dx * dx + dy * dy).sqrt();

    if length < 0.5 {
        return;
    }

    let steps = length.ceil() as i32;
    let half_thick = thickness / 2.0;

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let cx = x0 + dx * t;
        let cy = y0 + dy * t;

        // Draw a filled circle at each point for thickness
        for ox in (-half_thick.ceil() as i32)..=(half_thick.ceil() as i32) {
            for oy in (-half_thick.ceil() as i32)..=(half_thick.ceil() as i32) {
                let dist = ((ox * ox + oy * oy) as f32).sqrt();
                if dist <= half_thick {
                    let px = (cx + ox as f32) as i32;
                    let py = (cy + oy as f32) as i32;

                    if px >= 0 && py >= 0 && px < image.width() as i32 && py < image.height() as i32
                    {
                        blend_pixel(image, px as u32, py as u32, color);
                    }
                }
            }
        }
    }
}

/// Fill a polygon using the scanline even-odd algorithm with alpha compositing.
///
/// `pixels` are in image-space (px, py). The polygon is auto-closed when the
/// first and last vertex are more than 0.5 px apart.
fn fill_polygon(image: &mut RgbaImage, pixels: &[(f32, f32)], color: Rgba<u8>) {
    if pixels.len() < 3 {
        return;
    }

    let mut poly = pixels.to_vec();
    let first = poly[0];
    let last = *poly.last().unwrap();
    if (first.0 - last.0).abs() > 0.5 || (first.1 - last.1).abs() > 0.5 {
        poly.push(first);
    }

    let min_y = poly.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let max_y = poly.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
    let img_h = image.height() as i32;
    let img_w = image.width() as i32;
    let scan_min = (min_y.floor() as i32).max(0);
    let scan_max = (max_y.ceil() as i32).min(img_h - 1);

    let n = poly.len();
    for y in scan_min..=scan_max {
        let yf = y as f32 + 0.5;
        let mut xs: Vec<f32> = Vec::new();

        for i in 0..n - 1 {
            let (x0, y0) = poly[i];
            let (x1, y1) = poly[i + 1];
            if (y1 - y0).abs() < f32::EPSILON {
                continue;
            }
            let y_lo = y0.min(y1);
            let y_hi = y0.max(y1);
            if yf < y_lo || yf >= y_hi {
                continue;
            }
            let t = (yf - y0) / (y1 - y0);
            xs.push(x0 + t * (x1 - x0));
        }

        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        for chunk in xs.chunks(2) {
            if chunk.len() < 2 {
                break;
            }
            let x_start = (chunk[0].floor() as i32).max(0);
            let x_end = (chunk[1].ceil() as i32).min(img_w - 1);
            for x in x_start..=x_end {
                blend_pixel(image, x as u32, y as u32, color);
            }
        }
    }
}

/// Draw a marker on the image
#[allow(clippy::too_many_arguments)]
fn draw_marker(
    image: &mut RgbaImage,
    marker: &MarkerOverlay,
    center_lon: f64,
    center_lat: f64,
    zoom: f64,
    width: u32,
    height: u32,
    scale: f32,
) {
    let (px, py) = geo_to_pixel(
        &marker.position,
        center_lon,
        center_lat,
        zoom,
        width,
        height,
        scale,
    );

    let size = marker.size * scale;
    let half_size = size / 2.0;

    // Draw a simple pin marker (teardrop shape)
    // Draw the circle part
    let circle_radius = half_size * 0.6;
    let circle_cy = py - size * 0.3;

    for ox in (-circle_radius.ceil() as i32)..=(circle_radius.ceil() as i32) {
        for oy in (-circle_radius.ceil() as i32)..=(circle_radius.ceil() as i32) {
            let dist = ((ox * ox + oy * oy) as f32).sqrt();
            if dist <= circle_radius {
                let mx = (px + ox as f32) as i32;
                let my = (circle_cy + oy as f32) as i32;

                if mx >= 0 && my >= 0 && mx < image.width() as i32 && my < image.height() as i32 {
                    blend_pixel(image, mx as u32, my as u32, marker.color);
                }
            }
        }
    }

    // Draw the point part (triangle pointing down)
    let point_y = py;
    let triangle_height = size * 0.4;
    let triangle_width = circle_radius * 0.8;

    for y_offset in 0..=(triangle_height as i32) {
        let progress = y_offset as f32 / triangle_height;
        let width_at_y = triangle_width * (1.0 - progress);

        for x_offset in (-width_at_y.ceil() as i32)..=(width_at_y.ceil() as i32) {
            if (x_offset as f32).abs() <= width_at_y {
                let mx = (px + x_offset as f32) as i32;
                let my = (circle_cy + circle_radius + y_offset as f32) as i32;

                if mx >= 0
                    && my >= 0
                    && mx < image.width() as i32
                    && my < image.height() as i32
                    && my <= point_y as i32
                {
                    blend_pixel(image, mx as u32, my as u32, marker.color);
                }
            }
        }
    }

    // Draw white inner circle
    let inner_radius = circle_radius * 0.4;
    let white = Rgba([255, 255, 255, 255]);

    for ox in (-inner_radius.ceil() as i32)..=(inner_radius.ceil() as i32) {
        for oy in (-inner_radius.ceil() as i32)..=(inner_radius.ceil() as i32) {
            let dist = ((ox * ox + oy * oy) as f32).sqrt();
            if dist <= inner_radius {
                let mx = (px + ox as f32) as i32;
                let my = (circle_cy + oy as f32) as i32;

                if mx >= 0 && my >= 0 && mx < image.width() as i32 && my < image.height() as i32 {
                    blend_pixel(image, mx as u32, my as u32, white);
                }
            }
        }
    }
}

/// Blend a pixel with alpha compositing
fn blend_pixel(image: &mut RgbaImage, x: u32, y: u32, color: Rgba<u8>) {
    let existing = image.get_pixel(x, y);
    let alpha = color.0[3] as f32 / 255.0;
    let inv_alpha = 1.0 - alpha;

    let r = (color.0[0] as f32 * alpha + existing.0[0] as f32 * inv_alpha) as u8;
    let g = (color.0[1] as f32 * alpha + existing.0[1] as f32 * inv_alpha) as u8;
    let b = (color.0[2] as f32 * alpha + existing.0[2] as f32 * inv_alpha) as u8;
    let a = ((color.0[3] as f32 + existing.0[3] as f32 * inv_alpha).min(255.0)) as u8;

    image.put_pixel(x, y, Rgba([r, g, b, a]));
}

/// Calculate bounding box from paths and markers for auto-fit
#[must_use]
pub fn calculate_bounds(
    paths: &[PathOverlay],
    markers: &[MarkerOverlay],
) -> Option<(f64, f64, f64, f64)> {
    let mut min_lon = f64::MAX;
    let mut min_lat = f64::MAX;
    let mut max_lon = f64::MIN;
    let mut max_lat = f64::MIN;
    let mut has_points = false;

    for path in paths {
        for point in &path.points {
            min_lon = min_lon.min(point.lon);
            min_lat = min_lat.min(point.lat);
            max_lon = max_lon.max(point.lon);
            max_lat = max_lat.max(point.lat);
            has_points = true;
        }
    }

    for marker in markers {
        min_lon = min_lon.min(marker.position.lon);
        min_lat = min_lat.min(marker.position.lat);
        max_lon = max_lon.max(marker.position.lon);
        max_lat = max_lat.max(marker.position.lat);
        has_points = true;
    }

    if has_points {
        Some((min_lon, min_lat, max_lon, max_lat))
    } else {
        None
    }
}

/// Parse a GeoJSON string into path and marker overlays.
///
/// Accepts GeoJSON `Geometry`, `Feature`, or `FeatureCollection`.
/// Supported geometry types: `Point`, `LineString`, `MultiLineString`,
/// `Polygon` (outer ring only), `MultiPolygon` (outer rings only).
///
/// Styling is read from `Feature.properties`:
/// - `stroke` — hex stroke color (default `0000ff`)
/// - `stroke-width` — stroke width in pixels (default `3.0`)
/// - `fill` — hex fill color for polygons (optional)
/// - `marker-color` — hex marker color (default `ff0000`)
/// - `marker-size` — marker size in pixels (default `12.0`)
///
/// Invalid JSON is silently ignored (logged at WARN level).
pub fn parse_geojson(geojson_str: &str) -> (Vec<PathOverlay>, Vec<MarkerOverlay>) {
    let mut paths = Vec::new();
    let mut markers = Vec::new();

    let value = match serde_json::from_str::<Value>(geojson_str) {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!("Failed to parse GeoJSON overlay string");
            return (paths, markers);
        }
    };

    process_geojson_value(&value, &mut paths, &mut markers);
    (paths, markers)
}

fn process_geojson_value(
    value: &Value,
    paths: &mut Vec<PathOverlay>,
    markers: &mut Vec<MarkerOverlay>,
) {
    match value.get("type").and_then(Value::as_str) {
        Some("FeatureCollection") => {
            if let Some(features) = value.get("features").and_then(Value::as_array) {
                for feat in features {
                    process_geojson_value(feat, paths, markers);
                }
            }
        }
        Some("Feature") => {
            let props = value.get("properties");
            let stroke_color = props
                .and_then(|p| p.get("stroke"))
                .and_then(Value::as_str)
                .and_then(parse_hex_color)
                .unwrap_or(Rgba([0, 0, 255, 255]));
            let stroke_width = props
                .and_then(|p| p.get("stroke-width"))
                .and_then(Value::as_f64)
                .unwrap_or(3.0) as f32;
            let fill_color = props
                .and_then(|p| p.get("fill"))
                .and_then(Value::as_str)
                .and_then(parse_hex_color);
            let marker_color = props
                .and_then(|p| p.get("marker-color"))
                .and_then(Value::as_str)
                .and_then(parse_hex_color)
                .unwrap_or(Rgba([255, 0, 0, 255]));
            let marker_size = props
                .and_then(|p| p.get("marker-size"))
                .map(parse_marker_size)
                .unwrap_or(12.0);

            if let Some(geom) = value.get("geometry") {
                process_geometry(
                    geom,
                    stroke_color,
                    stroke_width,
                    fill_color,
                    marker_color,
                    marker_size,
                    paths,
                    markers,
                );
            }
        }
        Some(
            "Point" | "MultiPoint" | "LineString" | "MultiLineString" | "Polygon" | "MultiPolygon",
        ) => {
            process_geometry(
                value,
                Rgba([0, 0, 255, 255]),
                3.0,
                None,
                Rgba([255, 0, 0, 255]),
                12.0,
                paths,
                markers,
            );
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn process_geometry(
    geom: &Value,
    stroke_color: Rgba<u8>,
    stroke_width: f32,
    fill_color: Option<Rgba<u8>>,
    marker_color: Rgba<u8>,
    marker_size: f32,
    paths: &mut Vec<PathOverlay>,
    markers: &mut Vec<MarkerOverlay>,
) {
    match geom.get("type").and_then(Value::as_str) {
        Some("Point") => {
            if let Some(pos) = geojson_point(geom) {
                markers.push(MarkerOverlay {
                    position: pos,
                    color: marker_color,
                    label: None,
                    size: marker_size,
                });
            }
        }
        Some("MultiPoint") => {
            if let Some(coords) = geom.get("coordinates").and_then(Value::as_array) {
                for coord in coords {
                    if let Some(arr) = coord.as_array()
                        && let (Some(lon), Some(lat)) = (
                            arr.first().and_then(Value::as_f64),
                            arr.get(1).and_then(Value::as_f64),
                        )
                    {
                        markers.push(MarkerOverlay {
                            position: GeoPoint { lon, lat },
                            color: marker_color,
                            label: None,
                            size: marker_size,
                        });
                    }
                }
            }
        }
        Some("LineString") => {
            let pts = geojson_coord_array(geom, "coordinates");
            if pts.len() >= 2 {
                paths.push(PathOverlay {
                    points: pts,
                    stroke_color,
                    stroke_width,
                    fill_color: None,
                });
            }
        }
        Some("MultiLineString") => {
            if let Some(lines) = geom.get("coordinates").and_then(Value::as_array) {
                for line in lines {
                    if let Some(arr) = line.as_array() {
                        let pts = coords_to_geopoints(arr);
                        if pts.len() >= 2 {
                            paths.push(PathOverlay {
                                points: pts,
                                stroke_color,
                                stroke_width,
                                fill_color: None,
                            });
                        }
                    }
                }
            }
        }
        Some("Polygon") => {
            if let Some(rings) = geom.get("coordinates").and_then(Value::as_array)
                && let Some(outer) = rings.first()
                && let Some(arr) = outer.as_array()
            {
                let pts = coords_to_geopoints(arr);
                if pts.len() >= 3 {
                    paths.push(PathOverlay {
                        points: pts,
                        stroke_color,
                        stroke_width,
                        fill_color,
                    });
                }
            }
        }
        Some("MultiPolygon") => {
            if let Some(polys) = geom.get("coordinates").and_then(Value::as_array) {
                for poly in polys {
                    if let Some(rings) = poly.as_array()
                        && let Some(outer) = rings.first()
                        && let Some(arr) = outer.as_array()
                    {
                        let pts = coords_to_geopoints(arr);
                        if pts.len() >= 3 {
                            paths.push(PathOverlay {
                                points: pts,
                                stroke_color,
                                stroke_width,
                                fill_color,
                            });
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

/// Parse simplestyle-spec `marker-size`.
///
/// Accepts the string enum (`"small"`, `"medium"`, `"large"`) defined by the
/// simplestyle spec **and** a raw numeric pixel value for convenience.
fn parse_marker_size(value: &Value) -> f32 {
    if let Some(s) = value.as_str() {
        match s {
            "small" => 8.0,
            "medium" => 12.0,
            "large" => 18.0,
            _ => 12.0,
        }
    } else {
        value.as_f64().unwrap_or(12.0) as f32
    }
}

fn geojson_point(geom: &Value) -> Option<GeoPoint> {
    let arr = geom.get("coordinates")?.as_array()?;
    let lon = arr.first()?.as_f64()?;
    let lat = arr.get(1)?.as_f64()?;
    Some(GeoPoint { lon, lat })
}

fn geojson_coord_array(geom: &Value, field: &str) -> Vec<GeoPoint> {
    geom.get(field)
        .and_then(Value::as_array)
        .map(|a| coords_to_geopoints(a))
        .unwrap_or_default()
}

fn coords_to_geopoints(arr: &[Value]) -> Vec<GeoPoint> {
    arr.iter()
        .filter_map(|c| {
            let pair = c.as_array()?;
            let lon = pair.first()?.as_f64()?;
            let lat = pair.get(1)?.as_f64()?;
            Some(GeoPoint { lon, lat })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Hex Color Parsing Tests
    // ============================================================

    #[test]
    fn test_parse_hex_color_3_digit() {
        assert_eq!(parse_hex_color("f00"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(parse_hex_color("0f0"), Some(Rgba([0, 255, 0, 255])));
        assert_eq!(parse_hex_color("00f"), Some(Rgba([0, 0, 255, 255])));
        assert_eq!(parse_hex_color("fff"), Some(Rgba([255, 255, 255, 255])));
        assert_eq!(parse_hex_color("000"), Some(Rgba([0, 0, 0, 255])));
        assert_eq!(parse_hex_color("abc"), Some(Rgba([170, 187, 204, 255])));
    }

    #[test]
    fn test_parse_hex_color_4_digit_with_alpha() {
        assert_eq!(parse_hex_color("f00f"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(parse_hex_color("f008"), Some(Rgba([255, 0, 0, 136])));
        assert_eq!(parse_hex_color("f000"), Some(Rgba([255, 0, 0, 0])));
    }

    #[test]
    fn test_parse_hex_color_6_digit() {
        assert_eq!(parse_hex_color("ff0000"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(parse_hex_color("00ff00"), Some(Rgba([0, 255, 0, 255])));
        assert_eq!(parse_hex_color("0000ff"), Some(Rgba([0, 0, 255, 255])));
        assert_eq!(parse_hex_color("ffffff"), Some(Rgba([255, 255, 255, 255])));
        assert_eq!(parse_hex_color("000000"), Some(Rgba([0, 0, 0, 255])));
        assert_eq!(parse_hex_color("aabbcc"), Some(Rgba([170, 187, 204, 255])));
    }

    #[test]
    fn test_parse_hex_color_8_digit_with_alpha() {
        assert_eq!(parse_hex_color("ff0000ff"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(parse_hex_color("ff000080"), Some(Rgba([255, 0, 0, 128])));
        assert_eq!(parse_hex_color("ff000000"), Some(Rgba([255, 0, 0, 0])));
        assert_eq!(parse_hex_color("00ff00c0"), Some(Rgba([0, 255, 0, 192])));
    }

    #[test]
    fn test_parse_hex_color_with_hash_prefix() {
        assert_eq!(parse_hex_color("#f00"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(parse_hex_color("#00f"), Some(Rgba([0, 0, 255, 255])));
        assert_eq!(parse_hex_color("#ff0000"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(parse_hex_color("#ff000080"), Some(Rgba([255, 0, 0, 128])));
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert_eq!(parse_hex_color(""), None);
        assert_eq!(parse_hex_color("f"), None);
        assert_eq!(parse_hex_color("ff"), None);
        assert_eq!(parse_hex_color("fffff"), None); // 5 digits invalid
        assert_eq!(parse_hex_color("fffffff"), None); // 7 digits invalid
        assert_eq!(parse_hex_color("fffffffff"), None); // 9 digits invalid
        assert_eq!(parse_hex_color("xyz"), None); // Invalid hex chars
        assert_eq!(parse_hex_color("gggggg"), None); // Invalid hex chars
    }

    #[test]
    fn test_parse_hex_color_case_insensitive() {
        assert_eq!(parse_hex_color("F00"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(parse_hex_color("FF0000"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(parse_hex_color("AbCdEf"), Some(Rgba([171, 205, 239, 255])));
    }

    // ============================================================
    // Path Parsing Tests
    // ============================================================

    #[test]
    fn test_parse_path_basic() {
        let path = parse_path("path-5+f00(-122.4,37.8|-122.5,37.9)", false).unwrap();
        assert_eq!(path.points.len(), 2);
        assert_eq!(path.stroke_width, 5.0);
        assert_eq!(path.stroke_color, Rgba([255, 0, 0, 255]));
        assert!((path.points[0].lon - (-122.4)).abs() < 0.001);
        assert!((path.points[0].lat - 37.8).abs() < 0.001);
        assert!((path.points[1].lon - (-122.5)).abs() < 0.001);
        assert!((path.points[1].lat - 37.9).abs() < 0.001);
    }

    #[test]
    fn test_parse_path_with_fill_color() {
        let path = parse_path("path-3+00f-ff0(-10,20|30,40|50,60)", false).unwrap();
        assert_eq!(path.points.len(), 3);
        assert_eq!(path.stroke_width, 3.0);
        assert_eq!(path.stroke_color, Rgba([0, 0, 255, 255]));
        assert_eq!(path.fill_color, Some(Rgba([255, 255, 0, 255])));
    }

    #[test]
    fn test_parse_path_many_points() {
        let path =
            parse_path("path-2+fff(0,0|1,1|2,2|3,3|4,4|5,5|6,6|7,7|8,8|9,9)", false).unwrap();
        assert_eq!(path.points.len(), 10);
        assert_eq!(path.stroke_width, 2.0);
        for (i, point) in path.points.iter().enumerate() {
            assert!((point.lon - i as f64).abs() < 0.001);
            assert!((point.lat - i as f64).abs() < 0.001);
        }
    }

    #[test]
    fn test_parse_path_negative_coordinates() {
        let path = parse_path("path-1+000(-180,-90|180,90)", false).unwrap();
        assert_eq!(path.points.len(), 2);
        assert!((path.points[0].lon - (-180.0)).abs() < 0.001);
        assert!((path.points[0].lat - (-90.0)).abs() < 0.001);
        assert!((path.points[1].lon - 180.0).abs() < 0.001);
        assert!((path.points[1].lat - 90.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_path_decimal_precision() {
        let path = parse_path(
            "path-1+f00(-122.123456,37.987654|-122.654321,37.123456)",
            false,
        )
        .unwrap();
        assert_eq!(path.points.len(), 2);
        assert!((path.points[0].lon - (-122.123456)).abs() < 0.0000001);
        assert!((path.points[0].lat - 37.987654).abs() < 0.0000001);
    }

    #[test]
    fn test_parse_path_simple_format() {
        // Simple format without path- prefix
        let path = parse_path("0,0|10,10|20,0", false).unwrap();
        assert_eq!(path.points.len(), 3);
        // Should use defaults
        assert_eq!(path.stroke_width, 3.0);
        assert_eq!(path.stroke_color, Rgba([0, 0, 255, 255])); // Default blue
    }

    #[test]
    fn test_parse_path_single_point_returns_none() {
        assert!(parse_path("path-5+f00(-122.4,37.8)", false).is_none());
        assert!(parse_path("0,0", false).is_none());
    }

    #[test]
    fn test_parse_path_empty_returns_none() {
        assert!(parse_path("", false).is_none());
        assert!(parse_path("path-5+f00()", false).is_none());
    }

    #[test]
    fn test_parse_path_invalid_coordinates() {
        // Invalid coordinate format should be skipped
        let path = parse_path("path-5+f00(invalid|0,0|1,1)", false).unwrap();
        assert_eq!(path.points.len(), 2); // Only valid points
    }

    #[test]
    fn test_parse_path_default_stroke_width() {
        // When width can't be parsed, should use default 3.0
        let path = parse_path("path-invalid+f00(0,0|1,1)", false).unwrap();
        assert_eq!(path.stroke_width, 3.0);
    }

    #[test]
    fn test_parse_path_whitespace() {
        let path = parse_path("  path-5+f00(-122.4,37.8|-122.5,37.9)  ", false).unwrap();
        assert_eq!(path.points.len(), 2);
    }

    #[test]
    fn test_parse_path_6_digit_color() {
        let path = parse_path("path-5+ff5500(0,0|1,1)", false).unwrap();
        assert_eq!(path.stroke_color, Rgba([255, 85, 0, 255]));
    }

    // ============================================================
    // Marker Parsing Tests
    // ============================================================

    #[test]
    fn test_parse_marker_basic() {
        let marker = parse_marker("pin-s+f00(-122.4,37.8)").unwrap();
        assert!((marker.position.lon - (-122.4)).abs() < 0.001);
        assert!((marker.position.lat - 37.8).abs() < 0.001);
        assert_eq!(marker.color, Rgba([255, 0, 0, 255]));
        assert_eq!(marker.size, 20.0); // 's' = small = 20.0
    }

    #[test]
    fn test_parse_marker_sizes() {
        let small = parse_marker("pin-s+fff(0,0)").unwrap();
        assert_eq!(small.size, 20.0);

        let medium = parse_marker("pin-m+fff(0,0)").unwrap();
        assert_eq!(medium.size, 28.0);

        let large = parse_marker("pin-l+fff(0,0)").unwrap();
        assert_eq!(large.size, 36.0);

        // Unknown size defaults to 24.0
        let unknown = parse_marker("pin-x+fff(0,0)").unwrap();
        assert_eq!(unknown.size, 24.0);
    }

    #[test]
    fn test_parse_marker_with_label() {
        let marker = parse_marker("pin-s-A+f00(-122.4,37.8)").unwrap();
        assert_eq!(marker.label, Some("A".to_string()));
        assert_eq!(marker.size, 20.0);
        assert_eq!(marker.color, Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn test_parse_marker_simple_format() {
        // Simple lon,lat format
        let marker = parse_marker("-122.4,37.8").unwrap();
        assert!((marker.position.lon - (-122.4)).abs() < 0.001);
        assert!((marker.position.lat - 37.8).abs() < 0.001);
        // Should use defaults
        assert_eq!(marker.color, Rgba([255, 0, 0, 255])); // Default red
        assert_eq!(marker.size, 24.0);
        assert_eq!(marker.label, None);
    }

    #[test]
    fn test_parse_marker_negative_coordinates() {
        let marker = parse_marker("pin-m+00f(-180,-90)").unwrap();
        assert!((marker.position.lon - (-180.0)).abs() < 0.001);
        assert!((marker.position.lat - (-90.0)).abs() < 0.001);
    }

    #[test]
    fn test_parse_marker_decimal_precision() {
        let marker = parse_marker("pin-s+f00(-122.123456,37.987654)").unwrap();
        assert!((marker.position.lon - (-122.123456)).abs() < 0.0000001);
        assert!((marker.position.lat - 37.987654).abs() < 0.0000001);
    }

    #[test]
    fn test_parse_marker_6_digit_color() {
        let marker = parse_marker("pin-s+ff5500(0,0)").unwrap();
        assert_eq!(marker.color, Rgba([255, 85, 0, 255]));
    }

    #[test]
    fn test_parse_marker_invalid_returns_none() {
        assert!(parse_marker("").is_none());
        assert!(parse_marker("invalid").is_none());
        assert!(parse_marker("pin-s+f00()").is_none());
        assert!(parse_marker("pin-s+f00(invalid)").is_none());
        assert!(parse_marker("pin-s+f00(only_one_value)").is_none());
    }

    #[test]
    fn test_parse_marker_whitespace() {
        let marker = parse_marker("  pin-s+f00(-122.4,37.8)  ").unwrap();
        assert!((marker.position.lon - (-122.4)).abs() < 0.001);
    }

    // ============================================================
    // Bounds Calculation Tests
    // ============================================================

    #[test]
    fn test_calculate_bounds_single_marker() {
        let markers = vec![MarkerOverlay {
            position: GeoPoint {
                lon: -122.4,
                lat: 37.8,
            },
            color: Rgba([255, 0, 0, 255]),
            label: None,
            size: 24.0,
        }];

        let bounds = calculate_bounds(&[], &markers).unwrap();
        assert!((bounds.0 - (-122.4)).abs() < 0.001); // min_lon
        assert!((bounds.1 - 37.8).abs() < 0.001); // min_lat
        assert!((bounds.2 - (-122.4)).abs() < 0.001); // max_lon
        assert!((bounds.3 - 37.8).abs() < 0.001); // max_lat
    }

    #[test]
    fn test_calculate_bounds_multiple_markers() {
        let markers = vec![
            MarkerOverlay {
                position: GeoPoint {
                    lon: -122.4,
                    lat: 37.8,
                },
                color: Rgba([255, 0, 0, 255]),
                label: None,
                size: 24.0,
            },
            MarkerOverlay {
                position: GeoPoint {
                    lon: -122.5,
                    lat: 37.7,
                },
                color: Rgba([0, 255, 0, 255]),
                label: None,
                size: 24.0,
            },
        ];

        let bounds = calculate_bounds(&[], &markers).unwrap();
        assert!((bounds.0 - (-122.5)).abs() < 0.001); // min_lon
        assert!((bounds.1 - 37.7).abs() < 0.001); // min_lat
        assert!((bounds.2 - (-122.4)).abs() < 0.001); // max_lon
        assert!((bounds.3 - 37.8).abs() < 0.001); // max_lat
    }

    #[test]
    fn test_calculate_bounds_single_path() {
        let paths = vec![PathOverlay {
            points: vec![
                GeoPoint {
                    lon: -122.4,
                    lat: 37.8,
                },
                GeoPoint {
                    lon: -122.5,
                    lat: 37.9,
                },
            ],
            stroke_color: Rgba([0, 0, 255, 255]),
            stroke_width: 3.0,
            fill_color: None,
        }];

        let bounds = calculate_bounds(&paths, &[]).unwrap();
        assert!((bounds.0 - (-122.5)).abs() < 0.001); // min_lon
        assert!((bounds.1 - 37.8).abs() < 0.001); // min_lat
        assert!((bounds.2 - (-122.4)).abs() < 0.001); // max_lon
        assert!((bounds.3 - 37.9).abs() < 0.001); // max_lat
    }

    #[test]
    fn test_calculate_bounds_paths_and_markers() {
        let paths = vec![PathOverlay {
            points: vec![
                GeoPoint { lon: 0.0, lat: 0.0 },
                GeoPoint {
                    lon: 10.0,
                    lat: 10.0,
                },
            ],
            stroke_color: Rgba([0, 0, 255, 255]),
            stroke_width: 3.0,
            fill_color: None,
        }];

        let markers = vec![MarkerOverlay {
            position: GeoPoint {
                lon: -5.0,
                lat: 15.0,
            },
            color: Rgba([255, 0, 0, 255]),
            label: None,
            size: 24.0,
        }];

        let bounds = calculate_bounds(&paths, &markers).unwrap();
        assert!((bounds.0 - (-5.0)).abs() < 0.001); // min_lon (from marker)
        assert!((bounds.1 - 0.0).abs() < 0.001); // min_lat (from path)
        assert!((bounds.2 - 10.0).abs() < 0.001); // max_lon (from path)
        assert!((bounds.3 - 15.0).abs() < 0.001); // max_lat (from marker)
    }

    #[test]
    fn test_calculate_bounds_empty_returns_none() {
        let bounds = calculate_bounds(&[], &[]);
        assert!(bounds.is_none());
    }

    #[test]
    fn test_calculate_bounds_global() {
        let markers = vec![
            MarkerOverlay {
                position: GeoPoint {
                    lon: -180.0,
                    lat: -90.0,
                },
                color: Rgba([255, 0, 0, 255]),
                label: None,
                size: 24.0,
            },
            MarkerOverlay {
                position: GeoPoint {
                    lon: 180.0,
                    lat: 90.0,
                },
                color: Rgba([0, 255, 0, 255]),
                label: None,
                size: 24.0,
            },
        ];

        let bounds = calculate_bounds(&[], &markers).unwrap();
        assert!((bounds.0 - (-180.0)).abs() < 0.001);
        assert!((bounds.1 - (-90.0)).abs() < 0.001);
        assert!((bounds.2 - 180.0).abs() < 0.001);
        assert!((bounds.3 - 90.0).abs() < 0.001);
    }

    // ============================================================
    // Geo to Pixel Conversion Tests
    // ============================================================

    #[test]
    fn test_geo_to_pixel_center() {
        // Center point should be at center of image
        let (px, py) = geo_to_pixel(
            &GeoPoint { lon: 0.0, lat: 0.0 },
            0.0, // center_lon
            0.0, // center_lat
            1.0, // zoom
            800, // width
            600, // height
            1.0, // scale
        );
        assert!((px - 400.0).abs() < 1.0); // Center x
        assert!((py - 300.0).abs() < 1.0); // Center y
    }

    #[test]
    fn test_geo_to_pixel_offset() {
        // Point east of center should have higher x
        let center = GeoPoint { lon: 0.0, lat: 0.0 };
        let east = GeoPoint { lon: 1.0, lat: 0.0 };

        let (cx, _) = geo_to_pixel(&center, 0.0, 0.0, 10.0, 800, 600, 1.0);
        let (ex, _) = geo_to_pixel(&east, 0.0, 0.0, 10.0, 800, 600, 1.0);

        assert!(ex > cx); // East point should be to the right
    }

    #[test]
    fn test_geo_to_pixel_scale() {
        // Higher scale should spread points further apart
        let point = GeoPoint { lon: 1.0, lat: 0.0 };

        let (px1, _) = geo_to_pixel(&point, 0.0, 0.0, 10.0, 800, 600, 1.0);
        let (px2, _) = geo_to_pixel(&point, 0.0, 0.0, 10.0, 800, 600, 2.0);

        // At 2x scale, pixel offset from center should be larger
        let offset1 = (px1 - 400.0).abs();
        let offset2 = (px2 - 400.0).abs();
        assert!(offset2 > offset1);
    }

    // ============================================================
    // Draw Overlays Smoke Tests
    // ============================================================

    #[test]
    fn test_draw_overlays_smoke() {
        // Just verify it doesn't panic
        let mut image = RgbaImage::new(256, 256);

        let paths = vec![PathOverlay {
            points: vec![
                GeoPoint {
                    lon: -122.4,
                    lat: 37.8,
                },
                GeoPoint {
                    lon: -122.5,
                    lat: 37.9,
                },
            ],
            stroke_color: Rgba([255, 0, 0, 255]),
            stroke_width: 3.0,
            fill_color: None,
        }];

        let markers = vec![MarkerOverlay {
            position: GeoPoint {
                lon: -122.45,
                lat: 37.85,
            },
            color: Rgba([0, 0, 255, 255]),
            label: None,
            size: 24.0,
        }];

        draw_overlays(&mut image, &paths, &markers, -122.45, 37.85, 12.0, 1.0);

        // Image should have some non-black pixels if overlays were drawn
        let has_colored_pixels = image
            .pixels()
            .any(|p| p.0[0] > 0 || p.0[1] > 0 || p.0[2] > 0);
        assert!(has_colored_pixels);
    }

    #[test]
    fn test_draw_overlays_empty() {
        // Should not panic with empty overlays
        let mut image = RgbaImage::new(256, 256);
        draw_overlays(&mut image, &[], &[], 0.0, 0.0, 10.0, 1.0);
    }

    #[test]
    fn test_blend_pixel() {
        let mut image = RgbaImage::new(10, 10);

        // Start with transparent black
        assert_eq!(image.get_pixel(5, 5).0, [0, 0, 0, 0]);

        // Blend a red pixel with full opacity
        blend_pixel(&mut image, 5, 5, Rgba([255, 0, 0, 255]));
        assert_eq!(image.get_pixel(5, 5).0, [255, 0, 0, 255]);

        // Blend a blue pixel at 50% opacity
        blend_pixel(&mut image, 5, 5, Rgba([0, 0, 255, 128]));
        let pixel = image.get_pixel(5, 5).0;
        // Should be purple-ish (mix of red and blue)
        assert!(pixel[0] > 100); // Some red
        assert!(pixel[2] > 50); // Some blue
    }

    // ============================================================
    // Google Polyline Encoding/Decoding Tests
    // ============================================================

    #[test]
    fn test_decode_polyline_google_example() {
        // Example from Google's documentation
        // https://developers.google.com/maps/documentation/utilities/polylinealgorithm
        // Encoded: "_p~iF~ps|U_ulLnnqC_mqNvxq`@"
        // Decodes to: (38.5, -120.2), (40.7, -120.95), (43.252, -126.453)
        let points = decode_polyline("_p~iF~ps|U_ulLnnqC_mqNvxq`@");

        assert_eq!(points.len(), 3);

        // First point: lat 38.5, lon -120.2
        assert!((points[0].lat - 38.5).abs() < 0.001);
        assert!((points[0].lon - (-120.2)).abs() < 0.001);

        // Second point: lat 40.7, lon -120.95
        assert!((points[1].lat - 40.7).abs() < 0.001);
        assert!((points[1].lon - (-120.95)).abs() < 0.001);

        // Third point: lat 43.252, lon -126.453
        assert!((points[2].lat - 43.252).abs() < 0.001);
        assert!((points[2].lon - (-126.453)).abs() < 0.001);
    }

    #[test]
    fn test_decode_polyline_simple() {
        // Simple two-point line
        // Encoding (0, 0) -> (1, 1)
        let encoded = encode_polyline(&[
            GeoPoint { lon: 0.0, lat: 0.0 },
            GeoPoint { lon: 1.0, lat: 1.0 },
        ]);
        let decoded = decode_polyline(&encoded);

        assert_eq!(decoded.len(), 2);
        assert!((decoded[0].lat - 0.0).abs() < 0.00001);
        assert!((decoded[0].lon - 0.0).abs() < 0.00001);
        assert!((decoded[1].lat - 1.0).abs() < 0.00001);
        assert!((decoded[1].lon - 1.0).abs() < 0.00001);
    }

    #[test]
    fn test_decode_polyline_negative_coords() {
        // Test with negative coordinates (San Francisco area)
        let points = vec![
            GeoPoint {
                lon: -122.4194,
                lat: 37.7749,
            },
            GeoPoint {
                lon: -122.4089,
                lat: 37.7849,
            },
        ];
        let encoded = encode_polyline(&points);
        let decoded = decode_polyline(&encoded);

        assert_eq!(decoded.len(), 2);
        assert!((decoded[0].lat - 37.7749).abs() < 0.00001);
        assert!((decoded[0].lon - (-122.4194)).abs() < 0.00001);
        assert!((decoded[1].lat - 37.7849).abs() < 0.00001);
        assert!((decoded[1].lon - (-122.4089)).abs() < 0.00001);
    }

    #[test]
    fn test_decode_polyline_empty() {
        let points = decode_polyline("");
        assert!(points.is_empty());
    }

    #[test]
    fn test_encode_polyline_roundtrip() {
        // Test that encoding and decoding produces the same result
        let original = vec![
            GeoPoint {
                lon: -122.4,
                lat: 37.8,
            },
            GeoPoint {
                lon: -122.5,
                lat: 37.9,
            },
            GeoPoint {
                lon: -122.6,
                lat: 38.0,
            },
            GeoPoint {
                lon: -122.7,
                lat: 38.1,
            },
        ];

        let encoded = encode_polyline(&original);
        let decoded = decode_polyline(&encoded);

        assert_eq!(decoded.len(), original.len());
        for (orig, dec) in original.iter().zip(decoded.iter()) {
            assert!((orig.lat - dec.lat).abs() < 0.00001);
            assert!((orig.lon - dec.lon).abs() < 0.00001);
        }
    }

    #[test]
    fn test_encode_polyline_precision() {
        // Test 5 decimal place precision (standard Google polyline)
        let points = vec![GeoPoint {
            lon: -122.12345,
            lat: 37.98765,
        }];
        let encoded = encode_polyline(&points);
        let decoded = decode_polyline(&encoded);

        assert_eq!(decoded.len(), 1);
        // Should be accurate to 5 decimal places
        assert!((decoded[0].lat - 37.98765).abs() < 0.00001);
        assert!((decoded[0].lon - (-122.12345)).abs() < 0.00001);
    }

    #[test]
    fn test_is_encoded_polyline() {
        // Should detect encoded polylines
        // Google example polyline (contains | which is valid in polyline encoding)
        assert!(is_encoded_polyline("_p~iF~ps|U_ulLnnqC_mqNvxq`@"));
        assert!(is_encoded_polyline("??")); // Simplest valid polyline (0,0)
        assert!(is_encoded_polyline("_ibE_seK_seK_seK")); // A simple path

        // Should reject coordinate strings (contains parseable numbers)
        assert!(!is_encoded_polyline("0,0|1,1"));
        assert!(!is_encoded_polyline("-122.4,37.8|-122.5,37.9"));
        assert!(!is_encoded_polyline(""));

        // Should reject strings with invalid polyline characters
        assert!(!is_encoded_polyline("hello world")); // Space is < 63
        assert!(!is_encoded_polyline("abc!def")); // ! is < 63
    }

    #[test]
    fn test_parse_path_with_encoded_polyline() {
        // Create a known encoded polyline for testing
        let points = vec![
            GeoPoint {
                lon: -122.4,
                lat: 37.8,
            },
            GeoPoint {
                lon: -122.5,
                lat: 37.9,
            },
        ];
        let encoded = encode_polyline(&points);

        // Parse with style prefix and encoded polyline
        let path_str = format!("path-5+f00({})", encoded);
        let path = parse_path(&path_str, false).unwrap();

        assert_eq!(path.points.len(), 2);
        assert_eq!(path.stroke_width, 5.0);
        assert_eq!(path.stroke_color, Rgba([255, 0, 0, 255]));
        assert!((path.points[0].lat - 37.8).abs() < 0.001);
        assert!((path.points[0].lon - (-122.4)).abs() < 0.001);
    }

    #[test]
    fn test_parse_path_enc_prefix() {
        // Test the enc: prefix format
        let points = vec![
            GeoPoint { lon: 0.0, lat: 0.0 },
            GeoPoint {
                lon: 10.0,
                lat: 10.0,
            },
        ];
        let encoded = encode_polyline(&points);
        let path_str = format!("enc:{}", encoded);

        let path = parse_path(&path_str, false).unwrap();
        assert_eq!(path.points.len(), 2);
        assert!((path.points[0].lat - 0.0).abs() < 0.001);
        assert!((path.points[1].lat - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_decode_polyline_long_route() {
        // Test a longer route with many points
        let original: Vec<GeoPoint> = (0..100)
            .map(|i| GeoPoint {
                lon: -122.0 + (i as f64 * 0.01),
                lat: 37.0 + (i as f64 * 0.01),
            })
            .collect();

        let encoded = encode_polyline(&original);
        let decoded = decode_polyline(&encoded);

        assert_eq!(decoded.len(), 100);
        for (orig, dec) in original.iter().zip(decoded.iter()) {
            assert!((orig.lat - dec.lat).abs() < 0.00001);
            assert!((orig.lon - dec.lon).abs() < 0.00001);
        }
    }

    #[test]
    fn test_decode_polyline_crossing_meridian() {
        // Test crossing the prime meridian and equator
        let original = vec![
            GeoPoint {
                lon: -1.0,
                lat: -1.0,
            },
            GeoPoint { lon: 0.0, lat: 0.0 },
            GeoPoint { lon: 1.0, lat: 1.0 },
        ];

        let encoded = encode_polyline(&original);
        let decoded = decode_polyline(&encoded);

        assert_eq!(decoded.len(), 3);
        assert!((decoded[1].lat - 0.0).abs() < 0.00001);
        assert!((decoded[1].lon - 0.0).abs() < 0.00001);
    }

    #[test]
    fn test_decode_polyline_extreme_coords() {
        // Test extreme coordinates (near poles)
        let original = vec![
            GeoPoint {
                lon: -180.0,
                lat: -85.0,
            },
            GeoPoint {
                lon: 180.0,
                lat: 85.0,
            },
        ];

        let encoded = encode_polyline(&original);
        let decoded = decode_polyline(&encoded);

        assert_eq!(decoded.len(), 2);
        assert!((decoded[0].lat - (-85.0)).abs() < 0.00001);
        assert!((decoded[0].lon - (-180.0)).abs() < 0.00001);
        assert!((decoded[1].lat - 85.0).abs() < 0.00001);
        assert!((decoded[1].lon - 180.0).abs() < 0.00001);
    }

    // ============================================================
    // Polygon Fill Tests
    // ============================================================

    #[test]
    fn test_fill_polygon_basic() {
        let mut img = RgbaImage::new(100, 100);
        let pixels = vec![(10.0f32, 10.0), (90.0, 10.0), (50.0, 90.0)];
        fill_polygon(&mut img, &pixels, Rgba([255u8, 0, 0, 255]));
        let center = img.get_pixel(50, 40);
        assert!(center[3] > 0, "interior pixel should be filled");
        assert_eq!(center[0], 255, "R channel should be 255");
    }

    #[test]
    fn test_fill_polygon_auto_close() {
        let mut img = RgbaImage::new(100, 100);
        let pixels = vec![(10.0f32, 10.0), (90.0, 10.0), (50.0, 90.0)];
        fill_polygon(&mut img, &pixels, Rgba([0u8, 255, 0, 255]));
        let p = img.get_pixel(50, 50);
        assert!(p[3] > 0, "pixel should be written even for open polygon");
    }

    #[test]
    fn test_fill_polygon_noop_fewer_than_3_points() {
        let mut img = RgbaImage::new(100, 100);
        let before: Vec<_> = img.pixels().copied().collect();
        fill_polygon(
            &mut img,
            &[(10.0, 10.0), (90.0, 90.0)],
            Rgba([255, 0, 0, 255]),
        );
        let after: Vec<_> = img.pixels().copied().collect();
        assert_eq!(before, after, "image must not change for < 3 points");
    }

    #[test]
    fn test_draw_path_with_fill_modifies_image() {
        let path = PathOverlay {
            points: vec![
                GeoPoint {
                    lon: -0.01,
                    lat: -0.01,
                },
                GeoPoint {
                    lon: 0.01,
                    lat: -0.01,
                },
                GeoPoint {
                    lon: 0.0,
                    lat: 0.01,
                },
            ],
            stroke_color: Rgba([0, 0, 255, 255]),
            stroke_width: 2.0,
            fill_color: Some(Rgba([255, 0, 0, 128])),
        };
        let mut img = RgbaImage::new(512, 512);
        draw_path(&mut img, &path, 0.0, 0.0, 14.0, 512, 512, 1.0);
        let any_written = img.pixels().any(|p| p[3] > 0);
        assert!(any_written, "at least some pixels must be written");
    }

    #[test]
    fn test_parse_path_latlng_swaps_coordinates() {
        let path = parse_path("10.0,20.0|30.0,40.0", true).unwrap();
        assert!(
            (path.points[0].lon - 20.0).abs() < 0.001,
            "lon should be second value"
        );
        assert!(
            (path.points[0].lat - 10.0).abs() < 0.001,
            "lat should be first value"
        );
    }

    #[test]
    fn test_parse_path_latlng_false_preserves_order() {
        let path = parse_path("10.0,20.0|30.0,40.0", false).unwrap();
        assert!(
            (path.points[0].lon - 10.0).abs() < 0.001,
            "lon should be first value"
        );
        assert!(
            (path.points[0].lat - 20.0).abs() < 0.001,
            "lat should be second value"
        );
    }

    // ============================================================
    // GeoJSON Overlay Tests
    // ============================================================

    #[test]
    fn test_parse_geojson_point_feature() {
        let geojson = r#"{
            "type": "Feature",
            "geometry": {"type": "Point", "coordinates": [-122.4, 37.8]},
            "properties": {}
        }"#;
        let (paths, markers) = parse_geojson(geojson);
        assert!(paths.is_empty());
        assert_eq!(markers.len(), 1);
        assert!((markers[0].position.lon - (-122.4)).abs() < 1e-9);
        assert!((markers[0].position.lat - 37.8).abs() < 1e-9);
    }

    #[test]
    fn test_parse_geojson_linestring_with_stroke() {
        let geojson = r#"{
            "type": "Feature",
            "geometry": {
                "type": "LineString",
                "coordinates": [[-122.4,37.8],[-122.5,37.9],[-122.6,38.0]]
            },
            "properties": {"stroke": "ff0000", "stroke-width": 5}
        }"#;
        let (paths, markers) = parse_geojson(geojson);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].points.len(), 3);
        assert_eq!(paths[0].stroke_color, Rgba([255, 0, 0, 255]));
        assert!((paths[0].stroke_width - 5.0).abs() < f32::EPSILON);
        assert!(markers.is_empty());
    }

    #[test]
    fn test_parse_geojson_polygon_with_fill() {
        let geojson = r#"{
            "type": "Feature",
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[-1.0,-1.0],[1.0,-1.0],[0.0,1.0],[-1.0,-1.0]]]
            },
            "properties": {"fill": "00ff0080"}
        }"#;
        let (paths, markers) = parse_geojson(geojson);
        assert_eq!(paths.len(), 1);
        let fill = paths[0].fill_color.expect("fill_color should be set");
        assert_eq!(fill[1], 255, "G channel should be 255 (green)");
        assert_eq!(fill[3], 128, "alpha should be 128");
        assert!(markers.is_empty());
    }

    #[test]
    fn test_parse_geojson_feature_collection() {
        let geojson = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {"type": "Point", "coordinates": [0.0, 0.0]},
                    "properties": {}
                },
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [[0.0,0.0],[1.0,1.0]]
                    },
                    "properties": {}
                }
            ]
        }"#;
        let (paths, markers) = parse_geojson(geojson);
        assert_eq!(markers.len(), 1);
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn test_parse_geojson_invalid_returns_empty() {
        let (paths, markers) = parse_geojson("not json at all {{{");
        assert!(paths.is_empty());
        assert!(markers.is_empty());
    }

    #[test]
    fn test_parse_geojson_bare_geometry() {
        let geojson = r#"{"type":"Point","coordinates":[-74.0,40.7]}"#;
        let (paths, markers) = parse_geojson(geojson);
        assert!(paths.is_empty());
        assert_eq!(markers.len(), 1);
        assert!((markers[0].position.lon - (-74.0)).abs() < 1e-9);
    }

    #[test]
    fn test_parse_geojson_multipoint() {
        let geojson = r##"{
            "type": "Feature",
            "geometry": {
                "type": "MultiPoint",
                "coordinates": [[-74.0, 40.7], [-73.9, 40.8], [-74.1, 40.6]]
            },
            "properties": {"marker-color": "#00ff00"}
        }"##;
        let (paths, markers) = parse_geojson(geojson);
        assert!(paths.is_empty());
        assert_eq!(markers.len(), 3);
        assert!((markers[0].position.lon - (-74.0)).abs() < 1e-9);
        assert!((markers[1].position.lon - (-73.9)).abs() < 1e-9);
        assert!((markers[2].position.lon - (-74.1)).abs() < 1e-9);
        assert_eq!(markers[0].color, Rgba([0, 255, 0, 255]));
    }

    #[test]
    fn test_parse_geojson_multilinestring() {
        let geojson = r#"{
            "type": "MultiLineString",
            "coordinates": [
                [[0.0, 0.0], [1.0, 1.0]],
                [[2.0, 2.0], [3.0, 3.0], [4.0, 4.0]]
            ]
        }"#;
        let (paths, markers) = parse_geojson(geojson);
        assert!(markers.is_empty());
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].points.len(), 2);
        assert_eq!(paths[1].points.len(), 3);
    }

    #[test]
    fn test_parse_geojson_multipolygon() {
        let geojson = r##"{
            "type": "Feature",
            "geometry": {
                "type": "MultiPolygon",
                "coordinates": [
                    [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,0.0]]],
                    [[[2.0,2.0],[3.0,2.0],[3.0,3.0],[2.0,2.0]]]
                ]
            },
            "properties": {"fill": "#ff0000"}
        }"##;
        let (paths, markers) = parse_geojson(geojson);
        assert!(markers.is_empty());
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].fill_color, Some(Rgba([255, 0, 0, 255])));
        assert_eq!(paths[1].fill_color, Some(Rgba([255, 0, 0, 255])));
    }

    #[test]
    fn test_parse_geojson_marker_size_string() {
        for (size_str, expected_px) in [("small", 8.0_f32), ("medium", 12.0), ("large", 18.0)] {
            let geojson = format!(
                r#"{{"type":"Feature","geometry":{{"type":"Point","coordinates":[0,0]}},"properties":{{"marker-size":"{}"}}}}"#,
                size_str
            );
            let (_, markers) = parse_geojson(&geojson);
            assert_eq!(markers.len(), 1, "failed for marker-size={size_str}");
            assert!(
                (markers[0].size - expected_px).abs() < 1e-3,
                "marker-size=\"{size_str}\" should map to {expected_px}px, got {}",
                markers[0].size
            );
        }
    }

    #[test]
    fn test_parse_geojson_marker_size_numeric() {
        let geojson = r#"{"type":"Feature","geometry":{"type":"Point","coordinates":[0,0]},"properties":{"marker-size":24}}"#;
        let (_, markers) = parse_geojson(geojson);
        assert_eq!(markers.len(), 1);
        assert!((markers[0].size - 24.0).abs() < 1e-3);
    }
}
