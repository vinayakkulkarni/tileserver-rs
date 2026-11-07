//! PostgreSQL Out-of-Database raster source implementation.
//!
//! This source queries PostgreSQL to get file paths to VRT/COG files,
//! then uses GDAL to render tiles from those files.

use async_trait::async_trait;
use bytes::Bytes;
use gdal::raster::Buffer;
use gdal::spatial_ref::SpatialRef;
use gdal::{Dataset, DriverManager};
use image::{ImageBuffer, RgbaImage};
use moka::future::Cache;
use std::io::Cursor;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tokio_postgres::types::Type;

use crate::config::{ColorMapConfig, PostgresOutDbRasterConfig, ResamplingMethod, RescaleMode};
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

use super::PostgresPool;

#[derive(Debug, Clone)]
struct RasterPathResult {
    filepath: String,
    rescale_min: Option<f64>,
    rescale_max: Option<f64>,
}

const WEB_MERCATOR_EXTENT: f64 = 20_037_508.342_789_244;

/// Modified time and size of a raster file, used to spot a file that was
/// rewritten at the same path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    modified: SystemTime,
    len: u64,
}

impl FileStamp {
    /// Stamp of a local file. `None` for GDAL virtual paths (`/vsicurl/…`,
    /// `/vsis3/…`) and for files that cannot be read; those rely on the TTL.
    async fn of(path: &str) -> Option<Self> {
        if path.starts_with("/vsi") {
            return None;
        }
        let meta = tokio::fs::metadata(path).await.ok()?;
        Some(Self {
            modified: meta.modified().ok()?,
            len: meta.len(),
        })
    }
}

/// An open raster file and the stamp it had when it was opened.
struct CachedFile {
    dataset: Arc<Mutex<Dataset>>,
    stamp: Option<FileStamp>,
}

/// Bounded cache of open raster files for one out-db source (issue #1599).
///
/// - LRU with `capacity` entries and a `ttl`, so file handles and memory
///   stay bounded.
/// - Each hit re-reads the file's modified time and size. When either
///   changed, the file is opened again, so a raster rebuilt at the same
///   path is served without a restart. For a VRT only the VRT file itself
///   is checked; a change to a source file it references shows up after
///   the TTL.
struct OutDbDatasetCache {
    inner: Cache<String, Arc<CachedFile>>,
}

impl OutDbDatasetCache {
    fn new(capacity: u64, ttl: Duration) -> Self {
        Self {
            inner: Cache::builder()
                .max_capacity(capacity)
                .time_to_live(ttl)
                .build(),
        }
    }

    async fn get_or_open(&self, filepath: &str) -> Result<Arc<Mutex<Dataset>>> {
        let stamp = FileStamp::of(filepath).await;
        if let Some(cached) = self.inner.get(filepath).await
            && cached.stamp == stamp
        {
            return Ok(Arc::clone(&cached.dataset));
        }

        let path = filepath.to_string();
        let dataset = tokio::task::spawn_blocking(move || {
            Dataset::open(&path).map_err(|e| {
                TileServerError::RasterError(format!("failed to open raster file {}: {}", path, e))
            })
        })
        .await
        .map_err(|e| TileServerError::RasterError(format!("task failed: {}", e)))??;

        let dataset = Arc::new(Mutex::new(dataset));
        self.inner
            .insert(
                filepath.to_string(),
                Arc::new(CachedFile {
                    dataset: Arc::clone(&dataset),
                    stamp,
                }),
            )
            .await;
        Ok(dataset)
    }
}

pub struct PostgresOutDbRasterSource {
    pool: Arc<PostgresPool>,
    metadata: TileMetadata,
    schema: String,
    function: String,
    sql_query: String,
    default_resampling: ResamplingMethod,
    colormap: Option<ColorMapConfig>,
    dataset_cache: OutDbDatasetCache,
}

impl std::fmt::Debug for PostgresOutDbRasterSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresOutDbRasterSource")
            .field("id", &self.metadata.id)
            .field("schema", &self.schema)
            .field("function", &self.function)
            .finish()
    }
}

