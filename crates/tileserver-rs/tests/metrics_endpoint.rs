//! Integration tests for the Prometheus `/metrics` endpoint.
//!
//! TDD red phase — stubs that compile but panic at runtime.
//! Implementations added in T5.1.

/// Scrape returns HTTP 200 with correct content-type header.
#[tokio::test]
async fn scrape_returns_200_with_prometheus_content_type() {
    todo!("implement in T5.1 — metrics integration tests green phase")
}

/// A tile fetch increments `tile_requests_total` counter by 1.
#[tokio::test]
async fn tile_request_increments_counter() {
    todo!("implement in T5.1 — metrics integration tests green phase")
}

/// A tile fetch records a duration histogram entry.
#[tokio::test]
async fn tile_request_records_duration_histogram() {
    todo!("implement in T5.1 — metrics integration tests green phase")
}

/// When prometheus_bind is None, no listener is spawned but tile serving works.
#[tokio::test]
async fn metrics_disabled_when_bind_unset() {
    todo!("implement in T5.1 — metrics integration tests green phase")
}

/// Strict cardinality collapses z into three buckets: low/mid/high.
#[tokio::test]
async fn cardinality_strict_buckets_z_correctly() {
    todo!("implement in T5.1 — metrics integration tests green phase")
}

/// Verbose cardinality keeps exact numeric z values.
#[tokio::test]
async fn cardinality_verbose_keeps_z_value() {
    todo!("implement in T5.1 — metrics integration tests green phase")
}

/// OTLP and Prometheus readers report the same counter values.
#[tokio::test]
async fn otlp_and_prometheus_emit_same_counts() {
    todo!("implement in T5.1 — metrics integration tests green phase")
}

/// A v2.27.0-style config (no prometheus_bind) produces no listener
/// and identical OTLP behavior to the previous release.
#[tokio::test]
async fn existing_otlp_only_config_unchanged() {
    todo!("implement in T5.1 — metrics integration tests green phase")
}

/// Snapshot test — locks the /metrics body format against regression.
#[tokio::test]
async fn metrics_exposition_format_snapshot() {
    todo!("implement in T5.1 — metrics integration tests green phase")
}
