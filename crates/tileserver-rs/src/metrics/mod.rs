//! Metrics subsystem: Prometheus `/metrics` endpoint via OpenTelemetry bridge.
//!
//! # Architecture
//!
//! A single OpenTelemetry [`Meter`](opentelemetry::metrics::Meter) feeds both
//! the existing OTLP push pipeline (in `crate::telemetry`) and a new
//! Prometheus pull endpoint hosted on a separate listener. Application code
//! calls typed facade functions on [`recorder`] and never imports
//! `opentelemetry::*` directly.
//!
//! See `docs/superpowers/specs/2026-05-04-prometheus-metrics-design.md`
//! for the full design rationale.

mod http_middleware;
mod instruments;
mod labels;
mod recorder;
mod server;

pub use http_middleware::record_http_request;
pub use labels::{Cardinality, LabelBank};
pub use recorder::{
    HttpEvent, HttpStatusClass, RenderEvent, RenderOutcome, TileEvent, TileOutcome,
    cache_hit_recorded, cache_miss_recorded, http_request_recorded, init, render_recorded,
    tile_request_recorded,
};
pub use server::{MetricsServerHandle, spawn_metrics_server};

pub use opentelemetry_prometheus_text_exporter::PrometheusExporter;
