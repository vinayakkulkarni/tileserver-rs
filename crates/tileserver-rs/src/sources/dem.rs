//! Digital Elevation Model (DEM) raster tile source.
//!
//! Reads float elevation from a GDAL raster (COG / GeoTIFF, optionally
//! referencing another already-loaded `[[sources]]` entry) and re-encodes
//! each pixel as a Terrarium or Mapbox-RGB (`terrain-rgb`) PNG tile that
//! MapLibre GL JS consumes as a `raster-dem` source.
//!
//! ## Encoding correctness
//!
//! Both encodings are bit-exact to what MapLibre GL JS decodes
//! (`src/data/dem_data.ts`):
//!
//! - **Terrarium** (`decoded = R*256 + G + B/256 - 32768`), precision
//!   `1/256 m` (~0.0039 m). Nodata sentinel `(0, 0, 0)` = −32768 m.
//! - **Mapbox-RGB** (`decoded = -10000 + (R*65536 + G*256 + B) * 0.1`),
//!   precision `0.1 m`. Nodata sentinel `(1, 134, 160)` = sea level.
//!
//! ## The round-before-truncate rule
//!
//! The encoders round the scaled elevation to the nearest integer BEFORE
//! splitting into bytes. Truncating instead produces a systematic 1-LSB
//! (one elevation-interval) error — the classic `rio-rgbify` rounding bug.
//! MapLibre's own `packDEMData` rounds for exactly this reason.
//!
//! ## Why a sentinel RGB and not alpha
//!
//! MapLibre's `DEMData` constructor only reads R, G, B — it IGNORES the
//! alpha channel. A transparent pixel still decodes its RGB to a (usually
//! garbage) elevation. So nodata pixels MUST carry a sentinel RGB that
//! decodes to an obviously-out-of-range elevation; alpha is cosmetic only.

use async_trait::async_trait;
use bytes::Bytes;
use gdal::Dataset;
use gdal::raster::{Buffer, ResampleAlg};
use gdal::spatial_ref::SpatialRef;
use gdal::{DriverManager, raster::reproject};
use image::{ImageBuffer, RgbaImage};
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::{DemEncoding, ResamplingMethod, SourceConfig};
use crate::error::{Result, TileServerError};
use crate::sources::cog::CogSource;
use crate::sources::dataset_cache;
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

const WEB_MERCATOR_EXTENT: f64 = 20037508.342789244;

/// Terrarium base offset in metres (`decoded = packed - 32768`).
const TERRARIUM_BASE: f64 = 32768.0;
/// Mapbox-RGB base offset in metres (`decoded = -10000 + packed * 0.1`).
const MAPBOX_BASE: f64 = -10000.0;
/// Mapbox-RGB precision in metres per least-significant-bit.
const MAPBOX_INTERVAL: f64 = 0.1;
/// Largest packed value representable in 24 bits (`2^24 - 1`).
const MAX_U24: f64 = 16_777_215.0;

/// Default Terrarium nodata sentinel: `(0, 0, 0)` decodes to −32768 m.
const TERRARIUM_NODATA: [u8; 3] = [0, 0, 0];
/// Default Mapbox-RGB nodata sentinel: `(1, 134, 160)` decodes to 0 m
/// (sea level), the Mapbox convention for "no data / ocean".
const MAPBOX_NODATA: [u8; 3] = [1, 134, 160];

/// Encode an elevation (metres) as a Terrarium RGB triplet.
///
/// `decoded = R*256 + G + B/256 - 32768`. The packed value is rounded to
/// the nearest `1/256 m` before splitting into bytes (round-before-truncate)
/// and clamped to `[0, 0xFFFFFF]` so absurd inputs can't wrap a byte.
#[must_use]
pub fn encode_terrarium(elevation: f64) -> [u8; 3] {
    // Work in 1/256-m units so the blue byte is an exact integer; round to
    // nearest before splitting (the round-before-truncate rule), then clamp
    // to the 24-bit packed range [0, 256^3 - 1].
    let units = ((elevation + TERRARIUM_BASE) * 256.0).round();
    let packed = units.clamp(0.0, MAX_U24) as u32;
    let r = (packed >> 16) & 0xFF;
    let g = (packed >> 8) & 0xFF;
    let b = packed & 0xFF;
    [r as u8, g as u8, b as u8]
}

