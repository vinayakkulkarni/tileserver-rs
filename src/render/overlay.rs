//! Overlay drawing for static map images
//!
//! Supports drawing paths (polylines) and markers on rendered map images.

use image::{Rgba, RgbaImage};

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
    /// Fill color (RGBA) - for closed polygons (reserved for future use)
    #[allow(dead_code)]
    pub fill_color: Option<Rgba<u8>>,
}

/// A marker overlay to draw on the map
#[derive(Debug, Clone)]
pub struct MarkerOverlay {
    /// Position of the marker
    pub position: GeoPoint,
    /// Marker color (RGBA)
    pub color: Rgba<u8>,
    /// Optional label text (reserved for future use)
    #[allow(dead_code)]
    pub label: Option<String>,
    /// Marker size in pixels
    pub size: f32,
}

/// Parse a path string into a PathOverlay
///
/// Format: `path-{strokeWidth}+{strokeColor}-{fillColor}({coordinates})`
/// Example: `path-5+f00-88f(0,0|10,10|20,0)`
/// Or encoded polyline: `path-5+f00({encoded})`
pub fn parse_path(path_str: &str) -> Option<PathOverlay> {
    // Default values
    let mut stroke_width = 3.0f32;
    let mut stroke_color = Rgba([0, 0, 255, 255]); // Blue
    let mut fill_color: Option<Rgba<u8>> = None;
    let mut points = Vec::new();

    // Parse the path format
    let path_str = path_str.trim();

    // Try to parse path-{width}+{color}(-{fill})({coords}) format
    if let Some(rest) = path_str.strip_prefix("path-") {
        // Find the opening parenthesis for coordinates
        if let Some(paren_idx) = rest.find('(') {
            let style_part = &rest[..paren_idx];
            let coords_part = &rest[paren_idx + 1..rest.len() - 1]; // Remove ( and )

            // Parse style: width+color or width+color-fill
            let parts: Vec<&str> = style_part.split('+').collect();
            if !parts.is_empty() {
                stroke_width = parts[0].parse().unwrap_or(3.0);
            }
            if parts.len() > 1 {
                // Check for fill color
                let color_parts: Vec<&str> = parts[1].split('-').collect();
                stroke_color = parse_hex_color(color_parts[0]).unwrap_or(stroke_color);
                if color_parts.len() > 1 {
                    fill_color = parse_hex_color(color_parts[1]);
                }
            }

            // Parse coordinates: lon,lat|lon,lat|...
            for coord in coords_part.split('|') {
                let xy: Vec<&str> = coord.split(',').collect();
                if xy.len() >= 2 {
                    if let (Ok(lon), Ok(lat)) = (xy[0].parse(), xy[1].parse()) {
                        points.push(GeoPoint { lon, lat });
                    }
                }
            }
        }
    } else {
        // Simple format: just coordinates separated by |
        for coord in path_str.split('|') {
            let xy: Vec<&str> = coord.split(',').collect();
            if xy.len() >= 2 {
                if let (Ok(lon), Ok(lat)) = (xy[0].parse(), xy[1].parse()) {
                    points.push(GeoPoint { lon, lat });
                }
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
            if xy.len() >= 2 {
                if let (Ok(lon), Ok(lat)) = (xy[0].parse(), xy[1].parse()) {
                    return Some(MarkerOverlay {
                        position: GeoPoint { lon, lat },
                        color,
                        label,
                        size,
                    });
                }
            }
        }
    } else {
        // Simple format: lon,lat
        let xy: Vec<&str> = marker_str.split(',').collect();
        if xy.len() >= 2 {
            if let (Ok(lon), Ok(lat)) = (xy[0].parse(), xy[1].parse()) {
                return Some(MarkerOverlay {
                    position: GeoPoint { lon, lat },
                    color,
                    label: None,
                    size,
                });
            }
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

    // Convert all points to pixel coordinates
    let pixels: Vec<(f32, f32)> = path
        .points
        .iter()
        .map(|p| geo_to_pixel(p, center_lon, center_lat, zoom, width, height, scale))
        .collect();

    // Draw line segments
    let stroke_width = path.stroke_width * scale;
    for i in 0..pixels.len() - 1 {
        draw_line(
            image,
            pixels[i].0,
            pixels[i].1,
            pixels[i + 1].0,
            pixels[i + 1].1,
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
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("f00"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(parse_hex_color("ff0000"), Some(Rgba([255, 0, 0, 255])));
        assert_eq!(parse_hex_color("ff000080"), Some(Rgba([255, 0, 0, 128])));
        assert_eq!(parse_hex_color("#00f"), Some(Rgba([0, 0, 255, 255])));
    }

    #[test]
    fn test_parse_path() {
        let path = parse_path("path-5+f00(-122.4,37.8|-122.5,37.9)").unwrap();
        assert_eq!(path.points.len(), 2);
        assert_eq!(path.stroke_width, 5.0);
        assert_eq!(path.stroke_color, Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn test_parse_marker() {
        let marker = parse_marker("pin-s+f00(-122.4,37.8)").unwrap();
        assert_eq!(marker.position.lon, -122.4);
        assert_eq!(marker.position.lat, 37.8);
        assert_eq!(marker.color, Rgba([255, 0, 0, 255]));
    }
}
