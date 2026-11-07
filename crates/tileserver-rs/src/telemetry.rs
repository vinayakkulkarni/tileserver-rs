//! OpenTelemetry tracing and metrics initialization.
//!
//! A single [`SdkMeterProvider`] feeds both the OTLP push reader (existing)
//! and the Prometheus pull reader (via the official `opentelemetry-prometheus`
//! crate paired with `prometheus::Registry`).
//! See `docs/superpowers/specs/2026-05-04-prometheus-metrics-design.md` §3.

use std::time::Duration;

use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::{MeterProviderBuilder, PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::{Sampler, SdkTracerProvider};
use prometheus::Registry;
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
    pub prometheus_registry: Option<Registry>,
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

    let prometheus_registry = init_metrics_provider(config, resource);

    TelemetryOutput {
        tracing_layer,
        prometheus_registry,
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

fn init_metrics_provider(config: &TelemetryConfig, resource: Resource) -> Option<Registry> {
    let otlp_wanted = config.metrics_enabled && config.enabled;
    if config.prometheus_bind.is_none() && !otlp_wanted {
        return None;
    }
    let builder = SdkMeterProvider::builder().with_resource(resource);
    let (builder, otlp_attached) = if otlp_wanted {
        attach_otlp_metrics(builder, config)
    } else {
        (builder, false)
    };
    let (builder, prom_registry) = if config.prometheus_bind.is_some() {
        attach_prometheus(builder)
    } else {
        (builder, None)
    };
    if !otlp_attached && prom_registry.is_none() {
        return None;
    }
    let provider = builder.build();
    let _ = METER_PROVIDER.set(provider.clone());
    opentelemetry::global::set_meter_provider(provider);
    tracing::info!(
        otlp_metrics = otlp_attached,
        prometheus_metrics = prom_registry.is_some(),
        service_name = %config.service_name,
        "OpenTelemetry metrics initialized"
    );
    prom_registry
}

/// Add the OTLP push reader. `false` when the exporter cannot be built.
fn attach_otlp_metrics(
    builder: MeterProviderBuilder,
    config: &TelemetryConfig,
) -> (MeterProviderBuilder, bool) {
    match opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(&config.endpoint)
        .build()
    {
        Ok(exporter) => {
            let reader = PeriodicReader::builder(exporter)
                .with_interval(Duration::from_secs(config.metrics_export_interval_secs))
                .build();
            (builder.with_reader(reader), true)
        }
        Err(e) => {
            tracing::warn!("Failed to create OTLP metric exporter: {e}. OTLP push disabled.");
            (builder, false)
        }
    }
}

/// Add the Prometheus pull reader and return its registry.
fn attach_prometheus(builder: MeterProviderBuilder) -> (MeterProviderBuilder, Option<Registry>) {
    let registry = Registry::new();
    match opentelemetry_prometheus::exporter()
        .with_registry(registry.clone())
        .build()
    {
        Ok(exporter) => (builder.with_reader(exporter), Some(registry)),
        Err(e) => {
            tracing::warn!("Failed to build Prometheus exporter: {e}. Prometheus disabled.");
            (builder, None)
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::metrics::MeterProvider as _;

    fn config() -> TelemetryConfig {
        TelemetryConfig::default()
    }

    #[test]
    fn no_metrics_wanted_means_no_provider() {
        let mut c = config();
        c.enabled = false;
        c.prometheus_bind = None;
        assert!(init_metrics_provider(&c, Resource::builder().build()).is_none());
    }

    #[test]
    fn prometheus_reader_exposes_recorded_metrics() {
        let (builder, registry) = attach_prometheus(SdkMeterProvider::builder());
        let registry = registry.expect("prometheus exporter builds");
        let provider = builder.build();
        provider
            .meter("test")
            .u64_counter("tiles_served")
            .build()
            .add(3, &[]);
        let names: Vec<String> = registry
            .gather()
            .iter()
            .map(|f| f.name().to_string())
            .collect();
        assert!(
            names.iter().any(|n| n.starts_with("tiles_served")),
            "{names:?}"
        );
    }

    #[tokio::test]
    async fn both_readers_give_a_prometheus_registry() {
        let mut c = config();
        c.enabled = true;
        c.metrics_enabled = true;
        c.endpoint = "http://127.0.0.1:4317".to_string();
        c.prometheus_bind = Some("127.0.0.1:0".to_string());
        assert!(init_metrics_provider(&c, Resource::builder().build()).is_some());
    }

    #[tokio::test]
    async fn otlp_reader_attaches_for_a_valid_endpoint() {
        let mut c = config();
        c.endpoint = "http://127.0.0.1:4317".to_string();
        let (builder, attached) = attach_otlp_metrics(SdkMeterProvider::builder(), &c);
        assert!(attached);
        drop(builder.build());
    }
}