/// Encode an elevation (metres) as a Mapbox-RGB (`terrain-rgb`) triplet.
///
/// `decoded = -10000 + (R*65536 + G*256 + B) * 0.1`. Rounds to the
/// nearest 0.1 m then clamps to the 24-bit packed range.
#[must_use]
pub fn encode_mapbox(elevation: f64) -> [u8; 3] {
    let packed = ((elevation - MAPBOX_BASE) / MAPBOX_INTERVAL)
        .round()
        .clamp(0.0, MAX_U24) as u32;
    let r = (packed >> 16) & 0xFF;
    let g = (packed >> 8) & 0xFF;
    let b = packed & 0xFF;
    [r as u8, g as u8, b as u8]
}

/// Decode a Terrarium RGB triplet back to metres (for round-trip tests).
#[must_use]
pub fn decode_terrarium(rgb: [u8; 3]) -> f64 {
    f64::from(rgb[0]) * 256.0 + f64::from(rgb[1]) + f64::from(rgb[2]) / 256.0 - TERRARIUM_BASE
}

/// Decode a Mapbox-RGB triplet back to metres (for round-trip tests).
#[must_use]
pub fn decode_mapbox(rgb: [u8; 3]) -> f64 {
    MAPBOX_BASE
        + (f64::from(rgb[0]) * 65536.0 + f64::from(rgb[1]) * 256.0 + f64::from(rgb[2]))
            * MAPBOX_INTERVAL
}

/// Encode an elevation with the chosen encoding.
#[must_use]
pub fn encode_elevation(elevation: f64, encoding: DemEncoding) -> [u8; 3] {
    match encoding {
        DemEncoding::Terrarium => encode_terrarium(elevation),
        DemEncoding::MapboxRgb => encode_mapbox(elevation),
    }
}

/// Decode an RGB triplet with the chosen encoding.
#[must_use]
pub fn decode_elevation(rgb: [u8; 3], encoding: DemEncoding) -> f64 {
    match encoding {
        DemEncoding::Terrarium => decode_terrarium(rgb),
        DemEncoding::MapboxRgb => decode_mapbox(rgb),
    }
}

/// The RGBA sentinel written for nodata pixels.
///
/// `override_color` (the source's `dem_nodata_color`) wins when present;
/// otherwise the encoding-specific default sentinel is used with a fully
/// transparent alpha so a client that DOES honour alpha hides the pixel.
#[must_use]
pub fn nodata_rgba(encoding: DemEncoding, override_color: Option<[u8; 4]>) -> [u8; 4] {
    if let Some(rgba) = override_color {
        return rgba;
    }
    let [r, g, b] = match encoding {
        DemEncoding::Terrarium => TERRARIUM_NODATA,
        DemEncoding::MapboxRgb => MAPBOX_NODATA,
    };
    [r, g, b, 0]
}

/// Tunables that turn a float elevation grid into an encoded RGBA buffer.
#[derive(Debug, Clone, Copy)]
pub struct EncodeParams {
    pub encoding: DemEncoding,
    pub scale: f64,
    pub offset: f64,
    pub nodata_value: Option<f64>,
    pub nodata_rgba: [u8; 4],
}

/// Encode a row-major float elevation grid into a row-major RGBA byte buffer.
///
/// A pixel equal to `nodata_value`, or non-finite (NaN/±inf from a GDAL
/// masked read), is written as the nodata sentinel. Every other pixel is
/// `(value * scale + offset)` fed through the chosen encoder with opaque alpha.
#[must_use]
pub fn encode_pixels(values: &[f64], params: &EncodeParams) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for &raw in values {
        let is_nodata = !raw.is_finite()
            || params
                .nodata_value
                .is_some_and(|nd| (raw - nd).abs() < f64::EPSILON);
        if is_nodata {
            out.extend_from_slice(&params.nodata_rgba);
        } else {
            let elevation = raw * params.scale + params.offset;
            let [r, g, b] = encode_elevation(elevation, params.encoding);
            out.extend_from_slice(&[r, g, b, 255]);
        }
    }
    out
}

