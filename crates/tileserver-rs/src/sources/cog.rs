//! Cloud-Optimized GeoTIFF (COG) raster tile source via GDAL.

use async_trait::async_trait;
use bytes::Bytes;
use gdal::raster::{Buffer, ColorInterpretation, ResampleAlg};
use gdal::spatial_ref::SpatialRef;
use gdal::{Dataset, DriverManager};
use image::{ImageBuffer, RgbaImage};
use std::io::Cursor;
use std::sync::Arc;

use crate::config::{ColorMapConfig, ResamplingMethod, SourceConfig};
use crate::error::{Result, TileServerError};
use crate::sources::dataset_cache;
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

const WEB_MERCATOR_EXTENT: f64 = 20037508.342789244;

pub struct CogSource {
    metadata: TileMetadata,
    default_resampling: ResamplingMethod,
    band_count: usize,
    colormap: Option<ColorMapConfig>,
    /// COG source path. It is the `dataset_cache` key of the base pool, and
    /// the path the overview pools (`{path}#ovr{n}`) re-open with an
    /// `OVERVIEW_LEVEL=N` open option when a low-zoom tile is smaller
    /// than the full-resolution source.
    ///
    /// The source holds the path, not an `Arc<Pool>`: every render looks the
    /// pool up in the cache (a moka hit). A held `Arc` would keep a pool alive
    /// after reclaim or the TTL removed it from the cache, where reclaim can
    /// never reach its idle handles again.  This is the titiler/rio-tiler
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
        // Normalise remote paths to a GDAL VSI prefix so `/vsicurl/` range
        // reads are used. A bare `https://…` URL makes GDAL fall back to its
        // curl→/vsimem full-download shim: 4× full-file GETs, ~45 s open, and
        // the whole COG (~500 MB) materialised in memory per handle. With
        // `/vsicurl/` the open is a ~0.25 s range read and a small handle.
        // `to_gdal_cog_path` is idempotent (already-prefixed paths pass through),
        // so configs that spell out `/vsicurl/` manually keep working.
        let path = crate::sources::to_gdal_cog_path(&config.path);
        let id = config.id.clone();
        let name = config.name.clone();
        let attribution = config.attribution.clone();
        let resampling = ResamplingMethod::resolve(config.resampling, config.colormap.as_ref());
        let colormap = config.colormap.clone();

        // Reuse a cached `Dataset` when the same path (local or /vsicurl/)
        // has been opened before. This saves the 10-50ms HTTP+IFD parse
        // cost that would otherwise be paid on every STAC-mosaic asset
        // render.  See `dataset_cache` for eviction policy.
        //
        // The cache stores a *pool* per key, and a render borrows one handle
        // from it (base and overview alike). We borrow one handle now only to
        // read the metadata (band count / bounds / overview resolutions) and
        // return it to the pool at once. The source keeps the path, not the
        // pool; see the `path` field.
        let pool = dataset_cache::global().get_or_open(&path).await?;
        let band_count;
        let bounds;
        let overview_resolutions;
        {
            // Construction-time metadata read (once per source load, not per
            // tile). Use the reclaiming wrapper so a source can still be built
            // when the budget is momentarily saturated — a tile must never fail
            // (and a source must never fail to load) for budget reasons.
            let base_handle = dataset_cache::global().acquire_with_reclaim(&pool).await?;
            let (bc, b, ovr) = tokio::task::spawn_blocking(move || {
                let guard = base_handle.dataset();
                let band_count = guard.raster_count();
                if band_count == 0 {
                    return Err(TileServerError::RasterError(
                        "COG file has no raster bands".to_string(),
                    ));
                }
                let bounds = get_wgs84_bounds(guard)?;
                let overview_resolutions = compute_overview_resolutions(guard)?;
                Ok::<_, TileServerError>((band_count, bounds, overview_resolutions))
            })
            .await
            .map_err(|e| TileServerError::RasterError(format!("task failed: {e}")))??;
            band_count = bc;
            bounds = b;
            overview_resolutions = ovr;
        } // `base_handle` returned to the pool here.

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

