//! Directory-of-tiles source: serves `{z}/{x}/{y}.{ext}` files straight from
//! disk with zero startup index cost.
//!
//! Each tile fetch is a direct filesystem read, making this the lowest-friction
//! way to serve tippecanoe `--output-to-directory` output, `wget` mirrors, or
//! any on-disk tile pyramid. Metadata (`minzoom`/`maxzoom`/`bounds`) is derived
//! once by walking the directory the first time it is requested and then cached.

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::config::SourceConfig;
use crate::error::{Result, TileServerError};
use crate::sources::tile_layout::{TileLayout, detect_extension, flip_y};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

/// Directory-of-tiles source.
#[derive(Debug)]
pub struct DirSource {
    /// Base directory containing the `{z}/{x}/{y}.{ext}` pyramid.
    base: PathBuf,
    /// Parsed tile-path layout (placeholders + extension).
    layout: TileLayout,
    /// TMS (south-up) addressing when `true`, XYZ otherwise.
    tms: bool,
    /// Eagerly-resolved metadata. `bounds`/`minzoom`/`maxzoom` come from a
    /// one-time directory walk performed lazily on first metadata access.
    metadata: TileMetadata,
    /// Lazily-computed coverage, populated on the first TileJSON request.
    coverage: Arc<OnceCell<Coverage>>,
}

/// Coverage derived by walking the on-disk pyramid once.
#[derive(Debug, Clone)]
struct Coverage {
    minzoom: u8,
    maxzoom: u8,
    bounds: Option<[f64; 4]>,
}

impl DirSource {
    /// Build a directory source from operator config.
    ///
    /// # Errors
    ///
    /// Returns [`TileServerError::FileError`] if the path does not exist or is
    /// not a directory, or [`TileServerError::ConfigError`] if the
    /// `tile_path_template` is malformed.
    pub async fn from_file(config: &SourceConfig) -> Result<Self> {
        let base = PathBuf::from(&config.path);
        if !base.is_dir() {
            return Err(TileServerError::FileError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("tile directory not found: {}", config.path),
            )));
        }

        let layout = TileLayout::parse(config.tile_path_template.as_deref())?;

        let format = resolve_format(config, &base, &layout).await;

        let metadata = TileMetadata {
            id: config.id.clone(),
            name: config.name.clone().unwrap_or_else(|| config.id.clone()),
            description: config.description.clone(),
            attribution: config.attribution.clone(),
            format,
            minzoom: config.minzoom.unwrap_or(0),
            maxzoom: config.maxzoom.unwrap_or(22),
            bounds: None,
            center: None,
            vector_layers: None,
        };

        tracing::info!(
            "Loaded directory source '{}' at {} ({} scheme)",
            config.id,
            config.path,
            if config.tms { "TMS" } else { "XYZ" }
        );

        Ok(Self {
            base,
            layout,
            tms: config.tms,
            metadata,
            coverage: Arc::new(OnceCell::new()),
        })
    }

    /// Resolve the on-disk path for a tile, applying TMS flipping when enabled.
    fn tile_path(&self, z: u8, x: u32, y: u32) -> PathBuf {
        let disk_y = if self.tms { flip_y(z, y) } else { y };
        self.base.join(self.layout.render(z, x, disk_y))
    }

    /// Walk the pyramid once to derive `minzoom`/`maxzoom`/`bounds`.
    async fn compute_coverage(&self) -> Coverage {
        let base = self.base.clone();
        let tms = self.tms;
        tokio::task::spawn_blocking(move || walk_coverage(&base, tms))
            .await
            .unwrap_or(Coverage {
                minzoom: 0,
                maxzoom: 22,
                bounds: None,
            })
    }
}

