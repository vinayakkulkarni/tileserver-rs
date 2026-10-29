//! GDAL warp with an explicit resampling algorithm.
//!
//! `gdal::raster::reproject` (gdal 0.19) always passes
//! `GDALResampleAlg::GRA_Bilinear` to `GDALReprojectImage`, so the
//! `resampling` setting of every raster source had no effect on the warp
//! (issue #1599). For categorical rasters this created class values that do
//! not exist in the source. [`reproject`] makes the same call with the
//! algorithm the caller asks for.

use std::ffi::CStr;
use std::ptr::{null, null_mut};

use gdal::Dataset;
use gdal_sys::{CPLErr, GDALResampleAlg};

use crate::config::ResamplingMethod;
use crate::error::{Result, TileServerError};

/// Map a [`ResamplingMethod`] to the GDAL warp algorithm with the same name.
#[must_use]
pub fn warp_algorithm(method: ResamplingMethod) -> GDALResampleAlg::Type {
    match method {
        ResamplingMethod::Nearest => GDALResampleAlg::GRA_NearestNeighbour,
        ResamplingMethod::Bilinear => GDALResampleAlg::GRA_Bilinear,
        ResamplingMethod::Cubic => GDALResampleAlg::GRA_Cubic,
        ResamplingMethod::CubicSpline => GDALResampleAlg::GRA_CubicSpline,
        ResamplingMethod::Lanczos => GDALResampleAlg::GRA_Lanczos,
        ResamplingMethod::Average => GDALResampleAlg::GRA_Average,
        ResamplingMethod::Mode => GDALResampleAlg::GRA_Mode,
    }
}

