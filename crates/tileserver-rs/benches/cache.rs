use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tileserver_rs::cache::{TileCache, TileCacheKey};
use tileserver_rs::sources::{TileCompression, TileData, TileFormat};

fn make_tile(size: usize) -> TileData {
    TileData {
        data: Bytes::from(vec![42u8; size]),
        format: TileFormat::Pbf,
        compression: TileCompression::None,
    }
}

fn bench_cache(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("tile_cache");

    group.bench_function("cache_hit", |b| {
        let cache = TileCache::new(64, 3600);
        let key = TileCacheKey {
            source_id: "bench".into(),
            z: 14,
            x: 8192,
            y: 5461,
            encoding: TileCompression::None,
        };
        let tile = make_tile(4096);
        rt.block_on(cache.insert(key.clone(), tile));
        b.iter(|| rt.block_on(black_box(cache.get(&key))));
    });

    group.bench_function("cache_miss", |b| {
        let cache = TileCache::new(64, 3600);
        let key = TileCacheKey {
            source_id: "bench".into(),
            z: 14,
            x: 9999,
            y: 9999,
            encoding: TileCompression::None,
        };
        b.iter(|| rt.block_on(black_box(cache.get(&key))));
    });

    for &size in &[1_024usize, 10_240, 102_400] {
        group.bench_with_input(BenchmarkId::new("cache_insert", size), &size, |b, &sz| {
            let cache = TileCache::new(512, 3600);
            let tile = make_tile(sz);
            let mut counter: u32 = 0;
            b.iter(|| {
                let key = TileCacheKey {
                    source_id: "bench".into(),
                    z: 14,
                    x: counter,
                    y: counter,
                    encoding: TileCompression::None,
                };
                counter = counter.wrapping_add(1);
                rt.block_on(cache.insert(black_box(key), black_box(tile.clone())));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_cache);
criterion_main!(benches);