#[async_trait]
impl TileSource for DirSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        let max_tile = 1u32 << z;
        if x >= max_tile || y >= max_tile {
            return Err(TileServerError::InvalidCoordinates { z, x, y });
        }
        if z < self.metadata.minzoom || z > self.metadata.maxzoom {
            return Ok(None);
        }

        let path = self.tile_path(z, x, y);
        let format = self.metadata.format;

        let bytes = match tokio::fs::read(&path).await {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(TileServerError::FileError(e)),
        };

        let compression = detect_compression(&bytes);
        Ok(Some(TileData {
            data: bytes.into(),
            format,
            compression,
        }))
    }

    fn metadata(&self) -> &TileMetadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl DirSource {
    /// Metadata enriched with lazily-walked coverage. Used by the TileJSON
    /// route so `minzoom`/`maxzoom`/`bounds` reflect the on-disk pyramid
    /// without paying the walk cost at startup.
    ///
    /// # Errors
    ///
    /// Never errors; falls back to config/default coverage if the walk fails.
    pub async fn metadata_with_coverage(&self) -> TileMetadata {
        let coverage = self
            .coverage
            .get_or_init(|| self.compute_coverage())
            .await
            .clone();
        let mut meta = self.metadata.clone();
        if self.metadata_minzoom_unset() {
            meta.minzoom = coverage.minzoom;
        }
        if self.metadata_maxzoom_unset() {
            meta.maxzoom = coverage.maxzoom;
        }
        if meta.bounds.is_none() {
            meta.bounds = coverage.bounds;
        }
        meta
    }

    fn metadata_minzoom_unset(&self) -> bool {
        self.metadata.minzoom == 0
    }

    fn metadata_maxzoom_unset(&self) -> bool {
        self.metadata.maxzoom == 22
    }
}

/// Detect gzip framing on raw tile bytes (matches the MBTiles source policy).
fn detect_compression(data: &[u8]) -> TileCompression {
    if data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b {
        TileCompression::Gzip
    } else {
        TileCompression::None
    }
}

/// Resolve the served format: `serve_as` override wins, then the template
/// extension, then a probe of the first tile on disk, defaulting to PBF.
async fn resolve_format(config: &SourceConfig, base: &Path, layout: &TileLayout) -> TileFormat {
    if let Some(fmt) = config.serve_as {
        return fmt;
    }
    if let Some(ext) = layout.extension()
        && let Ok(fmt) = ext.parse::<TileFormat>()
    {
        return fmt;
    }
    let base = base.to_path_buf();
    tokio::task::spawn_blocking(move || detect_extension(&base))
        .await
        .ok()
        .flatten()
        .and_then(|ext| ext.parse::<TileFormat>().ok())
        .unwrap_or(TileFormat::Pbf)
}

/// Synchronously walk `{z}/{x}/{y}` directories to derive coverage.
fn walk_coverage(base: &Path, tms: bool) -> Coverage {
    let mut min_z = u8::MAX;
    let mut max_z = 0u8;
    let mut tiles_at_max: Vec<(u32, u32)> = Vec::new();

    let Ok(z_entries) = std::fs::read_dir(base) else {
        return Coverage {
            minzoom: 0,
            maxzoom: 22,
            bounds: None,
        };
    };

    for z_entry in z_entries.flatten() {
        let Some(z) = parse_dir_number::<u8>(&z_entry) else {
            continue;
        };
        if !z_entry.path().is_dir() {
            continue;
        }
        min_z = min_z.min(z);
        if z > max_z {
            max_z = z;
            tiles_at_max.clear();
        }
        if z == max_z {
            collect_tiles(&z_entry.path(), &mut tiles_at_max);
        }
    }

    if min_z == u8::MAX {
        return Coverage {
            minzoom: 0,
            maxzoom: 22,
            bounds: None,
        };
    }

    let bounds = bounds_from_tiles(max_z, &tiles_at_max, tms);
    Coverage {
        minzoom: min_z,
        maxzoom: max_z,
        bounds,
    }
}