impl PostgresOutDbRasterSource {
    /// # Errors
    ///
    /// Returns an error when the pool has no connection or the function is
    /// missing or has the wrong signature.
    pub async fn new(pool: Arc<PostgresPool>, config: &PostgresOutDbRasterConfig) -> Result<Self> {
        let conn = pool.get().await?;

        let function_name = config.function.as_ref().unwrap_or(&config.id);

        Self::verify_function(&conn, &config.schema, function_name).await?;

        let sql_query = format!(
            "SELECT * FROM \"{}\".\"{}\"($1::integer, $2::integer, $3::integer, $4::geometry, $5::jsonb)",
            config.schema, function_name
        );

        let metadata = TileMetadata {
            id: config.id.clone(),
            name: config.name.clone().unwrap_or_else(|| function_name.clone()),
            description: config.description.clone(),
            attribution: config.attribution.clone(),
            format: TileFormat::Png,
            minzoom: config.minzoom,
            maxzoom: config.maxzoom,
            bounds: config.bounds,
            center: config.bounds.map(|b| {
                let center_lon = (b[0] + b[2]) / 2.0;
                let center_lat = (b[1] + b[3]) / 2.0;
                let center_zoom = ((config.minzoom as f64 + config.maxzoom as f64) / 2.0).floor();
                [center_lon, center_lat, center_zoom]
            }),
            vector_layers: None,
        };

        tracing::info!(
            "Loaded PostgreSQL out-db raster source '{}': {}.{} (zoom {}-{})",
            config.id,
            config.schema,
            function_name,
            config.minzoom,
            config.maxzoom,
        );

        Ok(Self {
            pool,
            metadata,
            schema: config.schema.clone(),
            function: function_name.clone(),
            sql_query,
            default_resampling: ResamplingMethod::resolve(
                config.resampling,
                config.colormap.as_ref(),
            ),
            colormap: config.colormap.clone(),
            dataset_cache: OutDbDatasetCache::new(
                config.dataset_cache_size,
                Duration::from_secs(config.dataset_cache_ttl_secs),
            ),
        })
    }

    #[must_use]
    pub fn tile_query(&self) -> &str {
        &self.sql_query
    }

