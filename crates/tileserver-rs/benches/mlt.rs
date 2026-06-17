//! MLT (MapLibre Tiles) benchmarks using real OpenMapTiles fixtures.
//!
//! Run with: `cargo bench --features mlt --bench mlt`
//!
//! ## Benchmark Groups
//!
//! These benchmarks use **real OpenMapTiles tile fixtures** from the
//! [maplibre-tile-spec](https://github.com/maplibre/maplibre-tile-spec) repository —
//! the same fixtures used by `mlt-core`'s own Criterion benchmarks.
//!
//! | Group | What it measures |
//! |-------|------------------|
//! | `mlt_parse` | `mlt_core::parse_layers()` — header parsing only |
//! | `mlt_decode_all` | Full MLT decode (parse + `decode_all()` per layer) |
//! | `mvt_decode` | MVT protobuf decode via `prost` |
//! | `mlt_to_mvt_transcode` | Our full MLT→MVT transcoding pipeline |
//! | `mvt_to_mlt_transcode` | Our full MVT→MLT transcoding pipeline |
//! | `format_detection` | MLT format detection overhead |
//!
//! ## Fixture Sources
//!
//! - MLT: `maplibre-tile-spec/test/expected/tag0x01/omt/`
//! - MVT: `maplibre-tile-spec/test/fixtures/omt/`
//!
//! Zoom levels benchmarked: z0, z4, z7, z13 (matching `mlt-core` convention).
//!
//! ## Comparison with mlt-core & Martin
//!
//! - **mlt-core** benchmarks `parse_layers` and `decode_all` at z4/z7/z13.
//!   Our `mlt_parse` and `mlt_decode_all` groups use the identical fixtures
//!   and methodology, so throughput numbers are directly comparable.
//! - **Martin** has no MLT-specific benchmarks (passthrough only, no transcoding).
//!   Our `mlt_to_mvt_transcode` benchmark is unique — it measures the full
//!   MLT→MVT reverse transcoding pipeline that Martin does not provide.

use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use prost::Message;
use tileserver_rs::sources::{TileCompression, TileData, TileFormat};
use tileserver_rs::transcode::{MvtProto, transcode_tile};

// ---------------------------------------------------------------------------
// Fixture loading
// ---------------------------------------------------------------------------

/// Raw tile bytes loaded at compile time via `include_bytes!`.
struct TileFixture {
    data: &'static [u8],
}

/// Load all MLT fixtures for a given zoom level.
fn mlt_fixtures(zoom: u8) -> Vec<TileFixture> {
    match zoom {
        0 => vec![TileFixture {
            data: include_bytes!("fixtures/0_0_0.mlt"),
        }],
        4 => vec![
            TileFixture {
                data: include_bytes!("fixtures/4_8_10.mlt"),
            },
            TileFixture {
                data: include_bytes!("fixtures/4_8_5.mlt"),
            },
        ],
        7 => vec![
            TileFixture {
                data: include_bytes!("fixtures/7_66_85.mlt"),
            },
            TileFixture {
                data: include_bytes!("fixtures/7_67_85.mlt"),
            },
            TileFixture {
                data: include_bytes!("fixtures/7_67_84.mlt"),
            },
        ],
        13 => vec![
            TileFixture {
                data: include_bytes!("fixtures/13_4265_5467.mlt"),
            },
            TileFixture {
                data: include_bytes!("fixtures/13_4266_5468.mlt"),
            },
            TileFixture {
                data: include_bytes!("fixtures/13_4267_5469.mlt"),
            },
        ],
        _ => vec![],
    }
}

/// Load all MVT fixtures for a given zoom level.
fn mvt_fixtures(zoom: u8) -> Vec<TileFixture> {
    match zoom {
        0 => vec![TileFixture {
            data: include_bytes!("fixtures/0_0_0.mvt"),
        }],
        4 => vec![
            TileFixture {
                data: include_bytes!("fixtures/4_8_10.mvt"),
            },
            TileFixture {
                data: include_bytes!("fixtures/4_8_5.mvt"),
            },
        ],
        7 => vec![
            TileFixture {
                data: include_bytes!("fixtures/7_66_85.mvt"),
            },
            TileFixture {
                data: include_bytes!("fixtures/7_67_85.mvt"),
            },
            TileFixture {
                data: include_bytes!("fixtures/7_67_84.mvt"),
            },
        ],
        13 => vec![
            TileFixture {
                data: include_bytes!("fixtures/13_4265_5467.mvt"),
            },
            TileFixture {
                data: include_bytes!("fixtures/13_4266_5468.mvt"),
            },
            TileFixture {
                data: include_bytes!("fixtures/13_4267_5469.mvt"),
            },
        ],
        _ => vec![],
    }
}

