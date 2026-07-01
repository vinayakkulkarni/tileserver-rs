//! Cloud-Optimized GeoTIFF (COG) raster tile source via GDAL.

use async_trait::async_trait;
use bytes::Bytes;
use gdal::DatasetOptions;
use gdal::raster::{Buffer, ResampleAlg};
use gdal::spatial_ref::SpatialRef;
use gdal::{Dataset, DriverManager};
use image::{ImageBuffer, RgbaImage};
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::{ColorMapConfig, ResamplingMethod, SourceConfig};
use crate::error::{Result, TileServerError};
use crate::sources::dataset_cache;
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

const WEB_MERCATOR_EXTENT: f64 = 20037508.342789244;

pub struct CogSource {
    dataset: Arc<Mutex<Dataset>>,
    metadata: TileMetadata,
    default_resampling: ResamplingMethod,
    band_count: usize,
    colormap: Option<ColorMapConfig>,
    /// COG source path — kept so we can re-open the dataset with an
    /// `OVERVIEW_LEVEL=N` open option when a low-zoom tile is smaller
    /// than the full-resolution source.  This is the titiler/rio-tiler
    /// `get_overview_level` optimisation; without it a z0 world tile
    /// forces GDAL to read the entire source raster.
    path: String,
    /// Per-overview resolutions in EPSG:3857 metres/pixel, sorted
    /// finest → coarsest.  `overview_resolutions[0]` is the full-res
    /// source; `overview_resolutions[i+1]` is the i-th overview
    /// (matching GDAL's `OVERVIEW_LEVEL=i` numbering where
    /// level 0 is the first overview, NOT the base).
    overview_resolutions: Vec<f64>,
}

impl CogSource {
    pub async fn from_file(config: &SourceConfig) -> Result<Self> {
        let path = config.path.clone();
        let id = config.id.clone();
        let name = config.name.clone();
        let attribution = config.attribution.clone();
        let resampling = config.resampling.unwrap_or_default();
        let colormap = config.colormap.clone();

        // Reuse a cached `Dataset` when the same path (local or /vsicurl/)
        // has been opened before. This saves the 10-50ms HTTP+IFD parse
        // cost that would otherwise be paid on every STAC-mosaic asset
        // render.  See `dataset_cache` for eviction policy.
        let dataset = dataset_cache::global().get_or_open(&path).await?;

        let dataset_for_inspect = Arc::clone(&dataset);
        let (band_count, bounds, overview_resolutions) = tokio::task::spawn_blocking(move || {
            let guard = dataset_for_inspect.blocking_lock();
            let band_count = guard.raster_count();
            if band_count == 0 {
                return Err(TileServerError::RasterError(
                    "COG file has no raster bands".to_string(),
                ));
            }
            let bounds = get_wgs84_bounds(&guard)?;
            let overview_resolutions = compute_overview_resolutions(&guard)?;
            Ok::<_, TileServerError>((band_count, bounds, overview_resolutions))
        })
        .await
        .map_err(|e| TileServerError::RasterError(format!("task failed: {e}")))??;

        let metadata = TileMetadata {
            id,
            name: name.unwrap_or_else(|| "COG Source".to_string()),
            description: None,
            attribution,
            format: TileFormat::Png,
            minzoom: 0,
            maxzoom: 22,
            bounds: Some(bounds),
            center: Some([
                (bounds[0] + bounds[2]) / 2.0,
                (bounds[1] + bounds[3]) / 2.0,
                10.0,
            ]),
            vector_layers: None,
        };

        Ok(Self {
            dataset,
            metadata,
            default_resampling: resampling,
            band_count,
            colormap,
            path,
            overview_resolutions,
        })
    }

    #[must_use]
    pub fn resampling(&self) -> ResamplingMethod {
        self.default_resampling
    }

    /// Share this source's cached GDAL dataset handle so a wrapping source
    /// (e.g. a DEM source whose `input_source` points here) can read the
    /// same float bands without re-opening the file.
    #[must_use]
    pub fn dataset_handle(&self) -> Arc<Mutex<Dataset>> {
        Arc::clone(&self.dataset)
    }

