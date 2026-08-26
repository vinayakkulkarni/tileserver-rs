//! Encode [`RasterImage`] to PNG/JPEG/WebP bytes for the HTTP boundary.
//!
//! This is the *only* place in the raster pipeline where f32 pixel data
//! is compressed to a codec. Everything upstream (mosaic, band math,
//! colormap, masks) operates on [`RasterImage`]; this module produces the
//! `Vec<u8>` that ships over HTTP.

use std::io::Cursor;

use image::{ImageFormat, codecs::jpeg::JpegEncoder, codecs::webp::WebPEncoder};

use super::RasterImage;
use super::convert::rgba_from_raster;

#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    #[error("image encoding failed: {0}")]
    Image(#[from] image::ImageError),
    #[error("raster conversion failed: {0}")]
    Convert(String),
    #[error("i/o failure during encode: {0}")]
    Io(#[from] std::io::Error),
}

/// Encode the raster as a PNG byte buffer.
///
/// # Errors
/// Returns [`EncodeError`] on internal conversion or codec failure.
pub fn to_png(img: &RasterImage) -> Result<Vec<u8>, EncodeError> {
    let rgba = rgba_from_raster(img).map_err(EncodeError::Convert)?;
    let mut buf = Vec::with_capacity(rgba.as_raw().len());
    rgba.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)?;
    Ok(buf)
}

/// Encode the raster as a JPEG byte buffer at the given quality
/// (1-100). JPEG lacks an alpha channel, so masked pixels render as
/// opaque black.
///
/// # Errors
/// Returns [`EncodeError`] on internal conversion or codec failure.
pub fn to_jpeg(img: &RasterImage, quality: u8) -> Result<Vec<u8>, EncodeError> {
    let rgba = rgba_from_raster(img).map_err(EncodeError::Convert)?;
    let (w, h) = rgba.dimensions();
    let rgb: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|px| [px[0], px[1], px[2]])
        .collect();
    let mut buf = Vec::with_capacity(rgb.len() / 4);
    JpegEncoder::new_with_quality(&mut Cursor::new(&mut buf), quality).encode(
        &rgb,
        w,
        h,
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(buf)
}

/// Encode the raster as a WebP byte buffer (lossless, preserves alpha).
///
/// # Errors
/// Returns [`EncodeError`] on internal conversion or codec failure.
pub fn to_webp(img: &RasterImage) -> Result<Vec<u8>, EncodeError> {
    let rgba = rgba_from_raster(img).map_err(EncodeError::Convert)?;
    let (w, h) = rgba.dimensions();
    let mut buf = Vec::new();
    WebPEncoder::new_lossless(&mut buf).encode(
        rgba.as_raw(),
        w,
        h,
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;
    use ndarray::array;

    #[test]
    fn encode_png_produces_valid_image() {
        let data = array![[[10.0_f32, 20.0], [30.0, 40.0]]];
        let raster = RasterImage::from_opaque(data, None);
        let png = to_png(&raster).expect("encode png");
        assert!(!png.is_empty());
        let decoded = image::load_from_memory_with_format(&png, ImageFormat::Png).unwrap();
        assert_eq!(decoded.dimensions(), (2, 2));
    }

    #[test]
    fn encode_jpeg_respects_quality() {
        let data = array![[
            [100.0_f32, 110.0, 120.0],
            [130.0, 140.0, 150.0],
            [160.0, 170.0, 180.0]
        ]];
        let raster = RasterImage::from_opaque(data, None);
        let lo = to_jpeg(&raster, 20).unwrap();
        let hi = to_jpeg(&raster, 95).unwrap();
        assert!(
            lo.len() < hi.len(),
            "higher quality must produce larger file ({} vs {})",
            lo.len(),
            hi.len()
        );
    }

    #[test]
    fn encode_webp_is_valid() {
        let data = array![[[50.0_f32, 60.0], [70.0, 80.0]]];
        let raster = RasterImage::from_opaque(data, None);
        let webp = to_webp(&raster).unwrap();
        assert!(!webp.is_empty());
        let decoded = image::load_from_memory_with_format(&webp, ImageFormat::WebP).unwrap();
        assert_eq!(decoded.dimensions(), (2, 2));
    }

    #[test]
    fn encode_error_display_covers_all_variants() {
        let convert_err = EncodeError::Convert("bad band count".into());
        assert!(convert_err.to_string().contains("bad band count"));
        assert!(convert_err.to_string().contains("raster conversion"));

        let io_err = EncodeError::Io(std::io::Error::other("disk full"));
        assert!(io_err.to_string().contains("disk full"));
        assert!(io_err.to_string().contains("i/o failure"));

        let img_err = EncodeError::Image(image::ImageError::Limits(
            image::error::LimitError::from_kind(image::error::LimitErrorKind::DimensionError),
        ));
        assert!(img_err.to_string().contains("image encoding failed"));
    }

    #[test]
    fn encode_single_band_grayscale_works() {
        let data = array![[[42.0_f32, 42.0], [42.0, 42.0]]];
        let raster = RasterImage::from_opaque(data, None);
        let png = to_png(&raster).expect("single band should encode");
        assert!(!png.is_empty());
    }

    #[test]
    fn encode_jpeg_with_minimum_quality() {
        let data = array![[[100.0_f32, 110.0], [120.0, 130.0]]];
        let raster = RasterImage::from_opaque(data, None);
        let jpeg = to_jpeg(&raster, 1).expect("jpeg q=1 should encode");
        assert!(!jpeg.is_empty());
    }

    #[test]
    fn encode_webp_round_trip_preserves_dimensions() {
        let data = array![[
            [10.0_f32, 20.0, 30.0],
            [40.0, 50.0, 60.0],
            [70.0, 80.0, 90.0]
        ]];
        let raster = RasterImage::from_opaque(data, None);
        let webp = to_webp(&raster).expect("webp encode");
        let decoded = image::load_from_memory_with_format(&webp, ImageFormat::WebP).unwrap();
        assert_eq!(decoded.dimensions(), (3, 3));
    }
}
