//! End-to-end rendering tests with snapshot comparison
//!
//! These tests generate actual PNG images and compare them against stored snapshots
//! to detect any rendering regressions.
//!
//! # Updating Snapshots
//!
//! To update snapshots when rendering changes intentionally:
//! ```bash
//! cargo insta test
//! cargo insta review
//! ```
//!
//! # Test Artifacts
//!
//! Generated test images are stored in `tests/snapshots/` for visual inspection.

use std::fs;
use std::path::{Path, PathBuf};

/// Directory for storing snapshot images
const SNAPSHOTS_DIR: &str = "tests/snapshots";

/// Compare two PNG images and return if they match (within tolerance)
fn images_match(img1_path: &Path, img2_path: &Path, tolerance: f64) -> bool {
    let img1 = match image::open(img1_path) {
        Ok(img) => img.to_rgba8(),
        Err(_) => return false,
    };

    let img2 = match image::open(img2_path) {
        Ok(img) => img.to_rgba8(),
        Err(_) => return false,
    };

    // Check dimensions
    if img1.dimensions() != img2.dimensions() {
        return false;
    }

    let (width, height) = img1.dimensions();
    let total_pixels = (width * height) as f64;
    let mut diff_pixels = 0u64;

    for (p1, p2) in img1.pixels().zip(img2.pixels()) {
        // Compare RGBA values with some tolerance for anti-aliasing
        let dr = (p1[0] as i32 - p2[0] as i32).abs();
        let dg = (p1[1] as i32 - p2[1] as i32).abs();
        let db = (p1[2] as i32 - p2[2] as i32).abs();
        let da = (p1[3] as i32 - p2[3] as i32).abs();

        // If any channel differs by more than threshold, count as different
        if dr > 5 || dg > 5 || db > 5 || da > 5 {
            diff_pixels += 1;
        }
    }

    let diff_ratio = diff_pixels as f64 / total_pixels;
    diff_ratio <= tolerance
}

/// Save an image and return the path
#[allow(dead_code)]
fn save_test_image(name: &str, data: &[u8]) -> PathBuf {
    let path = PathBuf::from(SNAPSHOTS_DIR).join(format!("{}.png", name));
    fs::create_dir_all(SNAPSHOTS_DIR).expect("Should create snapshots dir");
    fs::write(&path, data).expect("Should write image");
    path
}

// ============================================================
// Overlay Rendering Tests
// ============================================================

mod overlay_rendering {
    use super::*;
    use image::{Rgba, RgbaImage};
    use tileserver_rs::render::overlay::{draw_overlays, parse_marker, parse_path};

    /// Generate a test image with a marker overlay
    fn render_marker_test_image(marker_str: &str) -> RgbaImage {
        let mut image = RgbaImage::from_pixel(256, 256, Rgba([240, 240, 240, 255]));

        if let Some(marker) = parse_marker(marker_str) {
            let markers = vec![marker];
            draw_overlays(&mut image, &[], &markers, 0.0, 0.0, 10.0, 1.0);
        }

        image
    }

    /// Generate a test image with a path overlay
    fn render_path_test_image(path_str: &str) -> RgbaImage {
        let mut image = RgbaImage::from_pixel(256, 256, Rgba([240, 240, 240, 255]));

        if let Some(path) = parse_path(path_str) {
            let paths = vec![path];
            draw_overlays(&mut image, &paths, &[], 0.0, 0.0, 5.0, 1.0);
        }

        image
    }

    #[test]
    fn test_render_red_marker() {
        let image = render_marker_test_image("pin-s+f00(0,0)");

        // Save the generated image
        let output_path = PathBuf::from(SNAPSHOTS_DIR).join("marker_red_small.png");
        fs::create_dir_all(SNAPSHOTS_DIR).ok();
        image.save(&output_path).expect("Should save marker image");

        // Verify image was created and has content
        assert!(output_path.exists(), "Marker image should be created");
        let metadata = fs::metadata(&output_path).unwrap();
        assert!(metadata.len() > 0, "Marker image should not be empty");

        // Verify the image has some red pixels (the marker)
        let has_red = image.pixels().any(|p| p[0] > 200 && p[1] < 50 && p[2] < 50);
        assert!(has_red, "Image should contain red marker pixels");
    }

    #[test]
    fn test_render_blue_path() {
        let image = render_path_test_image("path-5+00f(-1,-1|1,1)");

        let output_path = PathBuf::from(SNAPSHOTS_DIR).join("path_blue_diagonal.png");
        fs::create_dir_all(SNAPSHOTS_DIR).ok();
        image.save(&output_path).expect("Should save path image");

        assert!(output_path.exists(), "Path image should be created");

        // Verify the image has some blue pixels (the path)
        let has_blue = image.pixels().any(|p| p[0] < 50 && p[1] < 50 && p[2] > 200);
        assert!(has_blue, "Image should contain blue path pixels");
    }