    /// The source path, which is also the `dataset_cache` key of the base
    /// pool. A wrapping source (e.g. a DEM source whose `input_source` points
    /// here) looks up the same pool with it, so it shares the cached handles
    /// without a second open.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
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

        // `overview_index` of 0 means "read the full-resolution base". Any
        // other value means "read from overview level `overview_index - 1`",
        // which requires re-opening the source with GDAL's `OVERVIEW_LEVEL`
        // open option.
        //
        // Both base and overview datasets are **cached per key** in the
        // process-global `dataset_cache` (mirroring rio-tiler's per-level
        // reader cache): the base under `{path}`, each overview under
        // `{path}#ovr{n}`. Each key keeps a *pool* of handles, not one shared
        // handle, and a handle is **borrowed per render** and returned to the
        // pool on drop. This means concurrent tiles at the same level each
        // borrow a different handle and render in parallel (a single
        // `Arc<Mutex<Dataset>>` serialised them) without detaching handles
        // from the pool (detaching leaf the pool empty after `limit` sources
        // and defeated the cache).
        // The overview-pool cache lookup is async and runs on the async
        // runtime, before the blocking render is spawned.
        let render_pool: Arc<dataset_cache::Pool> = if overview_index == 0 {
            dataset_cache::global().get_or_open(&path).await?
        } else {
            let ovr_level = overview_index - 1;
            let ovr_arg = format!("OVERVIEW_LEVEL={ovr_level}");
            let key = format!("{path}#ovr{ovr_level}");
            dataset_cache::global()
                .get_or_open_with_options(&key, &path, &[&ovr_arg])
                .await?
        };

        // Acquire the handle HERE, on the async thread, so a budget refusal can
        // be answered by evicting idle pools and retrying. `acquire_blocking`
        // alone cannot do that: reclamation awaits moka's maintenance, which a
        // blocking task must not drive. A `PooledHandle` is `Send`, so the
        // acquired handle moves into the blocking render unchanged.
        let handle = dataset_cache::global()
            .acquire_with_reclaim(&render_pool)
            .await?;

