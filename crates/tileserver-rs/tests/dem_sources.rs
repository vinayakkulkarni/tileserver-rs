//! Integration tests for the DEM (Digital Elevation Model) tile source.
//!
//! These exercise the full GDAL pipeline against the shipped Float32 DEM
//! fixture (`data/raster/test-dem.cog.tif`, EPSG:4326, elevations −41..1041 m,
//! nodata −9999): config parse, source loading, tile encoding, the accuracy
//! round-trip (decode an encoded tile back to plausible elevations), the
//! `input_source` composition path, and error handling.

#![cfg(feature = "dem")]

use std::path::PathBuf;

use tileserver_rs::config::{Config, DemEncoding};
use tileserver_rs::sources::SourceManager;
use tileserver_rs::sources::dem::{decode_elevation, decode_mapbox, encode_mapbox};

const DEM_TEST_CONFIG: &str = "tests/config.dem.toml";
// A z11 tile that overlaps the SF-Bay fixture (lon −122.5..−122.3, lat 37.7..37.9).
const COVER_Z: u8 = 11;
const COVER_X: u32 = 327;
const COVER_Y: u32 = 791;

fn png_dimensions(bytes: &[u8]) -> (u32, u32) {
    assert_eq!(&bytes[0..8], b"\x89PNG\r\n\x1a\n", "not a PNG");
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    (w, h)
}

mod config_parsing {
    use super::*;

    #[test]
    fn dem_config_parses() {
        let config = Config::load(Some(PathBuf::from(DEM_TEST_CONFIG))).expect("load dem config");
        let terr = config
            .sources
            .iter()
            .find(|s| s.id == "dem-terrarium")
            .expect("dem-terrarium present");
        assert_eq!(terr.dem_encoding, DemEncoding::Terrarium);

        let mapbox = config
            .sources
            .iter()
            .find(|s| s.id == "dem-mapbox")
            .expect("dem-mapbox present");
        assert_eq!(mapbox.dem_encoding, DemEncoding::MapboxRgb);

        let composed = config
            .sources
            .iter()
            .find(|s| s.id == "dem-from-input")
            .expect("dem-from-input present");
        assert_eq!(composed.input_source.as_deref(), Some("elevation-cog"));
    }
}

mod source_loading {
    use super::*;