/// Collect `(x, y)` tile coordinates under a single zoom directory.
fn collect_tiles(z_dir: &Path, out: &mut Vec<(u32, u32)>) {
    let Ok(x_entries) = std::fs::read_dir(z_dir) else {
        return;
    };
    for x_entry in x_entries.flatten() {
        let Some(x) = parse_dir_number::<u32>(&x_entry) else {
            continue;
        };
        let Ok(y_entries) = std::fs::read_dir(x_entry.path()) else {
            continue;
        };
        for y_entry in y_entries.flatten() {
            let name = y_entry.file_name();
            let name = name.to_string_lossy();
            let stem = name.split('.').next().unwrap_or(&name);
            if let Ok(y) = stem.parse::<u32>() {
                out.push((x, y));
            }
        }
    }
}

/// Parse a directory entry's file name as a zoom/column number.
fn parse_dir_number<T: std::str::FromStr>(entry: &std::fs::DirEntry) -> Option<T> {
    entry.file_name().to_string_lossy().parse::<T>().ok()
}

/// Derive WGS-84 bounds `[west, south, east, north]` from the tile envelope at
/// the maximum zoom level. Returns `None` when there are no tiles.
fn bounds_from_tiles(z: u8, tiles: &[(u32, u32)], tms: bool) -> Option<[f64; 4]> {
    let (min_x, max_x, min_y, max_y) = tiles.iter().fold(
        (u32::MAX, 0u32, u32::MAX, 0u32),
        |(mn_x, mx_x, mn_y, mx_y), &(x, y)| {
            let y = if tms { flip_y(z, y) } else { y };
            (mn_x.min(x), mx_x.max(x), mn_y.min(y), mx_y.max(y))
        },
    );
    if min_x == u32::MAX {
        return None;
    }
    let west = tile_x_to_lon(min_x, z);
    let east = tile_x_to_lon(max_x + 1, z);
    let north = tile_y_to_lat(min_y, z);
    let south = tile_y_to_lat(max_y + 1, z);
    Some([west, south, east, north])
}

/// Web Mercator tile column → longitude (degrees) of its western edge.
fn tile_x_to_lon(x: u32, z: u8) -> f64 {
    f64::from(x) / f64::from(1u32 << z) * 360.0 - 180.0
}

