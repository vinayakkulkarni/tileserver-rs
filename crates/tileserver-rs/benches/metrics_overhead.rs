//! Criterion benchmarks for `metrics::tile_request_recorded` per-call overhead.
//!
//! Acceptance gates (per spec §10.1):
//! - `disabled`: < 5 ns (no global meter installed → atomic no-op path)
//! - `strict`:   < 250 ns (LabelBank hit + 2 atomic instrument calls)
//!
//! Verbose cardinality cannot be benchmarked in the same binary because the
//! metrics module's `LABEL_BANK` is a `OnceLock<LabelBank>` that can only be
//! initialised once per process. Strict is the production default and is the
//! merge-blocking budget. A separate `metrics_overhead_verbose` binary can be
//! added later if verbose budget enforcement becomes necessary.

use std::hint::black_box;
use std::sync::OnceLock;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use opentelemetry_sdk::metrics::SdkMeterProvider;

use tileserver_rs::metrics::{self, Cardinality, TileEvent, TileOutcome};
use tileserver_rs::sources::TileFormat;

fn make_event() -> TileEvent<'static> {
    TileEvent {
        source: "openmaptiles",
        format: TileFormat::Pbf,
        z: 14,
        bytes: 4096,
        duration: Duration::from_millis(5),
        outcome: TileOutcome::Hit,
    }
}

fn clone_event(event: &TileEvent<'static>) -> TileEvent<'static> {
    TileEvent {
        source: event.source,
        format: event.format,
        z: event.z,
        bytes: event.bytes,
        duration: event.duration,
        outcome: event.outcome,
    }
}

fn bench_disabled(c: &mut Criterion) {
    // No OTel provider installed: relies on the global no-op MeterProvider.
    // Do NOT call `metrics::init()` here — that would consume the OnceLock and
    // contaminate the strict bench in the same binary.
    let mut group = c.benchmark_group("tile_request_recorded/disabled");
    group.measurement_time(Duration::from_secs(5));
    group.bench_function("noop_provider", |b| {
        let event = make_event();
        b.iter(|| {
            metrics::tile_request_recorded(black_box(clone_event(&event)));
        });
    });
    group.finish();
}

static PROVIDER: OnceLock<SdkMeterProvider> = OnceLock::new();

fn init_strict_provider() -> &'static SdkMeterProvider {
    PROVIDER.get_or_init(|| {
        let provider = SdkMeterProvider::builder().build();
        opentelemetry::global::set_meter_provider(provider.clone());
        metrics::init(Cardinality::Strict);
        provider
    })
}

fn bench_strict(c: &mut Criterion) {
    let _ = init_strict_provider();

    let mut group = c.benchmark_group("tile_request_recorded/strict");
    group.measurement_time(Duration::from_secs(10));
    group.bench_function("in_memory_strict", |b| {
        let event = make_event();
        b.iter(|| {
            metrics::tile_request_recorded(black_box(clone_event(&event)));
        });
    });
    group.finish();
}

criterion_group!(benches_disabled, bench_disabled);
criterion_group!(benches_strict, bench_strict);
criterion_main!(benches_disabled, benches_strict);