    pub async fn get_tile_with_resampling(
        &self,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
        resampling: ResamplingMethod,
    ) -> Result<Option<TileData>> {
        let (minx, miny, maxx, maxy) = tile_to_web_mercator_bbox(z, x, y);

        let target_resolution = (maxx - minx) / f64::from(tile_size);
        let overview_index = select_overview_level(&self.overview_resolutions, target_resolution);

        let band_count = self.band_count;
        let colormap = self.colormap.clone();
        let path = self.path.clone();
        let dataset = self.dataset.clone();

        let png_data = tokio::task::spawn_blocking(move || {
            let render_from = |dataset: &Dataset| {
                render_tile_from_dataset(
                    dataset,
                    minx,
                    miny,
                    maxx,
                    maxy,
                    tile_size,
                    band_count,
                    resampling.into(),
                    colormap.as_ref(),
                )
            };

            // `overview_index` of 0 means "use the full-resolution base", which is
            // already the cached dataset.  Any other value means "read from overview
            // level `overview_index - 1`", which requires re-opening the source with
            // GDAL's `OVERVIEW_LEVEL` open option.
            //
            // We do NOT cache the overview-level-opened dataset — each tile request
            // may pick a different overview level, so a cache keyed by (path, idx)
            // would grow unboundedly per-zoom.  The cost is a single `Dataset::open_ex`
            // per tile (~1-5ms on warm OS page cache), recovered many-times-over by
            // not reading the full-res raster.
            if overview_index == 0 {
                let dataset = dataset.blocking_lock();
                render_from(&dataset)
            } else {
                let ovr_level = overview_index - 1;
                let ovr_arg = format!("OVERVIEW_LEVEL={ovr_level}");
                let open_options: [&str; 1] = [&ovr_arg];
                let dataset = Dataset::open_ex(
                    &path,
                    DatasetOptions {
                        open_options: Some(&open_options),
                        ..DatasetOptions::default()
                    },
                )
                .map_err(|e| {
                    TileServerError::RasterError(format!(
                        "failed to open {path} with OVERVIEW_LEVEL={ovr_level}: {e}"
                    ))
                })?;
                render_from(&dataset)
            }
        })
        .await
        .map_err(|e| TileServerError::RasterError(format!("task failed: {e}")))??;

        Ok(Some(TileData {
            data: Bytes::from(png_data),
            format: TileFormat::Png,
            compression: TileCompression::None,
        }))
    }
}

#[async_trait]
impl TileSource for CogSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        self.get_tile_with_resampling(z, x, y, 256, self.default_resampling)
            .await
    }

    fn metadata(&self) -> &TileMetadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Collect the full-resolution pixel size and each overview's
/// pixel size from band 1 of the COG.  Resolutions are expressed
/// in the source's native linear unit (typically metres for EPSG:3857
/// COGs and decimal-degrees for EPSG:4326 COGs).  The returned vector
/// is always non-empty and sorted finest → coarsest.
///
/// The source resolution is computed from the GeoTransform's pixel
/// size (index 1, GDAL convention is [ulx, px_size_x, rot_x, uly,
/// rot_y, -px_size_y]).  Overview resolutions are derived from the
/// overview's pixel count ratio to the base, matching the rio-tiler
/// formula `src_res * (src_width / overview_width)`.
fn compute_overview_resolutions(dataset: &Dataset) -> Result<Vec<f64>> {
    let transform = dataset
        .geo_transform()
        .map_err(|e| TileServerError::RasterError(format!("failed to get geotransform: {e}")))?;
    let src_res = transform[1].abs();
    let (src_width, _) = dataset.raster_size();
    let src_width = src_width as f64;

    let band = dataset
        .rasterband(1)
        .map_err(|e| TileServerError::RasterError(format!("failed to read band 1: {e}")))?;
    let overview_count = band
        .overview_count()
        .map_err(|e| TileServerError::RasterError(format!("failed to get overview count: {e}")))?;

    let mut resolutions = Vec::with_capacity(1 + overview_count.max(0) as usize);
    resolutions.push(src_res);

    for i in 0..overview_count {
        let ovr = band.overview(i as usize).map_err(|e| {
            TileServerError::RasterError(format!("failed to get overview {i}: {e}"))
        })?;
        let (ovr_width, _) = ovr.size();
        // Classic COG overview: decimation is src_width / ovr_width
        // (always ≥ 2 in a well-built COG, producing coarser res).
        let ovr_res = src_res * (src_width / ovr_width as f64);
        resolutions.push(ovr_res);
    }

    Ok(resolutions)
}