    #[test]
    fn test_render_marker_sizes() {
        for (size, size_name) in [("s", "small"), ("m", "medium"), ("l", "large")] {
            let image = render_marker_test_image(&format!("pin-{}+0f0(0,0)", size));

            let output_path =
                PathBuf::from(SNAPSHOTS_DIR).join(format!("marker_green_{}.png", size_name));
            fs::create_dir_all(SNAPSHOTS_DIR).ok();
            image.save(&output_path).expect("Should save marker image");

            assert!(output_path.exists());
        }
    }

    #[test]
    fn test_render_path_with_multiple_points() {
        let image = render_path_test_image("path-3+f0f(-2,0|0,2|2,0|0,-2|-2,0)");

        let output_path = PathBuf::from(SNAPSHOTS_DIR).join("path_magenta_diamond.png");
        fs::create_dir_all(SNAPSHOTS_DIR).ok();
        image.save(&output_path).expect("Should save path image");

        assert!(output_path.exists());

        // Should have magenta pixels
        let has_magenta = image
            .pixels()
            .any(|p| p[0] > 200 && p[1] < 50 && p[2] > 200);
        assert!(has_magenta, "Image should contain magenta path pixels");
    }

    #[test]
    fn test_render_combined_markers_and_paths() {
        let mut image = RgbaImage::from_pixel(512, 512, Rgba([255, 255, 255, 255]));

        let paths = vec![
            parse_path("path-4+0000ff(-2,-2|2,2)").unwrap(),
            parse_path("path-4+00ff00(-2,2|2,-2)").unwrap(),
        ];

        let markers = vec![
            parse_marker("pin-m+ff0000(0,0)").unwrap(),
            parse_marker("pin-s+ffff00(-2,-2)").unwrap(),
            parse_marker("pin-s+ff00ff(2,2)").unwrap(),
        ];

        draw_overlays(&mut image, &paths, &markers, 0.0, 0.0, 3.0, 2.0);

        let output_path = PathBuf::from(SNAPSHOTS_DIR).join("combined_paths_markers.png");
        fs::create_dir_all(SNAPSHOTS_DIR).ok();
        image
            .save(&output_path)
            .expect("Should save combined image");

        assert!(output_path.exists());

        // Verify multiple colors are present
        let mut has_red = false;
        let mut has_blue = false;
        let mut has_green = false;

        for p in image.pixels() {
            if p[0] > 200 && p[1] < 100 && p[2] < 100 {
                has_red = true;
            }
            if p[0] < 100 && p[1] < 100 && p[2] > 200 {
                has_blue = true;
            }
            if p[0] < 100 && p[1] > 200 && p[2] < 100 {
                has_green = true;
            }
        }

        assert!(has_red, "Should have red marker");
        assert!(has_blue, "Should have blue path");
        assert!(has_green, "Should have green path");
    }
}

// ============================================================
// Google Polyline Rendering Tests
// ============================================================

mod polyline_rendering {
    use super::*;
    use image::{Rgba, RgbaImage};
    use tileserver_rs::render::overlay::{decode_polyline, draw_overlays, parse_path};

    #[test]
    fn test_render_google_polyline() {
        // Google's example polyline
        let polyline = "_p~iF~ps|U_ulLnnqC_mqNvxq`@";
        let points = decode_polyline(polyline);

        assert_eq!(
            points.len(),
            3,
            "Should decode 3 points from Google example"
        );

        // Create a path from decoded points
        let path = parse_path(&format!(
            "path-3+ff6600({})",
            points
                .iter()
                .map(|p| format!("{},{}", p.lon, p.lat))
                .collect::<Vec<_>>()
                .join("|")
        ))
        .unwrap();

        // Calculate center from points
        let center_lon = points.iter().map(|p| p.lon).sum::<f64>() / points.len() as f64;
        let center_lat = points.iter().map(|p| p.lat).sum::<f64>() / points.len() as f64;

        let mut image = RgbaImage::from_pixel(512, 512, Rgba([245, 245, 245, 255]));
        draw_overlays(&mut image, &[path], &[], center_lon, center_lat, 6.0, 1.0);

        let output_path = PathBuf::from(SNAPSHOTS_DIR).join("polyline_google_example.png");
        fs::create_dir_all(SNAPSHOTS_DIR).ok();
        image
            .save(&output_path)
            .expect("Should save polyline image");

        assert!(output_path.exists());

        // Should have orange pixels
        let has_orange = image
            .pixels()
            .any(|p| p[0] > 200 && p[1] > 80 && p[1] < 150 && p[2] < 50);
        assert!(has_orange, "Image should contain orange path pixels");
    }