    async fn verify_function(
        conn: &deadpool_postgres::Object,
        schema: &str,
        function: &str,
    ) -> Result<()> {
        let query = r#"
            SELECT 1
            FROM pg_catalog.pg_proc p
            JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace
            WHERE n.nspname = $1
              AND p.proname = $2
            LIMIT 1
        "#;

        conn.query_opt(query, &[&schema, &function])
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!("failed to query function info: {}", e))
            })?
            .ok_or_else(|| {
                TileServerError::PostgresError(format!(
                    "function {}.{} not found",
                    schema, function
                ))
            })?;

        Ok(())
    }

    fn tile_to_web_mercator_bbox(z: u8, x: u32, y: u32) -> (f64, f64, f64, f64) {
        let n = 2.0_f64.powi(z as i32);
        let tile_size = (2.0 * WEB_MERCATOR_EXTENT) / n;

        let min_x = -WEB_MERCATOR_EXTENT + (x as f64 * tile_size);
        let max_x = min_x + tile_size;
        let max_y = WEB_MERCATOR_EXTENT - (y as f64 * tile_size);
        let min_y = max_y - tile_size;

        (min_x, min_y, max_x, max_y)
    }

    fn bbox_to_wkt(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> String {
        format!(
            "POLYGON(({} {}, {} {}, {} {}, {} {}, {} {}))",
            min_x, min_y, max_x, min_y, max_x, max_y, min_x, max_y, min_x, min_y
        )
    }

    /// # Errors
    ///
    /// [`TileServerError::InvalidCoordinates`] for an out-of-range tile, and
    /// [`TileServerError::PostgresError`] when the query fails.
    pub async fn get_tile_with_params(
        &self,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
        resampling: Option<ResamplingMethod>,
        query_params: Option<serde_json::Value>,
    ) -> Result<Option<TileData>> {
        let max_tile = 1u32 << z;
        if x >= max_tile || y >= max_tile {
            return Err(TileServerError::InvalidCoordinates { z, x, y });
        }

        if z < self.metadata.minzoom || z > self.metadata.maxzoom {
            return Ok(None);
        }

        let (min_x, min_y, max_x, max_y) = Self::tile_to_web_mercator_bbox(z, x, y);
        let bbox_wkt = Self::bbox_to_wkt(min_x, min_y, max_x, max_y);
        let params = query_params.unwrap_or(serde_json::json!({}));

        let conn = self.pool.get().await?;

        let param_types: &[Type] = &[Type::INT4, Type::INT4, Type::INT4, Type::TEXT, Type::JSONB];

        let prep_query = conn
            .prepare_typed_cached(&self.sql_query, param_types)
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!(
                    "failed to prepare query for {}.{}: {}",
                    self.schema, self.function, e
                ))
            })?;

        let rows = conn
            .query(
                &prep_query,
                &[&(z as i32), &(x as i32), &(y as i32), &bbox_wkt, &params],
            )
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!(
                    "failed to execute query for {}.{} at z={}, x={}, y={}: {}",
                    self.schema, self.function, z, x, y, e
                ))
            })?;

        if rows.is_empty() {
            return Ok(None);
        }

        let results: Vec<RasterPathResult> = rows
            .iter()
            .filter_map(|row| {
                let filepath = row.try_get::<_, String>("filepath").ok()?;
                Some(RasterPathResult {
                    filepath,
                    rescale_min: row.try_get::<_, f64>("rescale_min").ok(),
                    rescale_max: row.try_get::<_, f64>("rescale_max").ok(),
                })
            })
            .collect();

        if results.is_empty() {
            return Ok(None);
        }

        let resampling = resampling.unwrap_or(self.default_resampling);
        let colormap = self.colormap.clone();

        let tile_data = self
            .render_tile_from_files(
                &results, min_x, min_y, max_x, max_y, tile_size, resampling, colormap,
            )
            .await?;

        Ok(tile_data)
    }

    #[allow(clippy::too_many_arguments)]
    async fn render_tile_from_files(
        &self,
        results: &[RasterPathResult],
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        tile_size: u32,
        resampling: ResamplingMethod,
        colormap: Option<ColorMapConfig>,
    ) -> Result<Option<TileData>> {
        let bounds = [min_x, min_y, max_x, max_y];
        let mut composite: Option<RgbaImage> = None;
        for result in results {
            let dataset = self.dataset_cache.get_or_open(&result.filepath).await?;
            let dataset_guard = dataset.lock().await;
            let tile = Self::render_single_dataset(
                &dataset_guard,
                bounds,
                tile_size,
                resampling,
                colormap.as_ref(),
                (result.rescale_min, result.rescale_max),
            )?;
            if let Some(img) = tile {
                composite = Some(match composite {
                    Some(base) => overlay(base, &img),
                    None => img,
                });
            }
        }
        composite.map(|img| encode_png(&img)).transpose()
    }

    /// Warp one dataset onto the tile and turn it into RGBA: through the
    /// colormap for a single band, or as gray/RGB/RGBA bands otherwise.
    fn render_single_dataset(
        dataset: &Dataset,
        bounds: [f64; 4],
        tile_size: u32,
        resampling: ResamplingMethod,
        colormap: Option<&ColorMapConfig>,
        rescale: (Option<f64>, Option<f64>),
    ) -> Result<Option<RgbaImage>> {
        let band_count = dataset.raster_count();
        if band_count == 0 {
            return Ok(None);
        }
        let colormap = colormap.filter(|_| band_count == 1);
        let output_bands = if colormap.is_some() {
            1
        } else {
            band_count.min(4)
        };
        let Some(warped) = warp_to_tile(dataset, bounds, tile_size, output_bands, resampling)?
        else {
            return Ok(None);
        };
        let bands = read_bands(&warped, output_bands, tile_size)?;
        Ok(Some(match colormap {
            Some(cmap) => {
                let nodata = warped
                    .rasterband(1)
                    .map_err(|e| TileServerError::RasterError(format!("failed to get band: {e}")))?
                    .no_data_value();
                colorize(&bands[0], tile_size, nodata, cmap, rescale)
            }
            None => compose_bands(&bands, tile_size),
        }))
    }
}