        let png_data = tokio::task::spawn_blocking(move || {
            // Render from the handle borrowed above, then return it to the
            // pool on drop.
            render_tile_from_dataset(
                handle.dataset(),
                minx,
                miny,
                maxx,
                maxy,
                tile_size,
                band_count,
                resampling,
                colormap.as_ref(),
            )
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

/// Render one WebMercator tile from `dataset` (any CRS) into a PNG.
///
/// Band-layout contract: 1 band + colormap → colormap LUT; ≥ 3 source
/// bands → 4-band RGBA with warp-maintained alpha (`gdalwarp -dstalpha`
/// semantics: 255 where the source contributed valid pixels, 0 outside
/// the source footprint, blended at edges); 1-2 bands without colormap →
/// legacy gray/RG layout forced fully opaque.
#[allow(clippy::too_many_arguments)]
fn render_tile_from_dataset(
    dataset: &Dataset,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
    tile_size: u32,
    band_count: usize,
    resampling: ResamplingMethod,
    colormap: Option<&ColorMapConfig>,
) -> Result<Vec<u8>> {
    let web_mercator = SpatialRef::from_epsg(3857)
        .map_err(|e| TileServerError::RasterError(format!("Failed to create EPSG:3857: {}", e)))?;

    let mem_driver = DriverManager::get_driver_by_name("MEM")
        .map_err(|e| TileServerError::RasterError(format!("Failed to get MEM driver: {}", e)))?;

    let use_colormap = colormap.is_some() && band_count == 1;
    // Band layout of the MEM warp destination:
    //   - 1 band + colormap → single f64 band (colormap lookup below)
    //   - ≥ 3 source bands → 4-band RGBA: R, G, B plus a destination ALPHA
    //     band maintained by the warp kernel (gdalwarp -dstalpha
    //     semantics). A source alpha band (e.g. a 4-band RGBA COG) feeds
    //     the source density; without one every valid source pixel is
    //     dense. This keeps out-of-footprint areas transparent instead of
    //     opaque black — matching rio-tiler/TiTiler behaviour.
    //   - 1 or 2 source bands without colormap → legacy layout (gray or
    //     R,G), forced fully opaque below.
    let rgb_with_alpha = !use_colormap && band_count >= 3;
    let output_bands = if use_colormap {
        1
    } else if rgb_with_alpha {
        4
    } else {
        band_count.min(4)
    };

    // Byte-typed destination so the warp kernel writes byte-range alpha
    // (for non-Byte alpha bands the kernel writes 0..1 densities, which a
    // u8 read would collapse to 1). RGB band writes saturate-cast exactly
    // as the previous f64→u8 read did.
    let mut warped = if use_colormap {
        mem_driver
            .create_with_band_type::<f64, _>(
                "",
                tile_size as usize,
                tile_size as usize,
                output_bands,
            )
            .map_err(|e| TileServerError::RasterError(format!("Failed to create output: {}", e)))?
    } else {
        mem_driver
            .create_with_band_type::<u8, _>(
                "",
                tile_size as usize,
                tile_size as usize,
                output_bands,
            )
            .map_err(|e| TileServerError::RasterError(format!("Failed to create output: {}", e)))?
    };

    let pixel_size_x = (maxx - minx) / tile_size as f64;
    let pixel_size_y = (maxy - miny) / tile_size as f64;
    let geo_transform = [minx, pixel_size_x, 0.0, maxy, 0.0, -pixel_size_y];

    warped
        .set_geo_transform(&geo_transform)
        .map_err(|e| TileServerError::RasterError(format!("Failed to set geotransform: {}", e)))?;
    warped
        .set_spatial_ref(&web_mercator)
        .map_err(|e| TileServerError::RasterError(format!("Failed to set SRS: {}", e)))?;

    if rgb_with_alpha {
        // CRITICAL: mark the destination's last band as the alpha band
        // BEFORE warping. GDALReprojectImage maps only
        // min(src_bands, dst_bands) NON-ALPHA bands 1:1 (gdalwarper.cpp:
        // source/destination bands whose color interpretation is
        // GCI_AlphaBand are excluded from the count and handled by the
        // warp kernel as density). Without this marker a 4-band RGBA
        // source maps RGB only and the destination alpha stays at its
        // MEM init value 0 — fully transparent tiles (the bug this
        // fixes).
        warped
            .rasterband(output_bands)
            .and_then(|mut band| band.set_color_interpretation(ColorInterpretation::AlphaBand))
            .map_err(|e| {
                TileServerError::RasterError(format!("Failed to mark dst alpha band: {}", e))
            })?;
    }

    crate::sources::warp::reproject(dataset, &warped, resampling)?;

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
                Some(ResampleAlg::from(resampling)),
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
                    Some(ResampleAlg::from(resampling)),
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
            // Legacy 2-band layout (R, G): no alpha band exists — force
            // fully opaque so the tile is at least visible.
            2 => {
                for pixel in img.pixels_mut() {
                    pixel[3] = 255;
                }
            }
            // 4 bands: the destination alpha band was maintained by the
            // warp kernel from source validity — do not touch it.
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

    // ---- RGBA/RGB tile rendering (warp-maintained destination alpha) ----

    /// Write a small 64×64 GeoTIFF (EPSG:3857, uint8) with the given
    /// per-band data and color interpretations at the given geotransform.
    fn write_test_cog(
        dir: &std::path::Path,
        name: &str,
        bands_data: &[Vec<u8>],
        color_interps: impl IntoIterator<Item = ColorInterpretation>,
        geo_transform: &[f64; 6],
    ) -> Dataset {
        let driver = DriverManager::get_driver_by_name("GTiff").unwrap();
        let size = 64usize;
        let mut ds = driver
            .create_with_band_type::<u8, _>(dir.join(name), size, size, bands_data.len())
            .unwrap();
        ds.set_geo_transform(geo_transform).unwrap();
        let srs = SpatialRef::from_epsg(3857).unwrap();
        ds.set_spatial_ref(&srs).unwrap();
        for (i, (data, ci)) in bands_data.iter().zip(color_interps).enumerate() {
            let mut band = ds.rasterband(i + 1).unwrap();
            let mut buffer = Buffer::new((size, size), data.clone());
            band.write((0, 0), (size, size), &mut buffer).unwrap();
            band.set_color_interpretation(ci).unwrap();
        }
        ds
    }

    const TEST_GT_FULL: [f64; 6] = [
        -WEB_MERCATOR_EXTENT,
        2.0 * WEB_MERCATOR_EXTENT / 64.0,
        0.0,
        WEB_MERCATOR_EXTENT,
        0.0,
        -2.0 * WEB_MERCATOR_EXTENT / 64.0,
    ];

    #[test]
    fn render_rgba_cog_tile_keeps_source_alpha() {
        let dir = tempfile::tempdir().unwrap();
        let size = 64usize;
        // Constant RGB; source alpha: 0 on the left half, 255 right half.
        let r = vec![10u8; size * size];
        let g = vec![20u8; size * size];
        let b = vec![30u8; size * size];
        let mut a = vec![0u8; size * size];
        for y in 0..size {
            for x in size / 2..size {
                a[y * size + x] = 255;
            }
        }
        let ds = write_test_cog(
            dir.path(),
            "rgba.tif",
            &[r, g, b, a],
            [
                ColorInterpretation::RedBand,
                ColorInterpretation::GreenBand,
                ColorInterpretation::BlueBand,
                ColorInterpretation::AlphaBand,
            ],
            &TEST_GT_FULL,
        );
        // Render the z0 world tile — the dataset covers it exactly.
        let png = render_tile_from_dataset(
            &ds,
            -WEB_MERCATOR_EXTENT,
            -WEB_MERCATOR_EXTENT,
            WEB_MERCATOR_EXTENT,
            WEB_MERCATOR_EXTENT,
            256,
            4,
            ResamplingMethod::Nearest,
            None,
        )
        .unwrap();
        let img = image::load_from_memory(&png).unwrap().into_rgba8();
        // Deep inside the alpha-0 half: fully transparent (NOT opaque black).
        let left = img.get_pixel(32, 128);
        assert_eq!(
            (left[0], left[1], left[2], left[3]),
            (0, 0, 0, 0),
            "source alpha-0 pixels must render transparent"
        );
        // Deep inside the alpha-255 half: opaque with source RGB.
        let right = img.get_pixel(224, 128);
        assert_eq!(
            (right[0], right[1], right[2], right[3]),
            (10, 20, 30, 255),
            "source alpha-255 pixels must render opaque RGB"
        );
    }

    #[test]
    fn render_rgb_cog_tile_is_opaque() {
        let dir = tempfile::tempdir().unwrap();
        let size = 64usize;
        let r = vec![10u8; size * size];
        let g = vec![20u8; size * size];
        let b = vec![30u8; size * size];
        let ds = write_test_cog(
            dir.path(),
            "rgb.tif",
            &[r, g, b],
            [
                ColorInterpretation::RedBand,
                ColorInterpretation::GreenBand,
                ColorInterpretation::BlueBand,
            ],
            &TEST_GT_FULL,
        );
        let png = render_tile_from_dataset(
            &ds,
            -WEB_MERCATOR_EXTENT,
            -WEB_MERCATOR_EXTENT,
            WEB_MERCATOR_EXTENT,
            WEB_MERCATOR_EXTENT,
            256,
            3,
            ResamplingMethod::Nearest,
            None,
        )
        .unwrap();
        let img = image::load_from_memory(&png).unwrap().into_rgba8();
        // No source alpha band: the warp-maintained destination alpha must
        // be fully opaque where the source covers the tile.
        let px = img.get_pixel(128, 128);
        assert_eq!((px[0], px[1], px[2], px[3]), (10, 20, 30, 255));
    }

    #[test]
    fn render_out_of_footprint_tile_is_transparent() {
        let dir = tempfile::tempdir().unwrap();
        let size = 64usize;
        // Source covers ONLY the NW quadrant (z1 tile 0/0/0).
        let gt_nw = [
            -WEB_MERCATOR_EXTENT,
            WEB_MERCATOR_EXTENT / 64.0,
            0.0,
            WEB_MERCATOR_EXTENT,
            0.0,
            -WEB_MERCATOR_EXTENT / 64.0,
        ];
        let data = vec![200u8; size * size];
        let ds = write_test_cog(
            dir.path(),
            "nw.tif",
            &[data.clone(), data.clone(), data],
            [
                ColorInterpretation::RedBand,
                ColorInterpretation::GreenBand,
                ColorInterpretation::BlueBand,
            ],
            &gt_nw,
        );
        // Render the SE quadrant tile (z1 1/1) — zero source overlap.
        let png = render_tile_from_dataset(
            &ds,
            0.0,
            -WEB_MERCATOR_EXTENT,
            WEB_MERCATOR_EXTENT,
            0.0,
            256,
            3,
            ResamplingMethod::Nearest,
            None,
        )
        .unwrap();
        let img = image::load_from_memory(&png).unwrap().into_rgba8();
        for px in img.pixels() {
            assert_eq!(
                (px[0], px[1], px[2], px[3]),
                (0, 0, 0, 0),
                "out-of-footprint pixels must stay transparent, not opaque black"
            );
        }
    }

    // ---- Overview-path get_tile (the `{path}#ovr{n}` cache branch) ----

    /// Write a real on-disk RGB GeoTIFF (EPSG:3857, uint8) that has **built
    /// overviews**, so `compute_overview_resolutions` reports more than one
    /// level and a low-zoom tile actually selects the `{path}#ovr{n}` path.
    ///
    /// Returns the file path. The dataset is closed when it drops (before the
    /// caller re-opens it by path, which it must — GDAL overview levels are
    /// only reachable by re-opening with `OVERVIEW_LEVEL=N`, and in-memory
    /// datasets can't be re-opened).
    fn write_overviewed_cog(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        let driver = DriverManager::get_driver_by_name("GTiff").unwrap();
        // 512×512 so a 4×/16× overview chain is legal (GDAL won't build a
        // level below ~2 pixels).
        let size = 512usize;
        let path = dir.join(name);
        {
            let mut ds = driver
                .create_with_band_type::<u8, _>(&path, size, size, 3)
                .unwrap();
            // Full-world EPSG:3857 geotransform, like TEST_GT_FULL.
            let px = 2.0 * WEB_MERCATOR_EXTENT / size as f64;
            ds.set_geo_transform(&[-WEB_MERCATOR_EXTENT, px, 0.0, WEB_MERCATOR_EXTENT, 0.0, -px])
                .unwrap();
            let srs = SpatialRef::from_epsg(3857).unwrap();
            ds.set_spatial_ref(&srs).unwrap();
            for band in 1..=3u8 {
                let mut b = ds.rasterband(band as usize).unwrap();
                let data = vec![128u8; size * size];
                let mut buffer = Buffer::new((size, size), data);
                b.write((0, 0), (size, size), &mut buffer).unwrap();
            }
            // Build internal overviews so `OVERVIEW_LEVEL` opens are legal.
            // `build_overviews` is on `Dataset` (`&mut self`): GDAL builds the
            // pyramid for the given bands at the given decimation factors.
            ds.build_overviews("NEAREST", &[2, 4, 8, 16, 32, 64, 128, 256], &[1, 2, 3])
                .unwrap();
        } // dataset dropped -> file flushed and closed
        path
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[serial_test::serial]
    async fn get_tile_at_overview_level_uses_the_overview_pool() {
        // End-to-end regression for the overview branch of
        // `get_tile_with_resampling`: a real COG with overviews, rendered at a
        // low zoom, must route through the `{path}#ovr{n}` pool rather than the
        // base handle. The pool internals are covered separately; this guards
        // the *plumbing* from `CogSource` down through the cache key. It is
        // `#[serial]` because it asserts against the process-global cache.
        let dir = tempfile::tempdir().unwrap();
        let path = write_overviewed_cog(dir.path(), "overviewed.tif");
        let p = path.to_string_lossy().to_string();
        let config = SourceConfig {
            id: "ovr".to_string(),
            source_type: crate::config::SourceType::Cog,
            path: p.clone(),
            name: None,
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            query: None,
            minzoom: None,
            maxzoom: None,
            serve_as: None,
            colormap: None,
            options: None,
            collection: None,
            asset_role: "data".to_string(),
            dynamic: false,
            max_items: 500,
            stac_bbox: None,
            pixel_selection: crate::config::PixelSelectionMethod::First,
            tile_path_template: None,
            tms: false,
            #[cfg(feature = "dem")]
            input_source: None,
            #[cfg(feature = "dem")]
            dem_encoding: crate::config::DemEncoding::Terrarium,
            #[cfg(feature = "dem")]
            dem_scale: None,
            #[cfg(feature = "dem")]
            dem_offset: None,
            #[cfg(feature = "dem")]
            dem_band: 1,
            #[cfg(feature = "dem")]
            dem_nodata_color: None,
            filter: None,
        };

        let cog = CogSource::from_file(&config)
            .await
            .expect("open overviewed COG");
        assert!(
            cog.overview_resolutions.len() > 1,
            "fixture must actually have overviews: got {:?}",
            cog.overview_resolutions
        );

        // z0 world tile: the target resolution (~156543 m/px) is far coarser
        // than the 512px base (~783 m/px), so `select_overview_level` must
        // pick a level > 0 and the render must borrow from `{path}#ovr{n}`.
        let tile = cog
            .get_tile(0, 0, 0)
            .await
            .expect("render z0 tile")
            .expect("tile in footprint");
        assert!(!tile.data.is_empty(), "render must produce a PNG");
        let img = image::load_from_memory(&tile.data).unwrap().into_rgba8();
        assert_eq!(img.width(), 256);
        assert_eq!(img.height(), 256);

        // The overview branch must have populated a `{path}#ovr{n}` pool.
        let cache = dataset_cache::global();
        let ovr_key_present = (0..8).any(|level| {
            let key = format!("{p}#ovr{level}");
            cache.contains_key(&key)
        });
        assert!(
            ovr_key_present,
            "z0 render must go through the `{{path}}#ovr{{n}}` pool, but no overview key was cached"
        );
    }

    #[test]
    fn render_returns_error_when_warp_fails() {
        let ds = crate::sources::warp::tests::untransformable_source();
        let err = render_tile_from_dataset(
            &ds,
            -WEB_MERCATOR_EXTENT,
            -WEB_MERCATOR_EXTENT,
            WEB_MERCATOR_EXTENT,
            WEB_MERCATOR_EXTENT,
            256,
            1,
            ResamplingMethod::Nearest,
            None,
        )
        .unwrap_err();
        assert!(
            matches!(&err, TileServerError::RasterError(m) if m.contains("GDALReprojectImage")),
            "warp failure is surfaced, got {err:?}"
        );
    }
}