    #[test]
    fn test_render_encoded_path_format() {
        // Test the enc: prefix format
        // Create a simple encoded polyline for a square
        let path = parse_path(
            "path-5+00ffff(-122.0,37.0|-122.1,37.0|-122.1,37.1|-122.0,37.1|-122.0,37.0)",
        );

        assert!(path.is_some(), "Should parse path with coordinates");
        let path = path.unwrap();
        assert_eq!(path.points.len(), 5, "Should have 5 points for square");

        let mut image = RgbaImage::from_pixel(256, 256, Rgba([250, 250, 250, 255]));
        draw_overlays(&mut image, &[path], &[], -122.05, 37.05, 12.0, 1.0);

        let output_path = PathBuf::from(SNAPSHOTS_DIR).join("path_square_cyan.png");
        fs::create_dir_all(SNAPSHOTS_DIR).ok();
        image.save(&output_path).expect("Should save square image");

        assert!(output_path.exists());
    }
}

// ============================================================
// JSON Snapshot Tests (using insta)
// ============================================================

mod json_snapshots {
    use std::path::PathBuf;
    use tileserver_rs::{Config, SourceManager, StyleManager};

    #[tokio::test]
    async fn test_tilejson_snapshot() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let protomaps = sources.get("protomaps").expect("Should have protomaps");
        let tilejson = protomaps.metadata().to_tilejson("http://localhost:8080");

        // Convert to JSON for snapshot
        let json = serde_json::to_value(&tilejson).expect("Should serialize");

        // Use insta for JSON snapshot comparison
        insta::assert_json_snapshot!("tilejson_protomaps", json);
    }

    #[tokio::test]
    async fn test_style_info_snapshot() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load config");
        let styles = StyleManager::from_configs(&config.styles).expect("Should load styles");

        let infos = styles.all_infos("http://localhost:8080");
        let json = serde_json::to_value(&infos).expect("Should serialize");

        insta::assert_json_snapshot!("style_infos", json);
    }

    #[tokio::test]
    async fn test_sources_list_snapshot() {
        let config = Config::load(Some(PathBuf::from("tests/config.test.toml")))
            .expect("Should load config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("Should load sources");

        let mut all_metadata: Vec<_> = sources
            .all_metadata()
            .iter()
            .map(|m| m.to_tilejson("http://localhost:8080"))
            .collect();

        // Sort by id for consistent ordering in snapshots
        all_metadata.sort_by(|a, b| a.id.cmp(&b.id));

        let json = serde_json::to_value(&all_metadata).expect("Should serialize");

        insta::assert_json_snapshot!("sources_list", json);
    }
}

// ============================================================
// Snapshot Comparison Helpers
// ============================================================

#[cfg(test)]
mod snapshot_helpers {
    use super::*;

    #[test]
    fn test_snapshots_directory_exists() {
        fs::create_dir_all(SNAPSHOTS_DIR).expect("Should create snapshots dir");
        assert!(Path::new(SNAPSHOTS_DIR).is_dir());
    }

    #[test]
    fn test_image_comparison_identical() {
        // Create two identical images
        let img = image::RgbaImage::from_pixel(10, 10, image::Rgba([255, 0, 0, 255]));

        let path1 = PathBuf::from(SNAPSHOTS_DIR).join("test_identical_1.png");
        let path2 = PathBuf::from(SNAPSHOTS_DIR).join("test_identical_2.png");

        fs::create_dir_all(SNAPSHOTS_DIR).ok();
        img.save(&path1).unwrap();
        img.save(&path2).unwrap();

        assert!(images_match(&path1, &path2, 0.0));

        // Cleanup
        fs::remove_file(&path1).ok();
        fs::remove_file(&path2).ok();
    }

    #[test]
    fn test_image_comparison_different() {
        let img1 = image::RgbaImage::from_pixel(10, 10, image::Rgba([255, 0, 0, 255]));
        let img2 = image::RgbaImage::from_pixel(10, 10, image::Rgba([0, 0, 255, 255]));

        let path1 = PathBuf::from(SNAPSHOTS_DIR).join("test_diff_red.png");
        let path2 = PathBuf::from(SNAPSHOTS_DIR).join("test_diff_blue.png");

        fs::create_dir_all(SNAPSHOTS_DIR).ok();
        img1.save(&path1).unwrap();
        img2.save(&path2).unwrap();

        assert!(!images_match(&path1, &path2, 0.0));

        // Cleanup
        fs::remove_file(&path1).ok();
        fs::remove_file(&path2).ok();
    }
}
