//! Shared tile-path layout parsing for filesystem-style sources (`dir`, `tar`).
//!
//! Both sources address tiles by substituting `{z}`/`{x}`/`{y}` (and an
//! optional `{ext}`) into a template such as `{z}/{x}/{y}.pbf`. This module
//! centralises template parsing, path rendering, and the helpers shared between
//! the two sources so the layout grammar stays consistent.

use std::path::Path;

use crate::error::{Result, TileServerError};

/// Default tile-path template when the operator does not specify one.
pub const DEFAULT_TEMPLATE: &str = "{z}/{x}/{y}.{ext}";

/// A parsed tile-path template.
///
/// The template is split into a prefix/middle/suffix around the `{z}`, `{x}`,
/// and `{y}` placeholders so rendering is a handful of string pushes rather
/// than repeated `replace` allocations on the hot path.
#[derive(Debug, Clone)]
pub struct TileLayout {
    template: String,
    /// Literal extension when the template ends in `.ext` with a concrete
    /// extension (e.g. `pbf` from `{z}/{x}/{y}.pbf`). `None` when the template
    /// uses the `{ext}` placeholder.
    extension: Option<String>,
}

impl TileLayout {
    /// Parse a template, falling back to [`DEFAULT_TEMPLATE`] when `None`.
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::ConfigError`] when the template is missing any
    /// of the required `{z}`, `{x}`, or `{y}` placeholders.
    pub fn parse(template: Option<&str>) -> Result<Self> {
        let template = template.unwrap_or(DEFAULT_TEMPLATE).to_string();
        for placeholder in ["{z}", "{x}", "{y}"] {
            if !template.contains(placeholder) {
                return Err(TileServerError::ConfigError(format!(
                    "tile_path_template must contain {placeholder}: got `{template}`"
                )));
            }
        }

        let extension = if template.contains("{ext}") {
            None
        } else {
            template.rsplit_once('.').map(|(_, ext)| ext.to_string())
        };

        Ok(Self {
            template,
            extension,
        })
    }

    /// The fixed extension baked into the template, if any.
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }

    /// Render the relative tile path for the given coordinates.
    ///
    /// The `{ext}` placeholder (when present) is replaced with the literal
    /// `ext` so callers can supply the format-derived extension.
    #[must_use]
    pub fn render_with_ext(&self, z: u8, x: u32, y: u32, ext: &str) -> String {
        self.template
            .replace("{z}", &z.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string())
            .replace("{ext}", ext)
    }

    /// Render the relative tile path, using the template's own extension (or
    /// `pbf` when the template uses `{ext}` without a known format).
    #[must_use]
    pub fn render(&self, z: u8, x: u32, y: u32) -> String {
        let ext = self.extension.as_deref().unwrap_or("pbf");
        self.render_with_ext(z, x, y, ext)
    }
}

/// Flip the Y coordinate between XYZ (north-up) and TMS (south-up) at zoom `z`.
#[must_use]
pub fn flip_y(z: u8, y: u32) -> u32 {
    (1u32 << z) - 1 - y
}

/// Probe a directory pyramid for the first tile file and return its extension.
///
/// Walks `<base>/<z>/<x>/<first file>` and returns the extension of the first
/// tile encountered, used to auto-detect the served format when neither
/// `serve_as` nor a template extension is supplied.
#[must_use]
pub fn detect_extension(base: &Path) -> Option<String> {
    let z_dir = first_numeric_subdir(base)?;
    let x_dir = first_numeric_subdir(&z_dir)?;
    let entries = std::fs::read_dir(&x_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some((_, ext)) = name.rsplit_once('.') {
            return Some(ext.to_string());
        }
    }
    None
}

/// Return the first subdirectory whose name parses as a non-negative integer.
fn first_numeric_subdir(dir: &Path) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut numeric: Vec<std::path::PathBuf> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter(|e| e.file_name().to_string_lossy().parse::<u32>().is_ok())
        .map(|e| e.path())
        .collect();
    numeric.sort();
    numeric.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_template() {
        let layout = TileLayout::parse(None).unwrap();
        assert_eq!(layout.render(3, 4, 5), "3/4/5.pbf");
        assert_eq!(layout.extension(), None);
    }

    #[test]
    fn parse_literal_extension() {
        let layout = TileLayout::parse(Some("{z}/{x}/{y}.png")).unwrap();
        assert_eq!(layout.extension(), Some("png"));
        assert_eq!(layout.render(1, 0, 0), "1/0/0.png");
    }

    #[test]
    fn parse_retina_template() {
        let layout = TileLayout::parse(Some("{z}/{x}/{y}@2x.webp")).unwrap();
        assert_eq!(layout.render(2, 1, 3), "2/1/3@2x.webp");
        assert_eq!(layout.extension(), Some("webp"));
    }

    #[test]
    fn parse_ext_placeholder_has_no_fixed_extension() {
        let layout = TileLayout::parse(Some("{z}/{x}/{y}.{ext}")).unwrap();
        assert_eq!(layout.extension(), None);
        assert_eq!(layout.render_with_ext(1, 0, 0, "jpg"), "1/0/0.jpg");
    }

    #[test]
    fn parse_rejects_missing_placeholder() {
        let err = TileLayout::parse(Some("{z}/{x}.pbf")).unwrap_err();
        assert!(matches!(err, TileServerError::ConfigError(_)));
    }

    #[test]
    fn flip_y_is_involutive() {
        for z in 0..=5u8 {
            let max = 1u32 << z;
            for y in 0..max {
                assert_eq!(flip_y(z, flip_y(z, y)), y);
            }
        }
    }

    #[test]
    fn flip_y_z2_known_values() {
        assert_eq!(flip_y(2, 0), 3);
        assert_eq!(flip_y(2, 3), 0);
    }
}
