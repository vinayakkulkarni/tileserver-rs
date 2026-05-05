//! OpenTelemetry tracing and metrics initialization.
//!
//! A single [`SdkMeterProvider`] feeds both the OTLP push reader (existing)
//! and the Prometheus pull reader (new, via `opentelemetry-prometheus-text-exporter`).
//! See `docs/superpowers/specs/2026-05-04-prometheus-metrics-design.md` §3.

use std::time::Duration;

use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_prometheus_text_exporter::PrometheusExporter;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::{Sampler, SdkTracerProvider};
use tracing::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{Layer, registry::LookupSpan};

use crate::config::TelemetryConfig;

static TRACER_PROVIDER: std::sync::OnceLock<SdkTracerProvider> = std::sync::OnceLock::new();
static METER_PROVIDER: std::sync::OnceLock<SdkMeterProvider> = std::sync::OnceLock::new();

/// Output of [`init_telemetry`]: the tracing layer (if enabled) and the
/// Prometheus pull exporter (if configured).
///
/// Both fields are `Option` because tracing and metrics are independently
/// opt-in: an operator can enable one without the other (or neither, or
/// both). The caller wires the layer into `tracing_subscriber::Registry`
/// and passes the exporter to [`crate::metrics::spawn_metrics_server`].
pub struct TelemetryOutput<S>
where
    S: Subscriber + for<'span> LookupSpan<'span> + Send + Sync,
{
    pub tracing_layer: Option<Box<dyn Layer<S> + Send + Sync>>,
    pub prometheus_exporter: Option<PrometheusExporter>,
}

/// Initialize OpenTelemetry tracing and metrics from the [`TelemetryConfig`].
///
/// # Why `Box<dyn Layer>` (not `impl Layer`)
///
/// The concrete tracing-layer type is internal to `tracing-opentelemetry`
/// and varies by configuration. `impl Trait` cannot be used in return
/// position with `Option` when one branch returns `None`. The layer is
/// composed into a dynamic subscriber chain at runtime via
/// `tracing_subscriber::Registry::with()`, which accepts `Box<dyn Layer>`.
pub fn init_telemetry<S>(config: &TelemetryConfig) -> TelemetryOutput<S>
where
    S: Subscriber + for<'span> LookupSpan<'span> + Send + Sync,
{
    let resource = Resource::builder()
        .with_service_name(config.service_name.clone())
        .with_attribute(KeyValue::new("service.version", env!("CARGO_PKG_VERSION")))
        .build();

    let tracing_layer = if config.enabled {
        init_tracing(config, resource.clone())
    } else {
        tracing::info!("OpenTelemetry tracing disabled");
        None
    };

    let prometheus_exporter = init_metrics_provider(config, resource);

    TelemetryOutput {
        tracing_layer,
        prometheus_exporter,
    }
}

fn init_tracing<S>(
    config: &TelemetryConfig,
    resource: Resource,
) -> Option<Box<dyn Layer<S> + Send + Sync>>
where
    S: Subscriber + for<'span> LookupSpan<'span> + Send + Sync,
{
    let exporter = match opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&config.endpoint)
        .build()
    {
        Ok(exp) => exp,
        Err(e) => {
            tracing::warn!("Failed to create OTLP span exporter: {e}. Tracing disabled.");
            return None;
        }
    };

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_sampler(Sampler::TraceIdRatioBased(config.sample_rate))
        .with_resource(resource)
        .build();

    let tracer = provider.tracer("tileserver-rs");
    let _ = TRACER_PROVIDER.set(provider.clone());
    opentelemetry::global::set_tracer_provider(provider);

    Some(Box::new(OpenTelemetryLayer::new(tracer)))
}

fn init_metrics_provider(
    config: &TelemetryConfig,
    resource: Resource,
) -> Option<PrometheusExporter> {
    let prometheus_wanted = config.prometheus_bind.is_some();
    let otlp_wanted = config.metrics_enabled && config.enabled;

    if !prometheus_wanted && !otlp_wanted {
        return None;
    }

    let mut builder = SdkMeterProvider::builder().with_resource(resource);

    let mut otlp_attached = false;
    if otlp_wanted {
        match opentelemetry_otlp::MetricExporter::builder()
            .with_tonic()
            .with_endpoint(&config.endpoint)
            .build()
        {
            Ok(metrics_exporter) => {
                let reader = PeriodicReader::builder(metrics_exporter)
                    .with_interval(Duration::from_secs(config.metrics_export_interval_secs))
                    .build();
                builder = builder.with_reader(reader);
                otlp_attached = true;
            }
            Err(e) => {
                tracing::warn!("Failed to create OTLP metric exporter: {e}. OTLP push disabled.");
            }
        }
    }

    let prom_exporter = if prometheus_wanted {
        let exp = PrometheusExporter::new();
        builder = builder.with_reader(exp.clone());
        Some(exp)
    } else {
        None
    };

    if !otlp_attached && prom_exporter.is_none() {
        return None;
    }

    let provider = builder.build();
    let _ = METER_PROVIDER.set(provider.clone());
    opentelemetry::global::set_meter_provider(provider);

    tracing::info!(
        otlp_metrics = otlp_attached,
        prometheus_metrics = prom_exporter.is_some(),
        service_name = %config.service_name,
        "OpenTelemetry metrics initialized"
    );

    prom_exporter
}

pub fn shutdown_telemetry() {
    if let Some(meter_provider) = METER_PROVIDER.get()
        && let Err(e) = meter_provider.shutdown()
    {
        tracing::warn!("Metrics shutdown error: {e}");
    }
    if let Some(provider) = TRACER_PROVIDER.get()
        && let Err(e) = provider.shutdown()
    {
        tracing::warn!("Tracer shutdown error: {e}");
    }
    tracing::debug!("OpenTelemetry shutdown complete");
}