/// Web Mercator tile row → latitude (degrees) of its northern edge.
fn tile_y_to_lat(y: u32, z: u8) -> f64 {
    let n = std::f64::consts::PI * (1.0 - 2.0 * f64::from(y) / f64::from(1u32 << z));
    n.sinh().atan().to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn base_config(path: &str) -> SourceConfig {
        SourceConfig {
            id: "dir-src".to_string(),
            source_type: crate::config::SourceType::Dir,
            path: path.to_string(),
            name: None,
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            minzoom: None,
            maxzoom: None,
            query: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
            collection: None,
            asset_role: "visual".to_string(),
            dynamic: false,
            max_items: 100,
            stac_bbox: None,
            pixel_selection: crate::config::PixelSelectionMethod::First,
            tile_path_template: None,
            tms: false,
        }
    }

    /// Build a temp `{z}/{x}/{y}.pbf` pyramid and return its root.
    fn make_pyramid() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        write_tile(dir.path(), "2/1/1.pbf", b"tile-2-1-1");
        write_tile(dir.path(), "3/4/5.pbf", b"tile-3-4-5");
        dir
    }

    fn write_tile(base: &Path, rel: &str, bytes: &[u8]) {
        let full = base.join(rel);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        let mut f = std::fs::File::create(full).unwrap();
        f.write_all(bytes).unwrap();
    }

    #[tokio::test]
    async fn from_file_rejects_missing_dir() {
        let cfg = base_config("/__no_such_dir__/tiles");
        let err = DirSource::from_file(&cfg).await.unwrap_err();
        assert!(matches!(err, TileServerError::FileError(_)));
    }

    #[tokio::test]
    async fn get_tile_default_template_returns_bytes() {
        let dir = make_pyramid();
        let cfg = base_config(&dir.path().to_string_lossy());
        let src = DirSource::from_file(&cfg).await.unwrap();
        let tile = src.get_tile(3, 4, 5).await.unwrap().expect("tile present");
        assert_eq!(tile.data.as_ref(), b"tile-3-4-5");
        assert_eq!(tile.format, TileFormat::Pbf);
        assert_eq!(tile.compression, TileCompression::None);
    }

    #[tokio::test]
    async fn get_tile_missing_returns_none() {
        let dir = make_pyramid();
        let cfg = base_config(&dir.path().to_string_lossy());
        let src = DirSource::from_file(&cfg).await.unwrap();
        assert!(src.get_tile(5, 0, 0).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_tile_invalid_coords_errors() {
        let dir = make_pyramid();
        let cfg = base_config(&dir.path().to_string_lossy());
        let src = DirSource::from_file(&cfg).await.unwrap();
        let err = src.get_tile(2, 99, 0).await.unwrap_err();
        assert!(matches!(
            err,
            TileServerError::InvalidCoordinates { z: 2, x: 99, y: 0 }
        ));
    }

    #[tokio::test]
    async fn custom_template_resolves_retina_png() {
        let dir = tempfile::tempdir().unwrap();
        write_tile(dir.path(), "4/2/3@2x.png", b"\x89PNGretina");
        let mut cfg = base_config(&dir.path().to_string_lossy());
        cfg.tile_path_template = Some("{z}/{x}/{y}@2x.png".to_string());
        let src = DirSource::from_file(&cfg).await.unwrap();
        assert_eq!(src.metadata().format, TileFormat::Png);
        let tile = src.get_tile(4, 2, 3).await.unwrap().expect("retina tile");
        assert_eq!(tile.data.as_ref(), b"\x89PNGretina");
    }

    #[tokio::test]
    async fn lazy_metadata_derives_zoom_and_bounds() {
        let dir = make_pyramid();
        let cfg = base_config(&dir.path().to_string_lossy());
        let src = DirSource::from_file(&cfg).await.unwrap();
        // Eager metadata uses defaults (no walk yet).
        assert_eq!(src.metadata().minzoom, 0);
        assert_eq!(src.metadata().maxzoom, 22);
        assert!(src.metadata().bounds.is_none());
        // The walk runs lazily on first coverage request.
        let enriched = src.metadata_with_coverage().await;
        assert_eq!(enriched.minzoom, 2);
        assert_eq!(enriched.maxzoom, 3);
        assert!(
            enriched.bounds.is_some(),
            "bounds derived from max-zoom tiles"
        );
    }

    #[tokio::test]
    async fn tms_flips_y_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        // At z=2, XYZ y=1 maps to TMS disk row 2.
        write_tile(dir.path(), "2/1/2.pbf", b"tms-tile");
        let mut cfg = base_config(&dir.path().to_string_lossy());
        cfg.tms = true;
        let src = DirSource::from_file(&cfg).await.unwrap();
        let tile = src.get_tile(2, 1, 1).await.unwrap().expect("flipped tile");
        assert_eq!(tile.data.as_ref(), b"tms-tile");
    }

    #[tokio::test]
    async fn gzip_tiles_detected() {
        let dir = tempfile::tempdir().unwrap();
        write_tile(dir.path(), "1/0/0.pbf", &[0x1f, 0x8b, 0x08, 0x00, 0x01]);
        let cfg = base_config(&dir.path().to_string_lossy());
        let src = DirSource::from_file(&cfg).await.unwrap();
        let tile = src.get_tile(1, 0, 0).await.unwrap().unwrap();
        assert_eq!(tile.compression, TileCompression::Gzip);
    }

    #[tokio::test]
    async fn as_any_downcasts() {
        let dir = make_pyramid();
        let cfg = base_config(&dir.path().to_string_lossy());
        let src = DirSource::from_file(&cfg).await.unwrap();
        assert!(src.as_any().downcast_ref::<DirSource>().is_some());
    }

    #[test]
    fn tile_y_to_lat_equator_at_z1() {
        // Row 1 at z=1 is the equator.
        assert!((tile_y_to_lat(1, 1) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn tile_x_to_lon_antimeridian() {
        assert!((tile_x_to_lon(0, 1) - (-180.0)).abs() < 1e-9);
        assert!((tile_x_to_lon(2, 1) - 180.0).abs() < 1e-9);
    }
}
