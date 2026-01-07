use async_trait::async_trait;
use bytes::Bytes;
use gdal::raster::{Buffer, ResampleAlg};
use gdal::spatial_ref::SpatialRef;
use gdal::{Dataset, DriverManager};
use image::{ImageBuffer, RgbaImage};
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::{ColorMapConfig, ResamplingMethod, SourceConfig};
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

const WEB_MERCATOR_EXTENT: f64 = 20037508.342789244;

pub struct CogSource {
    dataset: Arc<Mutex<Dataset>>,
    metadata: TileMetadata,
    default_resampling: ResamplingMethod,
    band_count: usize,
    colormap: Option<ColorMapConfig>,
}

impl CogSource {
    pub async fn from_file(config: &SourceConfig) -> Result<Self> {
        let path = config.path.clone();
        let id = config.id.clone();
        let name = config.name.clone();
        let attribution = config.attribution.clone();
        let resampling = config.resampling.unwrap_or_default();
        let colormap = config.colormap.clone();

        let (dataset, band_count, bounds) = tokio::task::spawn_blocking(move || {
            let dataset = Dataset::open(Path::new(&path)).map_err(|e| {
                TileServerError::RasterError(format!("Failed to open COG file: {}", e))
            })?;

            let band_count = dataset.raster_count();
            if band_count == 0 {
                return Err(TileServerError::RasterError(
                    "COG file has no raster bands".to_string(),
                ));
            }

            let bounds = get_wgs84_bounds(&dataset)?;

            Ok::<_, TileServerError>((dataset, band_count, bounds))
        })
        .await
        .map_err(|e| TileServerError::RasterError(format!("Task failed: {}", e)))??;

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
            dataset: Arc::new(Mutex::new(dataset)),
            metadata,
            default_resampling: resampling,
            band_count,
            colormap,
        })
    }

    pub fn resampling(&self) -> ResamplingMethod {
        self.default_resampling
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

        let dataset = self.dataset.clone();
        let band_count = self.band_count;
        let colormap = self.colormap.clone();

        let png_data = tokio::task::spawn_blocking(move || {
            let dataset = dataset.blocking_lock();
            render_tile_from_dataset(
                &dataset,
                minx,
                miny,
                maxx,
                maxy,
                tile_size,
                band_count,
                resampling.into(),
                colormap.as_ref(),
            )
        })
        .await
        .map_err(|e| TileServerError::RasterError(format!("Task failed: {}", e)))??;

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

fn tile_to_web_mercator_bbox(z: u8, x: u32, y: u32) -> (f64, f64, f64, f64) {
    let n = 2_u32.pow(z as u32) as f64;
    let tile_size = 2.0 * WEB_MERCATOR_EXTENT / n;

    let minx = -WEB_MERCATOR_EXTENT + (x as f64) * tile_size;
    let maxx = minx + tile_size;

    let maxy = WEB_MERCATOR_EXTENT - (y as f64) * tile_size;
    let miny = maxy - tile_size;

    (minx, miny, maxx, maxy)
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
        let cmap = colormap.unwrap();
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
    fn test_tile_to_web_mercator_bbox_z1() {
        let (minx, miny, maxx, maxy) = tile_to_web_mercator_bbox(1, 0, 0);

        assert!((minx - (-WEB_MERCATOR_EXTENT)).abs() < 1e-6);
        assert!((maxy - WEB_MERCATOR_EXTENT).abs() < 1e-6);
        assert!((maxx - 0.0).abs() < 1e-6);
        assert!((miny - 0.0).abs() < 1e-6);
    }
}