/// Reproject `dataset` into a `tile_size`² EPSG:3857 MEM dataset covering
/// `bounds`. `None` when the warp fails, e.g. the tile is outside the raster.
fn warp_to_tile(
    dataset: &Dataset,
    [min_x, min_y, max_x, max_y]: [f64; 4],
    tile_size: u32,
    bands: usize,
    resampling: ResamplingMethod,
) -> Result<Option<Dataset>> {
    let raster_err = |what: &str, e: gdal::errors::GdalError| {
        TileServerError::RasterError(format!("failed to {what}: {e}"))
    };
    let web_mercator =
        SpatialRef::from_epsg(3857).map_err(|e| raster_err("create EPSG:3857", e))?;
    let mem_driver =
        DriverManager::get_driver_by_name("MEM").map_err(|e| raster_err("get MEM driver", e))?;
    let size = tile_size as usize;
    let mut warped = mem_driver
        .create_with_band_type::<f64, _>("", size, size, bands)
        .map_err(|e| raster_err("create output", e))?;
    let pixel_x = (max_x - min_x) / f64::from(tile_size);
    let pixel_y = (max_y - min_y) / f64::from(tile_size);
    warped
        .set_geo_transform(&[min_x, pixel_x, 0.0, max_y, 0.0, -pixel_y])
        .map_err(|e| raster_err("set geotransform", e))?;
    warped
        .set_spatial_ref(&web_mercator)
        .map_err(|e| raster_err("set SRS", e))?;
    if let Err(e) = crate::sources::warp::reproject(dataset, &warped, resampling) {
        tracing::debug!(
            "Reproject failed (tile may be outside raster bounds): {}",
            e
        );
        return Ok(None);
    }
    Ok(Some(warped))
}

/// Read bands `1..=count` of `warped` as `f64`, row-major.
fn read_bands(warped: &Dataset, count: usize, tile_size: u32) -> Result<Vec<Vec<f64>>> {
    let size = tile_size as usize;
    (1..=count)
        .map(|i| {
            let band = warped.rasterband(i).map_err(|e| {
                TileServerError::RasterError(format!("failed to get band {i}: {e}"))
            })?;
            let buffer: Buffer<f64> = band
                .read_as::<f64>((0, 0), (size, size), (size, size), None)
                .map_err(|e| {
                    TileServerError::RasterError(format!("failed to read band {i}: {e}"))
                })?;
            Ok(buffer.data().to_vec())
        })
        .collect()
}

/// How single-band values map onto the colormap's stops.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Rescale {
    /// Values are already in colormap units.
    Raw,
    /// Normalize `[min, max]` from the database row to `[0, 1]`.
    Window { min: f64, range: f64 },
}

fn rescale_for(cmap: &ColorMapConfig, (min, max): (Option<f64>, Option<f64>)) -> Rescale {
    match (cmap.rescale_mode, min, max) {
        (RescaleMode::Dynamic, Some(min), Some(max)) if (max - min).abs() > f64::EPSILON => {
            Rescale::Window {
                min,
                range: max - min,
            }
        }
        _ => Rescale::Raw,
    }
}

/// Color of one single-band value.
fn colormap_pixel(
    value: f64,
    nodata: Option<f64>,
    cmap: &ColorMapConfig,
    rescale: Rescale,
) -> [u8; 4] {
    if nodata.is_some_and(|nd| (value - nd).abs() < f64::EPSILON) {
        return cmap
            .nodata_color
            .as_ref()
            .and_then(|c| ColorMapConfig::parse_color(c))
            .unwrap_or([0, 0, 0, 0]);
    }
    match rescale {
        Rescale::Raw => cmap.get_color(value),
        Rescale::Window { min, range } => cmap.get_color(((value - min) / range).clamp(0.0, 1.0)),
    }
}

/// Single band through the colormap.
fn colorize(
    values: &[f64],
    tile_size: u32,
    nodata: Option<f64>,
    cmap: &ColorMapConfig,
    rescale: (Option<f64>, Option<f64>),
) -> RgbaImage {
    let rescale = rescale_for(cmap, rescale);
    ImageBuffer::from_fn(tile_size, tile_size, |x, y| {
        let value = values[(y * tile_size + x) as usize];
        image::Rgba(colormap_pixel(value, nodata, cmap, rescale))
    })
}

