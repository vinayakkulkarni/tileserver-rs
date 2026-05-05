//! Benchmarks the per-call overhead of `metrics::tile_request_recorded`.
//!
//! Acceptance gates (per spec §10.1):
//! - `disabled`: < 5 ns (no global meter installed → atomic no-op path)
//! - `strict`:   < 250 ns (LabelBank hit + 2 atomic instrument calls)
//! - `verbose`:  < 750 ns (per-z label, larger HashMap, but still cheap)

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};

use opentelemetry_prometheus_text_exporter::PrometheusExporter;
use opentelemetry_sdk::metrics::SdkMeterProvider;

use tileserver_rs::metrics::{self, Cardinality, TileEvent, TileOutcome};
use tileserver_rs::sources::TileFormat;

fn install_real_meter() {
    let exporter = PrometheusExporter::new();
    let provider = SdkMeterProvider::builder().with_reader(exporter).build();
    opentelemetry::global::set_meter_provider(provider);
}

fn warm_label_bank(source: &str, z: u8) {
    metrics::tile_request_recorded(TileEvent {
        source,
        format: TileFormat::Pbf,
        z,
        bytes: 1024,
        duration: Duration::from_millis(1),
        outcome: TileOutcome::Hit,
    });
}

fn bench_disabled(c: &mut Criterion) {
    metrics::init(Cardinality::Strict);
    warm_label_bank("openmaptiles", 14);
    c.bench_function("tile_request_recorded/disabled", |b| {
        b.iter(|| {
            metrics::tile_request_recorded(black_box(TileEvent {
                source: "openmaptiles",
                format: TileFormat::Pbf,
                z: 14,
                bytes: 1024,
                duration: Duration::from_millis(1),
                outcome: TileOutcome::Hit,
            }));
        });
    });
}

fn bench_strict(c: &mut Criterion) {
    install_real_meter();
    metrics::init(Cardinality::Strict);
    warm_label_bank("openmaptiles", 14);
    c.bench_function("tile_request_recorded/strict", |b| {
        b.iter(|| {
            metrics::tile_request_recorded(black_box(TileEvent {
                source: "openmaptiles",
                format: TileFormat::Pbf,
                z: 14,
                bytes: 1024,
                duration: Duration::from_millis(1),
                outcome: TileOutcome::Hit,
            }));
        });
    });
}

fn bench_verbose(c: &mut Criterion) {
    install_real_meter();
    metrics::init(Cardinality::Verbose);
    for z in 0u8..=22 {
        warm_label_bank("openmaptiles", z);
    }
    c.bench_function("tile_request_recorded/verbose", |b| {
        b.iter(|| {
            metrics::tile_request_recorded(black_box(TileEvent {
                source: "openmaptiles",
                format: TileFormat::Pbf,
                z: 14,
                bytes: 1024,
                duration: Duration::from_millis(1),
                outcome: TileOutcome::Hit,
            }));
        });
    });
}

criterion_group!(benches, bench_disabled, bench_strict, bench_verbose);
criterion_main!(benches);
