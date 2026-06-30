//! Benchmarks for DEM (Digital Elevation Model) encoder hot paths.
//!
//! Run with: `cargo bench --bench dem --features dem`
//!
//! Covers the two pure encoders (Terrarium / Mapbox-RGB) and a full
//! tile-sized pixel encode, which is the per-tile cost on the serving
//! hot path once GDAL has produced the float elevation grid.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use tileserver_rs::config::DemEncoding;
use tileserver_rs::sources::dem::{EncodeParams, encode_mapbox, encode_pixels, encode_terrarium};

const ELEVATIONS: &[f64] = &[0.0, 100.5, 1000.5, 2523.266, 8848.86, -41.386, 407.2];

fn bench_encode_terrarium(c: &mut Criterion) {
    let mut group = c.benchmark_group("dem_encode_terrarium");
    group.bench_function("single", |b| {
        b.iter(|| {
            for &e in ELEVATIONS {
                black_box(encode_terrarium(black_box(e)));
            }
        });
    });
    group.finish();
}

fn bench_encode_mapbox(c: &mut Criterion) {
    let mut group = c.benchmark_group("dem_encode_mapbox");
    group.bench_function("single", |b| {
        b.iter(|| {
            for &e in ELEVATIONS {
                black_box(encode_mapbox(black_box(e)));
            }
        });
    });
    group.finish();
}

fn bench_encode_tile(c: &mut Criterion) {
    // 256x256 float grid sweeping the real-terrain range, ~10% nodata —
    // representative of the per-tile encode cost after the GDAL read.
    let pixels = 256 * 256;
    let grid: Vec<f64> = (0..pixels)
        .map(|i| {
            if i % 10 == 0 {
                f64::NAN
            } else {
                (i as f64 % 4000.0) - 41.0
            }
        })
        .collect();

    let mut group = c.benchmark_group("dem_encode_tile_256");
    group.throughput(Throughput::Elements(pixels as u64));
    for encoding in [DemEncoding::Terrarium, DemEncoding::MapboxRgb] {
        let params = EncodeParams {
            encoding,
            scale: 1.0,
            offset: 0.0,
            nodata_value: None,
            nodata_rgba: [0, 0, 0, 0],
        };
        let label = match encoding {
            DemEncoding::Terrarium => "terrarium",
            DemEncoding::MapboxRgb => "mapbox_rgb",
        };
        group.bench_function(label, |b| {
            b.iter(|| black_box(encode_pixels(black_box(&grid), black_box(&params))));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_encode_terrarium,
    bench_encode_mapbox,
    bench_encode_tile
);
criterion_main!(benches);