/// 1 band → gray, 3 → RGB, 4 → RGBA; values clamped to `0..=255`.
fn compose_bands(bands: &[Vec<f64>], tile_size: u32) -> RgbaImage {
    let channel = |b: usize, idx: usize| bands[b][idx].clamp(0.0, 255.0) as u8;
    ImageBuffer::from_fn(tile_size, tile_size, |x, y| {
        let idx = (y * tile_size + x) as usize;
        image::Rgba(match bands.len() {
            n if n >= 4 => [
                channel(0, idx),
                channel(1, idx),
                channel(2, idx),
                channel(3, idx),
            ],
            3 => [channel(0, idx), channel(1, idx), channel(2, idx), 255],
            _ => {
                let gray = channel(0, idx);
                [gray, gray, gray, 255]
            }
        })
    })
}

/// Draw the non-transparent pixels of `top` over `base`.
fn overlay(mut base: RgbaImage, top: &RgbaImage) -> RgbaImage {
    for (x, y, pixel) in top.enumerate_pixels() {
        if pixel[3] > 0 {
            base.put_pixel(x, y, *pixel);
        }
    }
    base
}

fn encode_png(img: &RgbaImage) -> Result<TileData> {
    let mut png = Cursor::new(Vec::new());
    img.write_to(&mut png, image::ImageFormat::Png)
        .map_err(|e| TileServerError::RasterError(format!("failed to encode PNG: {e}")))?;
    Ok(TileData {
        data: Bytes::from(png.into_inner()),
        format: TileFormat::Png,
        compression: TileCompression::None,
    })
}