/// A DEM tile source: float elevation in, Terrarium/Mapbox-RGB PNG out.
pub struct DemSource {
    dataset: Arc<Mutex<Dataset>>,
    metadata: TileMetadata,
    resampling: ResamplingMethod,
    band: usize,
    params: EncodeParams,
}

impl DemSource {
    /// Build a DEM source from config, resolving `input_source` (the id of
    /// another already-loaded raster source) against `loaded` when present,
    /// otherwise opening the source's own `path`.
    pub async fn from_config(
        config: &SourceConfig,
        loaded: &std::collections::HashMap<String, Arc<dyn TileSource>>,
    ) -> Result<Self> {
        let dataset = Self::resolve_dataset(config, loaded).await?;

        let dataset_for_inspect = Arc::clone(&dataset);
        let bounds = tokio::task::spawn_blocking(move || {
            let guard = dataset_for_inspect.blocking_lock();
            if guard.raster_count() == 0 {
                return Err(TileServerError::RasterError(
                    "DEM source has no raster bands".to_string(),
                ));
            }
            crate::sources::cog::wgs84_bounds(&guard)
        })
        .await
        .map_err(|e| TileServerError::RasterError(format!("task failed: {e}")))??;

        let encoding = config.dem_encoding;
        let params = EncodeParams {
            encoding,
            scale: config.dem_scale.unwrap_or(1.0),
            offset: config.dem_offset.unwrap_or(0.0),
            nodata_value: None,
            nodata_rgba: nodata_rgba(encoding, config.dem_nodata_color),
        };

        let metadata = TileMetadata {
            id: config.id.clone(),
            name: config
                .name
                .clone()
                .unwrap_or_else(|| "DEM Source".to_string()),
            description: None,
            attribution: config.attribution.clone(),
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
            resampling: config.resampling.unwrap_or(ResamplingMethod::Bilinear),
            band: config.dem_band,
            params,
        })
    }

    async fn resolve_dataset(
        config: &SourceConfig,
        loaded: &std::collections::HashMap<String, Arc<dyn TileSource>>,
    ) -> Result<Arc<Mutex<Dataset>>> {
        if let Some(ref input_id) = config.input_source {
            let source = loaded.get(input_id).ok_or_else(|| {
                TileServerError::ConfigError(format!(
                    "DEM source '{}' references unknown input_source '{input_id}'",
                    config.id
                ))
            })?;
            let cog = source.as_any().downcast_ref::<CogSource>().ok_or_else(|| {
                TileServerError::ConfigError(format!(
                    "DEM source '{}' input_source '{input_id}' must be a cog/vrt raster source",
                    config.id
                ))
            })?;
            return Ok(cog.dataset_handle());
        }
        dataset_cache::global().get_or_open(&config.path).await
    }

    /// Render one tile, exposed directly so the manager can request a
    /// non-default tile size without a second async hop.
    pub async fn get_tile_sized(
        &self,
        z: u8,
        x: u32,
        y: u32,
        tile_size: u32,
    ) -> Result<Option<TileData>> {
        let (minx, miny, maxx, maxy) = tile_to_web_mercator_bbox(z, x, y);
        let dataset = self.dataset.clone();
        let band = self.band;
        let resampling: ResampleAlg = self.resampling.into();
        let params = self.params;

        let png = tokio::task::spawn_blocking(move || {
            let guard = dataset.blocking_lock();
            render_dem_tile(
                &guard, minx, miny, maxx, maxy, tile_size, band, resampling, &params,
            )
        })
        .await
        .map_err(|e| TileServerError::RasterError(format!("task failed: {e}")))??;

        Ok(Some(TileData {
            data: Bytes::from(png),
            format: TileFormat::Png,
            compression: TileCompression::None,
        }))
    }
}