/// Pick the overview level to read from when rendering at the given
/// target resolution (in the same unit as `overview_resolutions`).
///
/// Returns `0` for "use the base raster" or `N` for "use GDAL's
/// OVERVIEW_LEVEL=N-1".  The algorithm mirrors rio-tiler's
/// `get_overview_level` with a 50 % midpoint strategy: for a target
/// resolution between levels, pick the **coarser** one iff the target
/// is closer to it than to the finer level.  This gives the correct
/// behaviour when `target_res` matches the base (picks level 0) AND
/// when `target_res` is between overviews (picks the coarser one).
fn select_overview_level(overview_resolutions: &[f64], target_resolution: f64) -> usize {
    if overview_resolutions.len() <= 1 {
        return 0;
    }

    // Target resolution is at-or-finer than the base: no overview helps.
    if target_resolution <= overview_resolutions[0] {
        return 0;
    }

    // Walk finest → coarsest; stop at the first overview whose resolution
    // exceeds the target, then compare midpoint with the previous level.
    for idx in 1..overview_resolutions.len() {
        let current = overview_resolutions[idx];
        let prev = overview_resolutions[idx - 1];
        if current >= target_resolution {
            // 50 % midpoint rule: if target is closer to the finer level
            // (prev), keep the finer one; otherwise promote to coarser.
            let midpoint = (prev + current) / 2.0;
            return if target_resolution < midpoint {
                idx - 1
            } else {
                idx
            };
        }
    }

    // Target is coarser than every overview — use the coarsest available.
    overview_resolutions.len() - 1
}

fn tile_to_web_mercator_bbox(z: u8, x: u32, y: u32) -> (f64, f64, f64, f64) {
    let n = 2_u32.pow(z as u32) as f64;
    let tile_size = 2.0 * WEB_MERCATOR_EXTENT / n;

    let minx = -WEB_MERCATOR_EXTENT + (x as f64) * tile_size;
    let maxx = minx + tile_size;

    let maxy = WEB_MERCATOR_EXTENT - (y as f64) * tile_size;
    let miny = maxy - tile_size;

    (minx, miny, maxx, maxy)
}

pub(crate) fn wgs84_bounds(dataset: &Dataset) -> Result<[f64; 4]> {
    get_wgs84_bounds(dataset)
}

fn get_wgs84_bounds(dataset: &Dataset) -> Result<[f64; 4]> {
    let transform = dataset
        .geo_transform()
        .map_err(|e| TileServerError::RasterError(format!("Failed to get geotransform: {}", e)))?;

    let (width, height) = dataset.raster_size();

    let src_srs = dataset.spatial_ref().map_err(|e| {
        TileServerError::RasterError(format!("Failed to get spatial reference: {}", e))
    })?;

    let mut dst_srs = SpatialRef::from_epsg(4326)
        .map_err(|e| TileServerError::RasterError(format!("Failed to create WGS84 SRS: {}", e)))?;
    dst_srs.set_axis_mapping_strategy(gdal::spatial_ref::AxisMappingStrategy::TraditionalGisOrder);

    let coord_transform =
        gdal::spatial_ref::CoordTransform::new(&src_srs, &dst_srs).map_err(|e| {
            TileServerError::RasterError(format!("Failed to create coordinate transform: {}", e))
        })?;

    let ulx = transform[0];
    let uly = transform[3];
    let lrx = transform[0] + (width as f64) * transform[1];
    let lry = transform[3] + (height as f64) * transform[5];

    let mut xs = [ulx, lrx, ulx, lrx];
    let mut ys = [uly, uly, lry, lry];
    let mut zs = [0.0; 4];

    coord_transform
        .transform_coords(&mut xs, &mut ys, &mut zs)
        .map_err(|e| {
            TileServerError::RasterError(format!("Failed to transform coordinates: {}", e))
        })?;

    let min_lon = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_lon = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_lat = ys.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_lat = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    Ok([min_lon, min_lat, max_lon, max_lat])
}