/// Warp `src` into `dst` (which must already have its geotransform and SRS
/// set) with the given resampling method.
///
/// # Errors
///
/// Returns [`TileServerError::RasterError`] with GDAL's last error message
/// when `GDALReprojectImage` fails, e.g. when the tile is outside the
/// source footprint and no transform can be built.
pub fn reproject(src: &Dataset, dst: &Dataset, method: ResamplingMethod) -> Result<()> {
    // SAFETY: `src` and `dst` are open datasets borrowed for the whole call,
    // so both handles stay valid until it returns; GDAL keeps no reference to
    // them afterwards. Null WKT means "use the dataset's own SRS", the null
    // progress callback and null warp options are documented as allowed.
    let rv = unsafe {
        gdal_sys::GDALReprojectImage(
            src.c_dataset(),
            null(),
            dst.c_dataset(),
            null(),
            warp_algorithm(method),
            0.0,
            0.0,
            None,
            null_mut(),
            null_mut(),
        )
    };
    if rv == CPLErr::CE_None {
        return Ok(());
    }
    // SAFETY: `CPLGetLastErrorMsg` returns a non-null pointer to a
    // NUL-terminated, thread-local buffer owned by GDAL. It stays valid until
    // the next GDAL call on this thread, and it is copied here at once.
    let msg = unsafe { CStr::from_ptr(gdal_sys::CPLGetLastErrorMsg()) }
        .to_string_lossy()
        .into_owned();
    Err(TileServerError::RasterError(format!(
        "GDALReprojectImage ({method}) failed: {msg}"
    )))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use gdal::DriverManager;
    use gdal::raster::Buffer;
    use gdal::spatial_ref::SpatialRef;
    use std::collections::BTreeSet;

    /// 4×4 EPSG:3857 raster, 100 m pixels: left half class 1, right half
    /// class 5. Only the values 1 and 5 exist in the source.
    fn two_class_source() -> Dataset {
        let mem = DriverManager::get_driver_by_name("MEM").unwrap();
        let mut ds = mem.create_with_band_type::<u8, _>("", 4, 4, 1).unwrap();
        ds.set_geo_transform(&[0.0, 100.0, 0.0, 400.0, 0.0, -100.0])
            .unwrap();
        ds.set_spatial_ref(&SpatialRef::from_epsg(3857).unwrap())
            .unwrap();
        let data: Vec<u8> = (0..16).map(|i| if i % 4 < 2 { 1 } else { 5 }).collect();
        let mut buf = Buffer::new((4, 4), data);
        ds.rasterband(1)
            .unwrap()
            .write((0, 0), (4, 4), &mut buf)
            .unwrap();
        ds
    }

    /// Warp the two-class source to a 16×16 grid over the same extent and
    /// return the distinct output values.
    fn warped_values(method: ResamplingMethod) -> BTreeSet<u64> {
        let src = two_class_source();
        let mem = DriverManager::get_driver_by_name("MEM").unwrap();
        let mut dst = mem.create_with_band_type::<f64, _>("", 16, 16, 1).unwrap();
        dst.set_geo_transform(&[0.0, 25.0, 0.0, 400.0, 0.0, -25.0])
            .unwrap();
        dst.set_spatial_ref(&SpatialRef::from_epsg(3857).unwrap())
            .unwrap();
        reproject(&src, &dst, method).unwrap();
        let out: Buffer<f64> = dst
            .rasterband(1)
            .unwrap()
            .read_as::<f64>((0, 0), (16, 16), (16, 16), None)
            .unwrap();
        // Values are compared as scaled integers so a set can hold them.
        out.data()
            .iter()
            .map(|v| (v * 1000.0).round() as u64)
            .collect()
    }

    #[test]
    fn nearest_keeps_only_source_classes() {
        let values = warped_values(ResamplingMethod::Nearest);
        assert_eq!(values, BTreeSet::from([1000, 5000]));
    }

    #[test]
    fn mode_keeps_only_source_classes() {
        let values = warped_values(ResamplingMethod::Mode);
        assert_eq!(values, BTreeSet::from([1000, 5000]));
    }

    #[test]
    fn bilinear_interpolates_between_classes() {
        let values = warped_values(ResamplingMethod::Bilinear);
        assert!(values.contains(&1000) && values.contains(&5000));
        assert!(
            values.iter().any(|v| *v > 1000 && *v < 5000),
            "bilinear must produce values between the two classes, got {values:?}"
        );
    }

    /// A raster whose geotransform is all zeros: GDAL cannot invert it, so
    /// no pixel/map transform exists and the warp must fail.
    pub(crate) fn untransformable_source() -> Dataset {
        let mem = DriverManager::get_driver_by_name("MEM").unwrap();
        let mut ds = mem.create_with_band_type::<u8, _>("", 4, 4, 1).unwrap();
        ds.set_geo_transform(&[0.0; 6]).unwrap();
        ds.set_spatial_ref(&SpatialRef::from_epsg(3857).unwrap())
            .unwrap();
        ds
    }

    #[test]
    fn failed_warp_returns_raster_error_with_gdal_message() {
        let src = untransformable_source();
        let mem = DriverManager::get_driver_by_name("MEM").unwrap();
        let mut dst = mem.create_with_band_type::<f64, _>("", 16, 16, 1).unwrap();
        dst.set_geo_transform(&[0.0, 25.0, 0.0, 400.0, 0.0, -25.0])
            .unwrap();
        dst.set_spatial_ref(&SpatialRef::from_epsg(3857).unwrap())
            .unwrap();

        let prefix = "GDALReprojectImage (nearest) failed: ";
        let err = reproject(&src, &dst, ResamplingMethod::Nearest).unwrap_err();
        assert!(
            matches!(&err, TileServerError::RasterError(m)
                if m.starts_with(prefix) && m.len() > prefix.len()),
            "names the call and the method and includes GDAL's error text: {err:?}"
        );
    }

    #[test]
    fn each_method_maps_to_its_own_algorithm() {
        let methods = [
            ResamplingMethod::Nearest,
            ResamplingMethod::Bilinear,
            ResamplingMethod::Cubic,
            ResamplingMethod::CubicSpline,
            ResamplingMethod::Lanczos,
            ResamplingMethod::Average,
            ResamplingMethod::Mode,
        ];
        let algs: BTreeSet<_> = methods.iter().map(|m| warp_algorithm(*m)).collect();
        assert_eq!(algs.len(), methods.len());
        assert_eq!(
            warp_algorithm(ResamplingMethod::Nearest),
            GDALResampleAlg::GRA_NearestNeighbour
        );
    }
}
