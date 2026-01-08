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
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
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

pub struct PostgresOutDbRasterSource {
    pool: Arc<PostgresPool>,
    metadata: TileMetadata,
    schema: String,
    function: String,
    sql_query: String,
    default_resampling: ResamplingMethod,
    colormap: Option<ColorMapConfig>,
    dataset_cache: Arc<Mutex<HashMap<String, Arc<Mutex<Dataset>>>>>,
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
            default_resampling: config.resampling.unwrap_or_default(),
            colormap: config.colormap.clone(),
            dataset_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

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
                TileServerError::PostgresError(format!("Failed to query function info: {}", e))
            })?
            .ok_or_else(|| {
                TileServerError::PostgresError(format!(
                    "Function {}.{} not found",
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

    async fn get_or_open_dataset(&self, filepath: &str) -> Result<Arc<Mutex<Dataset>>> {
        let mut cache = self.dataset_cache.lock().await;

        if let Some(dataset) = cache.get(filepath) {
            return Ok(dataset.clone());
        }

        let path = filepath.to_string();
        let dataset = tokio::task::spawn_blocking(move || {
            Dataset::open(&path).map_err(|e| {
                TileServerError::RasterError(format!("Failed to open raster file {}: {}", path, e))
            })
        })
        .await
        .map_err(|e| TileServerError::RasterError(format!("Task failed: {}", e)))??;

        let dataset = Arc::new(Mutex::new(dataset));
        cache.insert(filepath.to_string(), dataset.clone());

        Ok(dataset)
    }

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
                    "Failed to prepare query for {}.{}: {}",
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
                    "Failed to execute query for {}.{} at z={}, x={}, y={}: {}",
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

        let _resample = resampling.unwrap_or(self.default_resampling);
        let colormap = self.colormap.clone();

        let tile_data = self
            .render_tile_from_files(&results, min_x, min_y, max_x, max_y, tile_size, colormap)
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
        colormap: Option<ColorMapConfig>,
    ) -> Result<Option<TileData>> {
        let mut composite_image: Option<RgbaImage> = None;

        for result in results {
            let dataset = self.get_or_open_dataset(&result.filepath).await?;
            let dataset_guard = dataset.lock().await;

            let tile_image = Self::render_single_dataset(
                &dataset_guard,
                min_x,
                min_y,
                max_x,
                max_y,
                tile_size,
                colormap.as_ref(),
                result.rescale_min,
                result.rescale_max,
            )?;

            if let Some(img) = tile_image {
                match &mut composite_image {
                    Some(composite) => {
                        for (x, y, pixel) in img.enumerate_pixels() {
                            if pixel[3] > 0 {
                                composite.put_pixel(x, y, *pixel);
                            }
                        }
                    }
                    None => {
                        composite_image = Some(img);
                    }
                }
            }
        }

        match composite_image {
            Some(img) => {
                let mut png_data = Cursor::new(Vec::new());
                img.write_to(&mut png_data, image::ImageFormat::Png)
                    .map_err(|e| {
                        TileServerError::RasterError(format!("Failed to encode PNG: {}", e))
                    })?;

                Ok(Some(TileData {
                    data: Bytes::from(png_data.into_inner()),
                    format: TileFormat::Png,
                    compression: TileCompression::None,
                }))
            }
            None => Ok(None),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_single_dataset(
        dataset: &Dataset,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        tile_size: u32,
        colormap: Option<&ColorMapConfig>,
        rescale_min: Option<f64>,
        rescale_max: Option<f64>,
    ) -> Result<Option<RgbaImage>> {
        let band_count = dataset.raster_count();

        if band_count == 0 {
            return Ok(None);
        }

        let web_mercator = SpatialRef::from_epsg(3857).map_err(|e| {
            TileServerError::RasterError(format!("Failed to create EPSG:3857: {}", e))
        })?;

        let mem_driver = DriverManager::get_driver_by_name("MEM").map_err(|e| {
            TileServerError::RasterError(format!("Failed to get MEM driver: {}", e))
        })?;

        let use_colormap = colormap.is_some() && band_count == 1;
        let output_bands = if use_colormap { 1 } else { band_count.min(4) };

        let mut warped = mem_driver
            .create_with_band_type::<f64, _>(
                "",
                tile_size as usize,
                tile_size as usize,
                output_bands,
            )
            .map_err(|e| TileServerError::RasterError(format!("Failed to create output: {}", e)))?;

        let pixel_size_x = (max_x - min_x) / tile_size as f64;
        let pixel_size_y = (max_y - min_y) / tile_size as f64;
        let geo_transform = [min_x, pixel_size_x, 0.0, max_y, 0.0, -pixel_size_y];

        warped.set_geo_transform(&geo_transform).map_err(|e| {
            TileServerError::RasterError(format!("Failed to set geotransform: {}", e))
        })?;
        warped
            .set_spatial_ref(&web_mercator)
            .map_err(|e| TileServerError::RasterError(format!("Failed to set SRS: {}", e)))?;

        if let Err(e) = gdal::raster::reproject(dataset, &warped) {
            tracing::debug!(
                "Reproject failed (tile may be outside raster bounds): {}",
                e
            );
            return Ok(None);
        }

        let mut img: RgbaImage = ImageBuffer::new(tile_size, tile_size);

        if use_colormap {
            let cmap = colormap.unwrap();
            let band = warped
                .rasterband(1)
                .map_err(|e| TileServerError::RasterError(format!("Failed to get band: {}", e)))?;

            let buffer: Buffer<f64> = band
                .read_as::<f64>(
                    (0, 0),
                    (tile_size as usize, tile_size as usize),
                    (tile_size as usize, tile_size as usize),
                    None,
                )
                .map_err(|e| TileServerError::RasterError(format!("Failed to read band: {}", e)))?;

            let nodata = band.no_data_value();

            let use_dynamic_rescale = cmap.rescale_mode == RescaleMode::Dynamic
                && rescale_min.is_some()
                && rescale_max.is_some();
            let use_no_rescale = cmap.rescale_mode == RescaleMode::None;
            let (dyn_min, dyn_max) = if use_dynamic_rescale {
                (rescale_min.unwrap(), rescale_max.unwrap())
            } else {
                (0.0, 1.0)
            };
            let dyn_range = dyn_max - dyn_min;

            for y in 0..tile_size {
                for x in 0..tile_size {
                    let idx = (y * tile_size + x) as usize;
                    let raw_value = buffer.data()[idx];

                    let color = if nodata
                        .map(|nd| (raw_value - nd).abs() < f64::EPSILON)
                        .unwrap_or(false)
                    {
                        cmap.nodata_color
                            .as_ref()
                            .and_then(|c| ColorMapConfig::parse_color(c))
                            .unwrap_or([0, 0, 0, 0])
                    } else if use_no_rescale {
                        cmap.get_color(raw_value)
                    } else if use_dynamic_rescale && dyn_range.abs() > f64::EPSILON {
                        let normalized = ((raw_value - dyn_min) / dyn_range).clamp(0.0, 1.0);
                        cmap.get_color(normalized)
                    } else {
                        cmap.get_color(raw_value)
                    };

                    img.put_pixel(x, y, image::Rgba(color));
                }
            }
        } else {
            let bands_to_read = band_count.min(4);
            let mut band_data: Vec<Buffer<f64>> = Vec::with_capacity(bands_to_read);

            for i in 1..=bands_to_read {
                let band = warped.rasterband(i).map_err(|e| {
                    TileServerError::RasterError(format!("Failed to get band {}: {}", i, e))
                })?;

                let buffer: Buffer<f64> = band
                    .read_as::<f64>(
                        (0, 0),
                        (tile_size as usize, tile_size as usize),
                        (tile_size as usize, tile_size as usize),
                        None,
                    )
                    .map_err(|e| {
                        TileServerError::RasterError(format!("Failed to read band {}: {}", i, e))
                    })?;

                band_data.push(buffer);
            }

            for y in 0..tile_size {
                for x in 0..tile_size {
                    let idx = (y * tile_size + x) as usize;

                    let (r, g, b, a) = if band_data.len() >= 4 {
                        (
                            band_data[0].data()[idx].clamp(0.0, 255.0) as u8,
                            band_data[1].data()[idx].clamp(0.0, 255.0) as u8,
                            band_data[2].data()[idx].clamp(0.0, 255.0) as u8,
                            band_data[3].data()[idx].clamp(0.0, 255.0) as u8,
                        )
                    } else if band_data.len() >= 3 {
                        (
                            band_data[0].data()[idx].clamp(0.0, 255.0) as u8,
                            band_data[1].data()[idx].clamp(0.0, 255.0) as u8,
                            band_data[2].data()[idx].clamp(0.0, 255.0) as u8,
                            255,
                        )
                    } else {
                        let gray = band_data[0].data()[idx].clamp(0.0, 255.0) as u8;
                        (gray, gray, gray, 255)
                    };

                    img.put_pixel(x, y, image::Rgba([r, g, b, a]));
                }
            }
        }

        Ok(Some(img))
    }
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
}