#[allow(clippy::too_many_arguments)]
fn render_tile_from_dataset(
    dataset: &Dataset,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
    tile_size: u32,
    band_count: usize,
    resampling: ResampleAlg,
    colormap: Option<&ColorMapConfig>,
) -> Result<Vec<u8>> {
    let web_mercator = SpatialRef::from_epsg(3857)
        .map_err(|e| TileServerError::RasterError(format!("Failed to create EPSG:3857: {}", e)))?;

    let mem_driver = DriverManager::get_driver_by_name("MEM")
        .map_err(|e| TileServerError::RasterError(format!("Failed to get MEM driver: {}", e)))?;

    let use_colormap = colormap.is_some() && band_count == 1;
    let output_bands = if use_colormap { 1 } else { band_count.min(4) };

    let mut warped = mem_driver
        .create_with_band_type::<f64, _>("", tile_size as usize, tile_size as usize, output_bands)
        .map_err(|e| TileServerError::RasterError(format!("Failed to create output: {}", e)))?;

    let pixel_size_x = (maxx - minx) / tile_size as f64;
    let pixel_size_y = (maxy - miny) / tile_size as f64;
    let geo_transform = [minx, pixel_size_x, 0.0, maxy, 0.0, -pixel_size_y];

    warped
        .set_geo_transform(&geo_transform)
        .map_err(|e| TileServerError::RasterError(format!("Failed to set geotransform: {}", e)))?;
    warped
        .set_spatial_ref(&web_mercator)
        .map_err(|e| TileServerError::RasterError(format!("Failed to set SRS: {}", e)))?;

    gdal::raster::reproject(dataset, &warped)
        .map_err(|e| TileServerError::RasterError(format!("Failed to reproject/warp: {}", e)))?;

    let mut img: RgbaImage = ImageBuffer::new(tile_size, tile_size);

    if use_colormap {
        let cmap = colormap.expect("colormap verified by use_colormap flag");
        let band = warped
            .rasterband(1)
            .map_err(|e| TileServerError::RasterError(format!("Failed to get band: {}", e)))?;

        let buffer: Buffer<f64> = band
            .read_as::<f64>(
                (0, 0),
                (tile_size as usize, tile_size as usize),
                (tile_size as usize, tile_size as usize),
                Some(resampling),
            )
            .map_err(|e| TileServerError::RasterError(format!("Failed to read band: {}", e)))?;

        let data = buffer.data();

        for (i, &value) in data.iter().enumerate() {
            let px = (i % tile_size as usize) as u32;
            let py = (i / tile_size as usize) as u32;
            let color = cmap.get_color(value);
            img.put_pixel(px, py, image::Rgba(color));
        }
    } else {
        for band_idx in 1..=output_bands {
            let band = warped.rasterband(band_idx).map_err(|e| {
                TileServerError::RasterError(format!("Failed to get band {}: {}", band_idx, e))
            })?;

            let buffer: Buffer<u8> = band
                .read_as::<u8>(
                    (0, 0),
                    (tile_size as usize, tile_size as usize),
                    (tile_size as usize, tile_size as usize),
                    Some(resampling),
                )
                .map_err(|e| TileServerError::RasterError(format!("Failed to read band: {}", e)))?;

            let data = buffer.data();

            for (i, &value) in data.iter().enumerate() {
                let px = (i % tile_size as usize) as u32;
                let py = (i / tile_size as usize) as u32;
                let pixel = img.get_pixel_mut(px, py);

                match band_idx {
                    1 => pixel[0] = value,
                    2 => pixel[1] = value,
                    3 => pixel[2] = value,
                    4 => pixel[3] = value,
                    _ => {}
                }
            }
        }

        match output_bands {
            1 => {
                for pixel in img.pixels_mut() {
                    let gray = pixel[0];
                    pixel[1] = gray;
                    pixel[2] = gray;
                    pixel[3] = 255;
                }
            }
            3 => {
                for pixel in img.pixels_mut() {
                    pixel[3] = 255;
                }
            }
            _ => {}
        }
    }

    let mut png_data = Vec::new();
    let mut cursor = Cursor::new(&mut png_data);
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| TileServerError::RasterError(format!("Failed to encode PNG: {}", e)))?;

    Ok(png_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_to_web_mercator_bbox() {
        let (minx, miny, maxx, maxy) = tile_to_web_mercator_bbox(0, 0, 0);

        assert!((minx - (-WEB_MERCATOR_EXTENT)).abs() < 1e-6);
        assert!((miny - (-WEB_MERCATOR_EXTENT)).abs() < 1e-6);
        assert!((maxx - WEB_MERCATOR_EXTENT).abs() < 1e-6);
        assert!((maxy - WEB_MERCATOR_EXTENT).abs() < 1e-6);
    }

    #[test]
    fn test_select_overview_level_no_overviews() {
        // Single-element resolutions (no overviews built): always pick base.
        let resolutions = vec![10.0];
        assert_eq!(select_overview_level(&resolutions, 1.0), 0);
        assert_eq!(select_overview_level(&resolutions, 10.0), 0);
        assert_eq!(select_overview_level(&resolutions, 1000.0), 0);
    }

    #[test]
    fn test_select_overview_level_target_finer_than_base() {
        // If the caller asks for resolution finer than the base, no overview
        // could ever help — stay at base.
        let resolutions = vec![10.0, 20.0, 40.0, 80.0];
        assert_eq!(select_overview_level(&resolutions, 5.0), 0);
        assert_eq!(select_overview_level(&resolutions, 10.0), 0);
    }

    #[test]
    fn test_select_overview_level_midpoint_rule() {
        // base=10, overviews at 20, 40, 80.
        let resolutions = vec![10.0, 20.0, 40.0, 80.0];

        // Between base (10) and first overview (20): midpoint = 15.
        // target=14 → closer to base → pick base (0).
        assert_eq!(select_overview_level(&resolutions, 14.0), 0);
        // target=16 → closer to first overview → pick overview 1 (returns 1).
        assert_eq!(select_overview_level(&resolutions, 16.0), 1);

        // Between first (20) and second (40) overview: midpoint = 30.
        assert_eq!(select_overview_level(&resolutions, 29.0), 1);
        assert_eq!(select_overview_level(&resolutions, 31.0), 2);

        // Exactly at a level: "close enough to step up".
        assert_eq!(select_overview_level(&resolutions, 20.0), 1);
    }

    #[test]
    fn test_select_overview_level_target_coarser_than_all() {
        // Target resolution exceeds the coarsest overview — clamp to it.
        let resolutions = vec![10.0, 20.0, 40.0, 80.0];
        assert_eq!(select_overview_level(&resolutions, 160.0), 3);
        assert_eq!(select_overview_level(&resolutions, 10_000.0), 3);
    }

    #[test]
    fn test_tile_to_web_mercator_bbox_z1() {
        let (minx, miny, maxx, maxy) = tile_to_web_mercator_bbox(1, 0, 0);

        assert!((minx - (-WEB_MERCATOR_EXTENT)).abs() < 1e-6);
        assert!((maxy - WEB_MERCATOR_EXTENT).abs() < 1e-6);
        assert!((maxx - 0.0).abs() < 1e-6);
        assert!((miny - 0.0).abs() < 1e-6);
    }
}
