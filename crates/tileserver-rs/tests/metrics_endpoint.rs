//! Integration tests for the Prometheus `/metrics` endpoint.
//!
//! Tests are consolidated into one happy-path function because the
//! `LazyLock` instrument cache (and the OpenTelemetry SDK's internal
//! `Counter<T>` binding) attaches to the first installed
//! [`SdkMeterProvider`] and cannot be re-pointed at a fresh provider.
//! Per-test fresh providers therefore observe no recordings — the
//! cached instruments keep writing to the original (orphaned) provider.
//! Aggregating the assertions into a single `#[tokio::test]` keeps a
//! single harness alive for the entire process, which is the only
//! correct shape given the upstream constraint.

use std::net::SocketAddr;
use std::time::Duration;

use opentelemetry_prometheus_text_exporter::PrometheusExporter;
use opentelemetry_sdk::metrics::SdkMeterProvider;

use tileserver_rs::metrics::{
    self, Cardinality, RenderEvent, RenderOutcome, TileEvent, TileOutcome,
};
use tileserver_rs::sources::TileFormat;

struct TestHarness {
    addr: SocketAddr,
    _provider: SdkMeterProvider,
    _exporter: PrometheusExporter,
}

async fn install_harness() -> TestHarness {
    let exporter = PrometheusExporter::new();
    let provider = SdkMeterProvider::builder()
        .with_reader(exporter.clone())
        .build();
    opentelemetry::global::set_meter_provider(provider.clone());
    metrics::init(Cardinality::Strict);

    let bind: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let handle = metrics::spawn_metrics_server(bind, "/metrics".to_string(), exporter.clone())
        .await
        .expect("metrics server bind failed");
    tokio::time::sleep(Duration::from_millis(20)).await;

    TestHarness {
        addr: handle.addr,
        _provider: provider,
        _exporter: exporter,
    }
}

async fn fetch(addr: SocketAddr, path: &str) -> reqwest::Response {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");
    client
        .get(format!("http://{addr}{path}"))
        .send()
        .await
        .expect("scrape")
}

#[tokio::test]
async fn metrics_endpoint_e2e_happy_path() {
    let h = install_harness().await;

    metrics::tile_request_recorded(TileEvent {
        source: "openmaptiles",
        format: TileFormat::Pbf,
        z: 14,
        bytes: 4096,
        duration: Duration::from_millis(85),
        outcome: TileOutcome::Hit,
    });
    for &z in &[3u8, 10, 18] {
        metrics::tile_request_recorded(TileEvent {
            source: "s",
            format: TileFormat::Pbf,
            z,
            bytes: 100,
            duration: Duration::from_millis(10),
            outcome: TileOutcome::Miss,
        });
    }
    metrics::cache_hit_recorded("openmaptiles");
    metrics::cache_hit_recorded("openmaptiles");
    metrics::cache_miss_recorded("openmaptiles");
    metrics::render_recorded(RenderEvent {
        style: "osm-bright",
        format: TileFormat::Png,
        duration: Duration::from_millis(120),
        outcome: RenderOutcome::Success,
        error_reason: None,
    });

    let resp = fetch(h.addr, "/metrics").await;
    assert_eq!(resp.status(), 200, "scrape must return 200");
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(ct.starts_with("text/plain"), "content-type: {ct}");
    assert!(ct.contains("version=0.0.4"), "missing 0.0.4 in: {ct}");
    let body = resp.text().await.expect("body");

    assert!(
        body.contains("tile_requests_total"),
        "missing tile_requests_total"
    );
    assert!(body.contains("source=\"openmaptiles\""), "missing source");
    assert!(body.contains("format=\"pbf\""), "missing format=pbf");
    assert!(body.contains("outcome=\"hit\""), "missing outcome=hit");
    assert!(body.contains("outcome=\"miss\""), "missing outcome=miss");

    assert!(
        body.contains("tile_request_duration_seconds"),
        "missing duration histogram"
    );
    assert!(
        body.contains("tile_request_bytes"),
        "missing bytes histogram"
    );

    assert!(body.contains("z_bucket=\"low\""), "missing z_bucket=low");
    assert!(body.contains("z_bucket=\"mid\""), "missing z_bucket=mid");
    assert!(body.contains("z_bucket=\"high\""), "missing z_bucket=high");
    assert!(
        !body.contains("z_bucket=\"3\""),
        "strict must not emit raw z labels"
    );

    assert!(
        body.contains("tile_cache_hits_total"),
        "missing cache_hits_total"
    );
    assert!(
        body.contains("tile_cache_misses_total"),
        "missing cache_misses_total"
    );

    assert!(
        body.contains("render_duration_seconds"),
        "missing render_duration_seconds"
    );
    assert!(body.contains("style=\"osm-bright\""), "missing style label");
    assert!(body.contains("format=\"png\""), "missing format=png");

    assert!(body.contains("# TYPE"), "missing # TYPE comment");
    assert!(body.contains("# HELP"), "missing # HELP comment");

    let health_resp = fetch(h.addr, "/metrics/health").await;
    assert_eq!(health_resp.status(), 200);
    assert_eq!(health_resp.text().await.expect("body"), "OK");
}

#[tokio::test]
async fn cardinality_verbose_label_bank() {
    let bank = metrics::LabelBank::new(metrics::Cardinality::Verbose);
    assert_eq!(bank.cardinality(), metrics::Cardinality::Verbose);
}
