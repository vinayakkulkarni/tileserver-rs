use std::time::Duration;

use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::{Sampler, SdkTracerProvider};
use opentelemetry_sdk::Resource;
use tracing::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{registry::LookupSpan, Layer};

use crate::config::TelemetryConfig;

static TRACER_PROVIDER: std::sync::OnceLock<SdkTracerProvider> = std::sync::OnceLock::new();
static METER_PROVIDER: std::sync::OnceLock<SdkMeterProvider> = std::sync::OnceLock::new();

pub fn init_telemetry<S>(config: &TelemetryConfig) -> Option<Box<dyn Layer<S> + Send + Sync>>
where
    S: Subscriber + for<'span> LookupSpan<'span> + Send + Sync,
{
    if !config.enabled {
        tracing::info!("OpenTelemetry disabled");
        return None;
    }

    let resource = Resource::builder()
        .with_service_name(config.service_name.clone())
        .with_attribute(KeyValue::new("service.version", env!("CARGO_PKG_VERSION")))
        .build();

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&config.endpoint)
        .build();

    let exporter = match exporter {
        Ok(exp) => exp,
        Err(e) => {
            tracing::warn!("Failed to create OTLP exporter: {}. Telemetry disabled.", e);
            return None;
        }
    };

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_sampler(Sampler::TraceIdRatioBased(config.sample_rate))
        .with_resource(resource.clone())
        .build();

    let tracer = provider.tracer("tileserver-rs");

    let _ = TRACER_PROVIDER.set(provider.clone());
    opentelemetry::global::set_tracer_provider(provider);

    let metrics_initialized = if config.metrics_enabled {
        match opentelemetry_otlp::MetricExporter::builder()
            .with_tonic()
            .with_endpoint(&config.endpoint)
            .build()
        {
            Ok(metrics_exporter) => {
                let reader = PeriodicReader::builder(metrics_exporter)
                    .with_interval(Duration::from_secs(config.metrics_export_interval_secs))
                    .build();

                let meter_provider = SdkMeterProvider::builder()
                    .with_reader(reader)
                    .with_resource(resource)
                    .build();

                let _ = METER_PROVIDER.set(meter_provider.clone());
                opentelemetry::global::set_meter_provider(meter_provider);
                true
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to create OTLP metric exporter: {}. Metrics disabled.",
                    e
                );
                false
            }
        }
    } else {
        false
    };

    tracing::info!(
        endpoint = %config.endpoint,
        service_name = %config.service_name,
        sample_rate = config.sample_rate,
        metrics = metrics_initialized,
        "OpenTelemetry initialized"
    );

    Some(Box::new(OpenTelemetryLayer::new(tracer)))
}

pub fn shutdown_telemetry() {
    if let Some(meter_provider) = METER_PROVIDER.get() {
        if let Err(e) = meter_provider.shutdown() {
            tracing::warn!("Metrics shutdown error: {}", e);
        }
    }
    if let Some(provider) = TRACER_PROVIDER.get() {
        if let Err(e) = provider.shutdown() {
            tracing::warn!("Tracer shutdown error: {}", e);
        }
    }
    tracing::debug!("OpenTelemetry shutdown complete");
}