/// Zoom levels to benchmark.
///
/// Matches `mlt-core`'s `BENCHMARKED_ZOOM_LEVELS` (z4, z7, z13 in release;
/// z0 added as baseline for world-overview tiles).
const ZOOM_LEVELS: [u8; 4] = [0, 4, 7, 13];

// ---------------------------------------------------------------------------
// Benchmark: MLT parse (header only, no geometry decode)
// ---------------------------------------------------------------------------

/// Benchmark `mlt_core::parse_layers()` — comparable to mlt-core's own
/// `bench_mlt_parse` benchmark.
///
/// Measures header parsing throughput without decoding geometry or properties.
fn bench_mlt_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("mlt_parse");

    for &zoom in &ZOOM_LEVELS {
        let fixtures = mlt_fixtures(zoom);
        if fixtures.is_empty() {
            continue;
        }

        let total_bytes: usize = fixtures.iter().map(|f| f.data.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        group.bench_with_input(BenchmarkId::new("zoom", zoom), &fixtures, |b, fixtures| {
            b.iter(|| {
                for fixture in fixtures {
                    let _ = mlt_core::Parser::default()
                        .parse_layers(std::hint::black_box(fixture.data));
                }
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: MLT full decode (parse + decode_all)
// ---------------------------------------------------------------------------

/// Benchmark full MLT decode — comparable to mlt-core's own
/// `bench_mlt_decode_all` benchmark.
///
/// Measures parse_layers + decode_all per layer, including geometry and
/// property decoding.
fn bench_mlt_decode_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("mlt_decode_all");

    for &zoom in &ZOOM_LEVELS {
        let fixtures = mlt_fixtures(zoom);
        if fixtures.is_empty() {
            continue;
        }

        let total_bytes: usize = fixtures.iter().map(|f| f.data.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        group.bench_with_input(BenchmarkId::new("zoom", zoom), &fixtures, |b, fixtures| {
            b.iter(|| {
                for fixture in fixtures {
                    if let Ok(layers) =
                        mlt_core::Parser::default().parse_layers(std::hint::black_box(fixture.data))
                    {
                        let mut dec = mlt_core::Decoder::default();
                        for layer in layers {
                            let _ = layer.decode_all(&mut dec);
                        }
                    }
                }
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: MVT protobuf decode (via prost)
// ---------------------------------------------------------------------------

/// Benchmark MVT protobuf decoding using `prost::Message::decode`.
///
/// Comparable to mlt-core's `bench_mvt_parse` — both measure the cost of
/// parsing the MVT protobuf structure (though mlt-core uses `mvt_reader`
/// while we use our prost-derived types).
fn bench_mvt_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("mvt_decode");

    for &zoom in &ZOOM_LEVELS {
        let fixtures = mvt_fixtures(zoom);
        if fixtures.is_empty() {
            continue;
        }

        let total_bytes: usize = fixtures.iter().map(|f| f.data.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        group.bench_with_input(BenchmarkId::new("zoom", zoom), &fixtures, |b, fixtures| {
            b.iter(|| {
                for fixture in fixtures {
                    let _ = MvtProto::Tile::decode(std::hint::black_box(fixture.data));
                }
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: Full MLT→MVT transcoding (our unique pipeline)
// ---------------------------------------------------------------------------

/// Benchmark the complete MLT→MVT transcoding pipeline via `transcode_tile`.
///
/// This is **unique to tileserver-rs** — Martin does not support transcoding.
/// The pipeline: decompress → parse_layers → decode_all → FeatureCollection →
/// MVT protobuf encode.
fn bench_mlt_to_mvt_transcode(c: &mut Criterion) {
    let mut group = c.benchmark_group("mlt_to_mvt_transcode");

    for &zoom in &ZOOM_LEVELS {
        let fixtures = mlt_fixtures(zoom);
        if fixtures.is_empty() {
            continue;
        }

        let total_bytes: usize = fixtures.iter().map(|f| f.data.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        // Pre-wrap fixtures into TileData for the transcode API
        let tile_datas: Vec<TileData> = fixtures
            .iter()
            .map(|f| TileData {
                data: Bytes::from_static(f.data),
                format: TileFormat::Mlt,
                compression: TileCompression::None,
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("zoom", zoom),
            &tile_datas,
            |b, tile_datas| {
                b.iter(|| {
                    for tile in tile_datas {
                        let _ = transcode_tile(std::hint::black_box(tile), TileFormat::Pbf);
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark the complete MVT→MLT transcoding pipeline via `transcode_tile`.
///
/// Exercises `mvt_to_mlt`, which (since mlt-core 0.11) decodes MVT directly
/// into row-oriented `TileLayer`s via `mvt::mvt_to_tile_layers` and re-encodes
/// each via `TileLayer::encode`, replacing the older hand-rolled GeoJSON
/// `FeatureCollection` bridge.
fn bench_mvt_to_mlt_transcode(c: &mut Criterion) {
    let mut group = c.benchmark_group("mvt_to_mlt_transcode");

    for &zoom in &ZOOM_LEVELS {
        let fixtures = mvt_fixtures(zoom);
        if fixtures.is_empty() {
            continue;
        }

        let total_bytes: usize = fixtures.iter().map(|f| f.data.len()).sum();
        group.throughput(Throughput::Bytes(total_bytes as u64));

        let tile_datas: Vec<TileData> = fixtures
            .iter()
            .map(|f| TileData {
                data: Bytes::from_static(f.data),
                format: TileFormat::Pbf,
                compression: TileCompression::None,
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("zoom", zoom),
            &tile_datas,
            |b, tile_datas| {
                b.iter(|| {
                    for tile in tile_datas {
                        let _ = transcode_tile(std::hint::black_box(tile), TileFormat::Mlt);
                    }
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: MLT format detection overhead
// ---------------------------------------------------------------------------

/// Benchmark `detect_mlt_format` — the 7-bit varint + tag check used to
/// identify MLT tiles (same algorithm as Martin).
///
/// Measures the per-tile overhead of format detection, which runs once per
/// source at startup (not per-request).
fn bench_format_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_detection");

    // Test detection on both MLT and MVT bytes
    let mlt_data = include_bytes!("fixtures/0_0_0.mlt");
    let mvt_data = include_bytes!("fixtures/0_0_0.mvt");

    group.throughput(Throughput::Bytes(mlt_data.len() as u64));
    group.bench_function("mlt_tile", |b| {
        b.iter(|| tileserver_rs::detect_mlt_format(std::hint::black_box(mlt_data)));
    });

    group.throughput(Throughput::Bytes(mvt_data.len() as u64));
    group.bench_function("mvt_tile", |b| {
        b.iter(|| tileserver_rs::detect_mlt_format(std::hint::black_box(mvt_data)));
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: MVT protobuf encode (prost)
// ---------------------------------------------------------------------------

/// Benchmark MVT protobuf encoding via `prost` — measures the encoding
/// half of our MLT→MVT pipeline in isolation.
fn bench_mvt_encode(c: &mut Criterion) {
    // Decode a real MVT tile, then benchmark re-encoding it
    let mvt_data = include_bytes!("fixtures/4_8_10.mvt");
    let tile = MvtProto::Tile::decode(&mvt_data[..]).expect("failed to decode MVT fixture");
    let encoded_size = tile.encoded_len();

    let mut group = c.benchmark_group("mvt_encode");
    group.throughput(Throughput::Bytes(encoded_size as u64));

    group.bench_function("z4_tile", |b| {
        b.iter(|| std::hint::black_box(&tile).encode_to_vec());
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: Transcode no-op (same format passthrough)
// ---------------------------------------------------------------------------

/// Benchmark the no-op path when source format matches target format.
/// Should be near-zero overhead (early return).
fn bench_transcode_noop(c: &mut Criterion) {
    let mlt_data = include_bytes!("fixtures/0_0_0.mlt");
    let tile = TileData {
        data: Bytes::from_static(mlt_data),
        format: TileFormat::Mlt,
        compression: TileCompression::None,
    };

    c.bench_function("transcode_noop", |b| {
        b.iter(|| transcode_tile(std::hint::black_box(&tile), TileFormat::Mlt));
    });
}

criterion_group!(
    mlt_benches,
    bench_mlt_parse,
    bench_mlt_decode_all,
    bench_mvt_decode,
    bench_mlt_to_mvt_transcode,
    bench_mvt_to_mlt_transcode,
    bench_format_detection,
    bench_mvt_encode,
    bench_transcode_noop,
);
criterion_main!(mlt_benches);