    #[tokio::test]
    async fn loads_all_dem_sources() {
        let config = Config::load(Some(PathBuf::from(DEM_TEST_CONFIG))).expect("load config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("load sources");
        for id in [
            "dem-terrarium",
            "dem-mapbox",
            "elevation-cog",
            "dem-from-input",
        ] {
            assert!(sources.get(id).is_some(), "source {id} should load");
        }
    }

    #[tokio::test]
    async fn dem_metadata_is_png() {
        let config = Config::load(Some(PathBuf::from(DEM_TEST_CONFIG))).expect("load config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("load sources");
        let meta = sources
            .get("dem-terrarium")
            .expect("source")
            .metadata()
            .clone();
        assert_eq!(meta.format, tileserver_rs::sources::TileFormat::Png);
        assert!(meta.bounds.is_some(), "DEM source has WGS84 bounds");
    }
}

mod tile_encoding {
    use super::*;

    #[tokio::test]
    async fn terrarium_tile_is_valid_png() {
        let config = Config::load(Some(PathBuf::from(DEM_TEST_CONFIG))).expect("load config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("load sources");
        let tile = sources
            .get("dem-terrarium")
            .expect("source")
            .get_tile(COVER_Z, COVER_X, COVER_Y)
            .await
            .expect("get_tile ok")
            .expect("tile present");
        assert_eq!(tile.format, tileserver_rs::sources::TileFormat::Png);
        assert_eq!(png_dimensions(&tile.data), (256, 256), "256x256 tile");
    }

    #[tokio::test]
    async fn out_of_bounds_tile_renders_nodata_not_error() {
        // A tile far from the fixture must not error; GDAL warps an empty
        // window and every pixel is nodata, still a valid PNG.
        let config = Config::load(Some(PathBuf::from(DEM_TEST_CONFIG))).expect("load config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("load sources");
        let tile = sources
            .get("dem-terrarium")
            .expect("source")
            .get_tile(11, 0, 0)
            .await
            .expect("get_tile ok");
        assert!(tile.is_some(), "off-fixture tile still yields a PNG");
    }
}

mod accuracy_proof {
    use super::*;
    use image::GenericImageView;

    /// Decode a rendered DEM tile back to elevations and assert they land in
    /// the fixture's real range (−41..1041 m, with nodata at the sentinel).
    /// This is the correctness proof: the encoder is lossless to within the
    /// encoding interval, so a round-trip of real terrain must reproduce
    /// plausible elevations — not garbage.
    async fn decode_tile_elevations(source_id: &str, encoding: DemEncoding) -> Vec<f64> {
        let config = Config::load(Some(PathBuf::from(DEM_TEST_CONFIG))).expect("load config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("load sources");
        let tile = sources
            .get(source_id)
            .expect("source")
            .get_tile(COVER_Z, COVER_X, COVER_Y)
            .await
            .expect("get_tile ok")
            .expect("tile present");
        let img = image::load_from_memory(&tile.data).expect("decode png");
        let mut elevations = Vec::new();
        for (_, _, px) in img.pixels() {
            let rgb = [px.0[0], px.0[1], px.0[2]];
            elevations.push(decode_elevation(rgb, encoding));
        }
        elevations
    }

    #[tokio::test]
    async fn terrarium_roundtrip_recovers_plausible_elevations() {
        let elevations = decode_tile_elevations("dem-terrarium", DemEncoding::Terrarium).await;
        // Drop nodata sentinel (−32768 m) and assert the rest are real terrain.
        let real: Vec<f64> = elevations.into_iter().filter(|&e| e > -30000.0).collect();
        assert!(!real.is_empty(), "tile contains some real elevation pixels");
        let min = real.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = real.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        // Fixture min/max is −41..1041 m; allow a small resampling margin.
        assert!(min > -100.0, "decoded min {min} within plausible terrain");
        assert!(max < 1200.0, "decoded max {max} within plausible terrain");
    }

    #[tokio::test]
    async fn mapbox_roundtrip_recovers_plausible_elevations() {
        let elevations = decode_tile_elevations("dem-mapbox", DemEncoding::MapboxRgb).await;
        let real: Vec<f64> = elevations.into_iter().filter(|&e| e > -5000.0).collect();
        assert!(!real.is_empty(), "tile contains real elevations");
        let max = real.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(max < 1200.0 && max > 50.0, "decoded max {max} plausible");
    }

    #[test]
    fn encoder_is_deterministic_against_spec() {
        // Lock the encoder to the published Mapbox spec: 0 m → (1,134,160),
        // round-tripping below 0.05 m. This is the determinism half of the
        // accuracy proof (no GDAL needed).
        let rgb = encode_mapbox(0.0);
        assert_eq!(rgb, [1, 134, 160]);
        assert!((decode_mapbox(rgb) - 0.0).abs() < 0.05);
    }
}

mod input_source_composition {
    use super::*;

    #[tokio::test]
    async fn dem_reads_through_input_source() {
        // dem-from-input has no path of its own — it must read the COG it
        // references by id. A valid tile proves the composition resolved.
        let config = Config::load(Some(PathBuf::from(DEM_TEST_CONFIG))).expect("load config");
        let sources = SourceManager::from_configs(&config.sources)
            .await
            .expect("load sources");
        let tile = sources
            .get("dem-from-input")
            .expect("composed source loaded")
            .get_tile(COVER_Z, COVER_X, COVER_Y)
            .await
            .expect("get_tile ok")
            .expect("tile present");
        assert_eq!(png_dimensions(&tile.data), (256, 256));
    }
}