#[async_trait]
impl TileSource for DemSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        self.get_tile_sized(z, x, y, 256).await
    }

    fn metadata(&self) -> &TileMetadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn tile_to_web_mercator_bbox(z: u8, x: u32, y: u32) -> (f64, f64, f64, f64) {
    let n = 2_u32.pow(u32::from(z)) as f64;
    let span = 2.0 * WEB_MERCATOR_EXTENT / n;
    let minx = -WEB_MERCATOR_EXTENT + f64::from(x) * span;
    let maxy = WEB_MERCATOR_EXTENT - f64::from(y) * span;
    (minx, maxy - span, minx + span, maxy)
}

#[allow(clippy::too_many_arguments)]
fn render_dem_tile(
    dataset: &Dataset,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
    tile_size: u32,
    band: usize,
    resampling: ResampleAlg,
    params: &EncodeParams,
) -> Result<Vec<u8>> {
    let web_mercator = SpatialRef::from_epsg(3857)
        .map_err(|e| TileServerError::RasterError(format!("failed to create EPSG:3857: {e}")))?;
    let mem = DriverManager::get_driver_by_name("MEM")
        .map_err(|e| TileServerError::RasterError(format!("failed to get MEM driver: {e}")))?;
    let mut warped = mem
        .create_with_band_type::<f64, _>("", tile_size as usize, tile_size as usize, 1)
        .map_err(|e| TileServerError::RasterError(format!("failed to create warp target: {e}")))?;

    let px = (maxx - minx) / f64::from(tile_size);
    let py = (maxy - miny) / f64::from(tile_size);
    warped
        .set_geo_transform(&[minx, px, 0.0, maxy, 0.0, -py])
        .map_err(|e| TileServerError::RasterError(format!("failed to set geotransform: {e}")))?;
    warped
        .set_spatial_ref(&web_mercator)
        .map_err(|e| TileServerError::RasterError(format!("failed to set SRS: {e}")))?;
    reproject(dataset, &warped)
        .map_err(|e| TileServerError::RasterError(format!("failed to reproject: {e}")))?;

    let src_band = dataset
        .rasterband(band)
        .map_err(|e| TileServerError::RasterError(format!("failed to read band {band}: {e}")))?;
    let nodata = src_band.no_data_value();

    let out_band = warped
        .rasterband(1)
        .map_err(|e| TileServerError::RasterError(format!("failed to read warp band: {e}")))?;
    let buffer: Buffer<f64> = out_band
        .read_as::<f64>(
            (0, 0),
            (tile_size as usize, tile_size as usize),
            (tile_size as usize, tile_size as usize),
            Some(resampling),
        )
        .map_err(|e| TileServerError::RasterError(format!("failed to read band: {e}")))?;

    let mut params = *params;
    params.nodata_value = nodata;
    let rgba = encode_pixels(buffer.data(), &params);

    let img: RgbaImage = ImageBuffer::from_raw(tile_size, tile_size, rgba)
        .ok_or_else(|| TileServerError::RasterError("RGBA buffer size mismatch".to_string()))?;
    let mut png = Vec::new();
    img.write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| TileServerError::RasterError(format!("failed to encode PNG: {e}")))?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Terrarium round-trip (precision 1/256 m ≈ 0.0039 m) ----

    #[test]
    fn terrarium_roundtrip_within_precision() {
        // Spread across the full real-terrain range: Mariana Trench floor,
        // sea level, Everest summit, plus a fractional value that exercises
        // the blue (sub-metre) byte.
        for &elev in &[
            0.0_f64, 2523.266, -10994.0, 8848.86, 1000.5, -11000.0, 8900.0,
        ] {
            let rgb = encode_terrarium(elev);
            let decoded = decode_terrarium(rgb);
            assert!(
                (decoded - elev).abs() < 0.005,
                "terrarium roundtrip elev={elev} rgb={rgb:?} decoded={decoded} err={}",
                (decoded - elev).abs()
            );
        }
    }

    #[test]
    fn terrarium_known_vectors() {
        // 0 m → (128, 0, 0): 128*256 + 0 + 0 - 32768 = 0.
        assert_eq!(encode_terrarium(0.0), [128, 0, 0], "terrarium sea level");
        // -32768 m → (0, 0, 0): the nodata sentinel elevation.
        assert_eq!(encode_terrarium(-32768.0), [0, 0, 0], "terrarium min");
    }

    #[test]
    fn terrarium_clamps_out_of_range() {
        // Below the representable floor clamps to (0,0,0); above the ceiling
        // clamps to the max triplet — never wraps a byte.
        assert_eq!(encode_terrarium(-40000.0), [0, 0, 0], "terrarium underflow");
        assert_eq!(
            encode_terrarium(40000.0),
            [255, 255, 255],
            "terrarium overflow"
        );
    }

    // ---- Mapbox-RGB round-trip (precision 0.1 m) ----

    #[test]
    fn mapbox_roundtrip_within_precision() {
        for &elev in &[0.0_f64, 407.2, -10000.0, 8848.86, 1000.5, 6553.6, 1234.5] {
            let rgb = encode_mapbox(elev);
            let decoded = decode_mapbox(rgb);
            assert!(
                (decoded - elev).abs() < 0.05,
                "mapbox roundtrip elev={elev} rgb={rgb:?} decoded={decoded} err={}",
                (decoded - elev).abs()
            );
        }
    }

    #[test]
    fn mapbox_known_vectors() {
        // -10000 m → (0,0,0): the base of the Mapbox range.
        assert_eq!(encode_mapbox(-10000.0), [0, 0, 0], "mapbox min");
        // 0 m → packed = 100000 = 0x0186A0 → (1, 134, 160).
        assert_eq!(encode_mapbox(0.0), [1, 134, 160], "mapbox sea level");
    }

    #[test]
    fn mapbox_clamps_out_of_range() {
        // Below base clamps to (0,0,0); far above the 24-bit ceiling clamps
        // to (255,255,255), never wraps.
        assert_eq!(encode_mapbox(-20000.0), [0, 0, 0], "mapbox underflow");
        let top = decode_mapbox([255, 255, 255]);
        assert_eq!(
            encode_mapbox(top + 1000.0),
            [255, 255, 255],
            "mapbox overflow"
        );
    }

    #[test]
    fn mapbox_round_before_truncate() {
        // The classic rgbify bug: 1000.5 m packs to exactly 110005.0; a
        // truncating encoder that computed 110004.999.. would lose the LSB.
        // Round-then-split must land on the exact triplet.
        let rgb = encode_mapbox(1000.5);
        let decoded = decode_mapbox(rgb);
        assert!(
            (decoded - 1000.5).abs() < 0.001,
            "mapbox round-before-truncate 1000.5 → {rgb:?} → {decoded}"
        );
    }

    // ---- Dispatcher + nodata ----

    #[test]
    fn encode_dispatch_matches_concrete() {
        assert_eq!(
            encode_elevation(1234.5, DemEncoding::Terrarium),
            encode_terrarium(1234.5)
        );
        assert_eq!(
            encode_elevation(1234.5, DemEncoding::MapboxRgb),
            encode_mapbox(1234.5)
        );
    }

    #[test]
    fn decode_dispatch_matches_concrete() {
        assert_eq!(
            decode_elevation([1, 134, 160], DemEncoding::MapboxRgb),
            decode_mapbox([1, 134, 160])
        );
        assert_eq!(
            decode_elevation([128, 0, 0], DemEncoding::Terrarium),
            decode_terrarium([128, 0, 0])
        );
    }

    #[test]
    fn nodata_defaults_per_encoding() {
        // Terrarium default sentinel decodes to -32768; alpha transparent.
        assert_eq!(nodata_rgba(DemEncoding::Terrarium, None), [0, 0, 0, 0]);
        // Mapbox default sentinel is sea-level (1,134,160); alpha transparent.
        assert_eq!(nodata_rgba(DemEncoding::MapboxRgb, None), [1, 134, 160, 0]);
    }

    #[test]
    fn nodata_override_wins() {
        assert_eq!(
            nodata_rgba(DemEncoding::Terrarium, Some([10, 20, 30, 255])),
            [10, 20, 30, 255]
        );
    }

    // ---- encode_pixels (the GDAL-free pixel loop) ----

    fn params(encoding: DemEncoding, nodata: Option<f64>) -> EncodeParams {
        EncodeParams {
            encoding,
            scale: 1.0,
            offset: 0.0,
            nodata_value: nodata,
            nodata_rgba: nodata_rgba(encoding, None),
        }
    }

    #[test]
    fn encode_pixels_encodes_each_value() {
        let values = [0.0_f64, 1000.5, 8848.86];
        let out = encode_pixels(&values, &params(DemEncoding::MapboxRgb, None));
        assert_eq!(out.len(), values.len() * 4, "4 RGBA bytes per pixel");
        // Pixel 0 (0 m) → Mapbox sentinel-free sea level (1,134,160,255 opaque).
        assert_eq!(&out[0..4], &[1, 134, 160, 255]);
        // Round-trip the middle pixel to confirm real encoding, not nodata.
        let mid = [out[4], out[5], out[6]];
        assert!((decode_mapbox(mid) - 1000.5).abs() < 0.05);
        assert_eq!(out[7], 255, "valid pixel is opaque");
    }

    #[test]
    fn encode_pixels_writes_sentinel_for_nodata_value() {
        // Pixel exactly equal to the band's nodata value → sentinel (alpha 0).
        let out = encode_pixels(
            &[-9999.0, 100.0],
            &params(DemEncoding::Terrarium, Some(-9999.0)),
        );
        assert_eq!(
            &out[0..4],
            &[0, 0, 0, 0],
            "nodata pixel → transparent sentinel"
        );
        assert_eq!(out[7], 255, "valid pixel opaque");
    }

    #[test]
    fn encode_pixels_writes_sentinel_for_non_finite() {
        // NaN / inf from a masked GDAL read are nodata regardless of config.
        let out = encode_pixels(
            &[f64::NAN, f64::INFINITY, 50.0],
            &params(DemEncoding::MapboxRgb, None),
        );
        assert_eq!(&out[0..4], &[1, 134, 160, 0], "NaN → sentinel");
        assert_eq!(&out[4..8], &[1, 134, 160, 0], "inf → sentinel");
        assert_eq!(out[11], 255, "finite pixel opaque");
    }

    #[test]
    fn encode_pixels_applies_scale_and_offset() {
        // Source stored in feet, consumer wants metres: scale 0.3048.
        let p = EncodeParams {
            encoding: DemEncoding::MapboxRgb,
            scale: 0.3048,
            offset: 0.0,
            nodata_value: None,
            nodata_rgba: nodata_rgba(DemEncoding::MapboxRgb, None),
        };
        let out = encode_pixels(&[1000.0], &p);
        let decoded = decode_mapbox([out[0], out[1], out[2]]);
        assert!(
            (decoded - 304.8).abs() < 0.05,
            "1000 ft → 304.8 m, got {decoded}"
        );
    }

    #[test]
    fn encode_pixels_empty_input() {
        assert!(encode_pixels(&[], &params(DemEncoding::Terrarium, None)).is_empty());
    }

    #[test]
    fn tile_bbox_z0_is_full_extent() {
        let (minx, miny, maxx, maxy) = tile_to_web_mercator_bbox(0, 0, 0);
        assert!((minx + WEB_MERCATOR_EXTENT).abs() < 1e-6);
        assert!((miny + WEB_MERCATOR_EXTENT).abs() < 1e-6);
        assert!((maxx - WEB_MERCATOR_EXTENT).abs() < 1e-6);
        assert!((maxy - WEB_MERCATOR_EXTENT).abs() < 1e-6);
    }
}