#[async_trait]
impl TileSource for PostgresOutDbRasterSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        self.get_tile_with_params(z, x, y, 256, None, None).await
    }

    fn metadata(&self) -> &TileMetadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_to_web_mercator_bbox_z0() {
        let (min_x, min_y, max_x, max_y) =
            PostgresOutDbRasterSource::tile_to_web_mercator_bbox(0, 0, 0);
        assert!((min_x - (-WEB_MERCATOR_EXTENT)).abs() < 1.0);
        assert!((max_x - WEB_MERCATOR_EXTENT).abs() < 1.0);
        assert!((min_y - (-WEB_MERCATOR_EXTENT)).abs() < 1.0);
        assert!((max_y - WEB_MERCATOR_EXTENT).abs() < 1.0);
    }

    #[test]
    fn test_tile_to_web_mercator_bbox_z1() {
        let (min_x, min_y, max_x, _max_y) =
            PostgresOutDbRasterSource::tile_to_web_mercator_bbox(1, 0, 0);
        let tile_width = max_x - min_x;
        let expected_width = WEB_MERCATOR_EXTENT;
        assert!((tile_width - expected_width).abs() < 1.0);
        assert!((min_y - 0.0).abs() < 1.0);
    }

    #[test]
    fn test_tile_to_web_mercator_bbox_z10() {
        let (min_x, min_y, max_x, max_y) =
            PostgresOutDbRasterSource::tile_to_web_mercator_bbox(10, 512, 512);
        let tile_size = max_x - min_x;
        let expected_size = (2.0 * WEB_MERCATOR_EXTENT) / 1024.0;
        assert!((tile_size - expected_size).abs() < 1.0);
        assert!(max_y > min_y);
    }

    #[test]
    fn test_tile_bbox_symmetry() {
        let (min_x, min_y, max_x, max_y) =
            PostgresOutDbRasterSource::tile_to_web_mercator_bbox(2, 1, 1);
        let tile_width = max_x - min_x;
        let tile_height = max_y - min_y;
        assert!((tile_width - tile_height).abs() < 0.001);
    }

    #[test]
    fn test_bbox_to_wkt_format() {
        let wkt = PostgresOutDbRasterSource::bbox_to_wkt(0.0, 0.0, 1.0, 1.0);
        assert!(wkt.starts_with("POLYGON(("));
        assert!(wkt.ends_with("))"));
        assert!(wkt.contains("0 0"));
        assert!(wkt.contains("1 0"));
        assert!(wkt.contains("1 1"));
        assert!(wkt.contains("0 1"));
    }

    #[test]
    fn test_bbox_to_wkt_negative_coords() {
        let wkt = PostgresOutDbRasterSource::bbox_to_wkt(-180.0, -90.0, 180.0, 90.0);
        assert!(wkt.contains("-180 -90"));
        assert!(wkt.contains("180 90"));
    }

    #[test]
    fn test_bbox_to_wkt_web_mercator_extent() {
        let wkt = PostgresOutDbRasterSource::bbox_to_wkt(
            -WEB_MERCATOR_EXTENT,
            -WEB_MERCATOR_EXTENT,
            WEB_MERCATOR_EXTENT,
            WEB_MERCATOR_EXTENT,
        );
        assert!(wkt.starts_with("POLYGON(("));
        assert!(wkt.contains("-20037508"));
        assert!(wkt.contains("20037508"));
    }

    #[test]
    fn test_tile_bbox_coverage_z0() {
        let (min_x, _min_y, max_x, _max_y) =
            PostgresOutDbRasterSource::tile_to_web_mercator_bbox(0, 0, 0);
        let total_width = 2.0 * WEB_MERCATOR_EXTENT;
        let bbox_width = max_x - min_x;
        assert!((bbox_width - total_width).abs() < 1.0);
    }

    #[test]
    fn test_tile_bbox_adjacent_tiles() {
        let (_, _, max_x_0, _) = PostgresOutDbRasterSource::tile_to_web_mercator_bbox(2, 0, 0);
        let (min_x_1, _, _, _) = PostgresOutDbRasterSource::tile_to_web_mercator_bbox(2, 1, 0);
        assert!((max_x_0 - min_x_1).abs() < 0.001);
    }

    #[test]
    fn test_tile_bbox_valid_range() {
        for z in 0..5 {
            let max_tile = 1u32 << z;
            for x in 0..max_tile.min(4) {
                for y in 0..max_tile.min(4) {
                    let (min_x, min_y, max_x, max_y) =
                        PostgresOutDbRasterSource::tile_to_web_mercator_bbox(z, x, y);
                    assert!(min_x < max_x, "min_x should be less than max_x");
                    assert!(min_y < max_y, "min_y should be less than max_y");
                    assert!(min_x >= -WEB_MERCATOR_EXTENT - 1.0);
                    assert!(max_x <= WEB_MERCATOR_EXTENT + 1.0);
                }
            }
        }
    }

    // ---- Issue #1599 ----

    use crate::config::{ColorMapEntry, ColorMapType};
    use std::collections::BTreeSet;

    /// Extent of the test rasters and tiles, in EPSG:3857 metres.
    const EXTENT: f64 = 1600.0;

    /// Write a single-band Byte GeoTIFF over `[0, EXTENT]²` with `size`²
    /// pixels: the left half `left`, the right half `right`. NoData is 0.
    fn write_two_class_tiff(path: &std::path::Path, size: usize, left: u8, right: u8) {
        let driver = DriverManager::get_driver_by_name("GTiff").unwrap();
        let mut ds = driver
            .create_with_band_type::<u8, _>(path, size, size, 1)
            .unwrap();
        let px = EXTENT / size as f64;
        ds.set_geo_transform(&[0.0, px, 0.0, EXTENT, 0.0, -px])
            .unwrap();
        ds.set_spatial_ref(&SpatialRef::from_epsg(3857).unwrap())
            .unwrap();
        let data: Vec<u8> = (0..size * size)
            .map(|i| if i % size < size / 2 { left } else { right })
            .collect();
        let mut band = ds.rasterband(1).unwrap();
        band.set_no_data_value(Some(0.0)).unwrap();
        band.write((0, 0), (size, size), &mut Buffer::new((size, size), data))
            .unwrap();
    }

    /// The colormap from the issue: classes 1–4 coloured, 0 and 5 clear.
    fn landcover_colormap() -> ColorMapConfig {
        let entry = |value: f64, color: &str| ColorMapEntry {
            value,
            color: color.to_string(),
        };
        ColorMapConfig {
            map_type: ColorMapType::Discrete,
            rescale_mode: RescaleMode::None,
            entries: vec![
                entry(0.0, "#00000000"),
                entry(1.0, "#FD080C"),
                entry(2.0, "#1D90FF"),
                entry(3.0, "#22FDD5"),
                entry(4.0, "#FAD200"),
                entry(5.0, "#00000000"),
            ],
            nodata_color: Some("#00000000".to_string()),
        }
    }

    /// Render a 256² tile over the whole test raster and return the set of
    /// output colours.
    fn rendered_colours(method: ResamplingMethod) -> BTreeSet<[u8; 4]> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("landcover.tif");
        // 8 source pixels → 256 output pixels: a strong upsample, so any
        // interpolation shows up along the 1↔5 boundary.
        write_two_class_tiff(&path, 8, 1, 5);
        let ds = Dataset::open(&path).unwrap();
        let cmap = landcover_colormap();
        let img = PostgresOutDbRasterSource::render_single_dataset(
            &ds,
            [0.0, 0.0, EXTENT, EXTENT],
            256,
            method,
            Some(&cmap),
            (None, None),
        )
        .unwrap()
        .expect("tile covers the raster");
        img.pixels().map(|p| p.0).collect()
    }

    #[test]
    fn nearest_renders_only_source_classes() {
        // Source has only classes 1 and 5: class-1 red and clear.
        assert_eq!(
            rendered_colours(ResamplingMethod::Nearest),
            BTreeSet::from([[0xFD, 0x08, 0x0C, 0xFF], [0, 0, 0, 0]])
        );
    }

    #[test]
    fn bilinear_is_applied_and_creates_in_between_classes() {
        // Before the fix every method rendered like this. Now it only
        // happens when bilinear is asked for.
        let colours = rendered_colours(ResamplingMethod::Bilinear);
        let in_between = [
            [0x1D, 0x90, 0xFF, 0xFF],
            [0x22, 0xFD, 0xD5, 0xFF],
            [0xFA, 0xD2, 0x00, 0xFF],
        ];
        assert!(
            in_between.iter().any(|c| colours.contains(c)),
            "bilinear must interpolate between classes 1 and 5, got {colours:?}"
        );
    }

    /// Read the pixel value at the left edge of a cached dataset.
    async fn first_pixel(dataset: &Arc<Mutex<Dataset>>) -> u8 {
        let guard = dataset.lock().await;
        let buf: Buffer<u8> = guard
            .rasterband(1)
            .unwrap()
            .read_as::<u8>((0, 0), (1, 1), (1, 1), None)
            .unwrap();
        buf.data()[0]
    }

    #[tokio::test]
    async fn cache_reopens_a_file_rewritten_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rebuilt.tif");
        let p = path.to_string_lossy().to_string();
        write_two_class_tiff(&path, 8, 1, 5);

        let cache = OutDbDatasetCache::new(10, Duration::from_secs(600));
        let first = cache.get_or_open(&p).await.unwrap();
        assert_eq!(first_pixel(&first).await, 1);
        let again = cache.get_or_open(&p).await.unwrap();
        assert!(
            Arc::ptr_eq(&first, &again),
            "unchanged file is served from cache"
        );

        // Rebuild at the same path with new content. Set the modified time
        // explicitly so the test does not depend on filesystem time
        // resolution.
        write_two_class_tiff(&path, 8, 3, 5);
        let later = SystemTime::now() + Duration::from_secs(5);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(later)
            .unwrap();

        let rebuilt = cache.get_or_open(&p).await.unwrap();
        assert!(
            !Arc::ptr_eq(&first, &rebuilt),
            "changed file must be reopened"
        );
        assert_eq!(first_pixel(&rebuilt).await, 3, "new content is served");
    }

    #[tokio::test]
    async fn cache_stays_within_capacity() {
        let dir = tempfile::tempdir().unwrap();
        let cache = OutDbDatasetCache::new(3, Duration::from_secs(600));
        for i in 0..10 {
            let path = dir.path().join(format!("f{i}.tif"));
            write_two_class_tiff(&path, 8, 1, 5);
            cache.get_or_open(&path.to_string_lossy()).await.unwrap();
        }
        cache.inner.run_pending_tasks().await;
        assert!(
            cache.inner.entry_count() <= 3,
            "cache holds {} files, limit is 3",
            cache.inner.entry_count()
        );
    }

    #[tokio::test]
    async fn cache_with_zero_capacity_still_opens_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nocache.tif");
        write_two_class_tiff(&path, 8, 1, 5);
        let cache = OutDbDatasetCache::new(0, Duration::from_secs(600));
        let ds = cache.get_or_open(&path.to_string_lossy()).await.unwrap();
        assert_eq!(first_pixel(&ds).await, 1);
    }

    #[tokio::test]
    async fn virtual_paths_have_no_stamp() {
        assert_eq!(
            FileStamp::of("/vsicurl/https://example.com/a.tif").await,
            None
        );
        assert_eq!(FileStamp::of("/nonexistent/file.tif").await, None);
    }

    fn ramp() -> ColorMapConfig {
        toml::from_str(
            r##"
rescale_mode = "dynamic"
nodata_color = "#00000000"
entries = [
  { value = 0.0, color = "#000000" },
  { value = 1.0, color = "#ffffff" },
]
"##,
        )
        .unwrap()
    }

    #[test]
    fn rescale_uses_the_row_window_only_when_dynamic_and_non_empty() {
        let cmap = ramp();
        assert_eq!(
            rescale_for(&cmap, (Some(10.0), Some(20.0))),
            Rescale::Window {
                min: 10.0,
                range: 10.0
            }
        );
        assert_eq!(rescale_for(&cmap, (Some(5.0), Some(5.0))), Rescale::Raw);
        assert_eq!(rescale_for(&cmap, (None, Some(5.0))), Rescale::Raw);
        let mut fixed = ramp();
        fixed.rescale_mode = RescaleMode::None;
        assert_eq!(rescale_for(&fixed, (Some(0.0), Some(1.0))), Rescale::Raw);
    }

    #[test]
    fn colormap_pixel_handles_nodata_window_and_clamp() {
        let cmap = ramp();
        let window = Rescale::Window {
            min: 100.0,
            range: 100.0,
        };
        assert_eq!(
            colormap_pixel(-9.0, Some(-9.0), &cmap, window),
            [0, 0, 0, 0]
        );
        assert_eq!(
            colormap_pixel(200.0, None, &cmap, window),
            cmap.get_color(1.0)
        );
        assert_eq!(
            colormap_pixel(500.0, None, &cmap, window),
            cmap.get_color(1.0)
        );
        assert_eq!(
            colormap_pixel(0.0, None, &cmap, window),
            cmap.get_color(0.0)
        );
        assert_eq!(
            colormap_pixel(0.5, None, &cmap, Rescale::Raw),
            cmap.get_color(0.5)
        );
    }

    #[test]
    fn compose_bands_maps_gray_rgb_and_rgba() {
        let gray = compose_bands(&[vec![300.0]], 1);
        assert_eq!(gray.get_pixel(0, 0).0, [255, 255, 255, 255]);
        let rgb = compose_bands(&[vec![1.0], vec![2.0], vec![3.0]], 1);
        assert_eq!(rgb.get_pixel(0, 0).0, [1, 2, 3, 255]);
        let rgba = compose_bands(&[vec![1.0], vec![2.0], vec![3.0], vec![-4.0]], 1);
        assert_eq!(rgba.get_pixel(0, 0).0, [1, 2, 3, 0]);
    }

    #[test]
    fn overlay_keeps_base_under_transparent_pixels_and_encodes_png() {
        let base = ImageBuffer::from_pixel(2, 1, image::Rgba([1, 1, 1, 255]));
        let mut top: RgbaImage = ImageBuffer::new(2, 1);
        top.put_pixel(1, 0, image::Rgba([9, 9, 9, 255]));
        let out = overlay(base, &top);
        assert_eq!(out.get_pixel(0, 0).0, [1, 1, 1, 255]);
        assert_eq!(out.get_pixel(1, 0).0, [9, 9, 9, 255]);
        let png = encode_png(&out).unwrap();
        assert_eq!(png.format, TileFormat::Png);
        assert!(png.data.starts_with(&[0x89, b'P', b'N', b'G']));
    }

    #[test]
    fn colorize_reads_values_row_major() {
        let cmap = ramp();
        let img = colorize(&[0.0, 1.0], 1, None, &cmap, (None, None));
        assert_eq!(img.get_pixel(0, 0).0, cmap.get_color(0.0));
        let img = colorize(&[0.0, 1.0, 0.0, 1.0], 2, None, &cmap, (None, None));
        assert_eq!(img.get_pixel(1, 0).0, cmap.get_color(1.0));
    }
}
