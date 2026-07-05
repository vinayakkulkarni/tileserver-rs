//! Benchmarks for convert-pipeline hot paths.
//!
//! Run with: `cargo bench --bench convert --no-default-features --features convert`
//!
//! Covers Douglas-Peucker simplification (the per-feature per-zoom cost),
//! Web-Mercator projection helpers, and the full TileBuilder
//! partition + MVT-encode path, which dominates conversion wall time.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::collections::BTreeMap;
use std::hint::black_box;
use tileserver_rs::convert::input::{ConvertFeature, Geometry, PropValue};
use tileserver_rs::convert::simplify::dp_simplify;
use tileserver_rs::convert::tile_builder::{
    TileBuilder, TileOptions, lat_to_mercator_y, lon_to_mercator_x,
};

/// A synthetic GPS-track-like line: 1000 vertices with small jitter.
fn synthetic_line(n: usize) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let jitter = ((i * 2_654_435_761) % 1000) as f64 / 1e6;
            (8.5 + t * 0.1 + jitter, 47.3 + t * 0.05 - jitter)
        })
        .collect()
}

fn bench_dp_simplify(c: &mut Criterion) {
    let line = synthetic_line(1000);
    let mut group = c.benchmark_group("convert_dp_simplify");
    group.throughput(Throughput::Elements(line.len() as u64));
    for tolerance in [1e-6, 1e-4, 1e-2] {
        group.bench_function(format!("n1000_tol{tolerance:e}"), |b| {
            b.iter(|| black_box(dp_simplify(black_box(&line), black_box(tolerance))));
        });
    }
    group.finish();
}

fn bench_mercator_projection(c: &mut Criterion) {
    let coords = synthetic_line(1000);
    let mut group = c.benchmark_group("convert_mercator");
    group.throughput(Throughput::Elements(coords.len() as u64));
    group.bench_function("project_1000", |b| {
        b.iter(|| {
            for &(lon, lat) in &coords {
                black_box(lon_to_mercator_x(black_box(lon)));
                black_box(lat_to_mercator_y(black_box(lat)));
            }
        });
    });
    group.finish();
}

/// Build a representative feature set: points spread over a city extent.
fn synthetic_features(n: usize) -> Vec<ConvertFeature> {
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let mut properties = BTreeMap::new();
            properties.insert("name".to_owned(), PropValue::String(format!("poi-{i}")));
            properties.insert("rank".to_owned(), PropValue::Float(t * 100.0));
            ConvertFeature {
                geometry: Geometry::Point((8.5 + t * 0.06, 47.35 + (t * 7.0).fract() * 0.04)),
                properties,
                id: Some(i as u64),
            }
        })
        .collect()
}

fn bench_tile_builder_full(c: &mut Criterion) {
    let features = synthetic_features(1000);
    let mut group = c.benchmark_group("convert_tile_builder");
    group.sample_size(20);
    group.throughput(Throughput::Elements(features.len() as u64));
    group.bench_function("points1000_z0_14", |b| {
        b.iter(|| {
            let mut builder = TileBuilder::new(TileOptions {
                min_zoom: 0,
                max_zoom: 14,
                layer_name: "bench".to_owned(),
                simplification: None,
                include_properties: Vec::new(),
                exclude_properties: Vec::new(),
                drop_densest: false,
            });
            for feature in &features {
                builder.add_feature(black_box(feature.clone()));
            }
            black_box(builder.finish().expect("encode"))
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_dp_simplify,
    bench_mercator_projection,
    bench_tile_builder_full
);
criterion_main!(benches);
