//! Lazy-initialized OpenTelemetry instrument handles.
//!
//! Instruments are created on first use against the global meter provider
//! installed by `crate::telemetry`. If telemetry is disabled, the global
//! meter is a no-op provider and instrument calls compile to atomic no-ops.

use std::sync::LazyLock;

use opentelemetry::global;
use opentelemetry::metrics::{Counter, Histogram, UpDownCounter};

const METER_NAME: &str = "tileserver-rs";

fn meter() -> opentelemetry::metrics::Meter {
    global::meter(METER_NAME)
}

pub(super) static HTTP_REQUESTS_TOTAL: LazyLock<Counter<u64>> = LazyLock::new(|| {
    meter()
        .u64_counter("http_requests_total")
        .with_description("Total HTTP requests by route and status class")
        .build()
});

pub(super) static HTTP_REQUEST_DURATION_SECONDS: LazyLock<Histogram<f64>> = LazyLock::new(|| {
    meter()
        .f64_histogram("http_request_duration_seconds")
        .with_description("HTTP request duration in seconds")
        .with_unit("s")
        .build()
});

pub(super) static HTTP_REQUESTS_IN_FLIGHT: LazyLock<UpDownCounter<i64>> = LazyLock::new(|| {
    meter()
        .i64_up_down_counter("http_requests_in_flight")
        .with_description("HTTP requests currently being processed")
        .build()
});

pub(super) static TILE_REQUESTS_TOTAL: LazyLock<Counter<u64>> = LazyLock::new(|| {
    meter()
        .u64_counter("tile_requests_total")
        .with_description("Total tile requests by source, format, zoom bucket, and outcome")
        .build()
});

pub(super) static TILE_REQUEST_DURATION_SECONDS: LazyLock<Histogram<f64>> = LazyLock::new(|| {
    meter()
        .f64_histogram("tile_request_duration_seconds")
        .with_description("Tile request duration in seconds")
        .with_unit("s")
        .build()
});

pub(super) static TILE_REQUEST_BYTES: LazyLock<Histogram<u64>> = LazyLock::new(|| {
    meter()
        .u64_histogram("tile_request_bytes")
        .with_description("Tile response size in bytes")
        .with_unit("By")
        .build()
});

pub(super) static TILE_CACHE_HITS_TOTAL: LazyLock<Counter<u64>> = LazyLock::new(|| {
    meter()
        .u64_counter("tile_cache_hits_total")
        .with_description("In-process tile cache hits")
        .build()
});

pub(super) static TILE_CACHE_MISSES_TOTAL: LazyLock<Counter<u64>> = LazyLock::new(|| {
    meter()
        .u64_counter("tile_cache_misses_total")
        .with_description("In-process tile cache misses")
        .build()
});

pub(super) static RENDER_DURATION_SECONDS: LazyLock<Histogram<f64>> = LazyLock::new(|| {
    meter()
        .f64_histogram("render_duration_seconds")
        .with_description("Native MapLibre tile render duration in seconds")
        .with_unit("s")
        .build()
});

pub(super) static RENDER_ERRORS_TOTAL: LazyLock<Counter<u64>> = LazyLock::new(|| {
    meter()
        .u64_counter("render_errors_total")
        .with_description("Native MapLibre render failures by reason")
        .build()
});
