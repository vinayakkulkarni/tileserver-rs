//! Criterion benchmark for per-feature CEL filtering (#1010).
//!
//! Target from the issue: filtering a 5000-feature tile completes in < 50 ms
//! with a warm compiled-expression cache.

use bytes::Bytes;
use criterion::{Criterion, criterion_group, criterion_main};
use geozero::mvt::{Message, Tile, tile};
use std::hint::black_box;
use tileserver_rs::cel_filter::{CelFilter, apply};
use tileserver_rs::sources::{TileCompression, TileData, TileFormat};

fn tile_with_features(count: usize) -> TileData {
    let kinds = ["residential", "commercial", "industrial", "park"];
    let mut values = Vec::with_capacity(count * 2);
    let mut features = Vec::with_capacity(count);
    for i in 0..count {
        values.push(tile::Value {
            string_value: Some(kinds[i % kinds.len()].to_string()),
            ..Default::default()
        });
        values.push(tile::Value {
            int_value: Some(i64::try_from(i % 200).expect("height")),
            ..Default::default()
        });
        let base = u32::try_from(i * 2).expect("index");
        features.push(tile::Feature {
            id: Some(i as u64),
            tags: vec![0, base, 1, base + 1],
            r#type: Some(1),
            geometry: vec![9, 50, 34],
        });
    }
    let layer = tile::Layer {
        version: 2,
        name: "landuse".to_string(),
        features,
        keys: vec!["type".to_string(), "height".to_string()],
        values,
        extent: Some(4096),
    };
    TileData {
        data: Bytes::from(
            Tile {
                layers: vec![layer],
            }
            .encode_to_vec(),
        ),
        format: TileFormat::Pbf,
        compression: TileCompression::None,
    }
}

fn bench_cel_filter(c: &mut Criterion) {
    let tile = tile_with_features(5000);
    let filter =
        CelFilter::compile("type == 'residential' && height > 50").expect("filter compiles");
    c.bench_function("cel_filter_5000_features", |b| {
        b.iter(|| apply(black_box(tile.clone()), &[black_box(&filter)]).expect("apply"));
    });
}

criterion_group!(benches, bench_cel_filter);
criterion_main!(benches);
