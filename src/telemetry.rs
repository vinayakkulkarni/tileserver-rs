use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::{Sampler, SdkTracerProvider};
use opentelemetry_sdk::Resource;
use tracing::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{registry::LookupSpan, Layer};

use crate::config::TelemetryConfig;

static TRACER_PROVIDER: std::sync::OnceLock<SdkTracerProvider> = std::sync::OnceLock::new();

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
        .with_resource(resource)
        .build();

    let tracer = provider.tracer("tileserver-rs");

    let _ = TRACER_PROVIDER.set(provider.clone());
    opentelemetry::global::set_tracer_provider(provider);

    tracing::info!(
        endpoint = %config.endpoint,
        service_name = %config.service_name,
        sample_rate = config.sample_rate,
        "OpenTelemetry initialized"
    );

    Some(Box::new(OpenTelemetryLayer::new(tracer)))
}

pub fn shutdown_telemetry() {
    if let Some(provider) = TRACER_PROVIDER.get() {
        if let Err(e) = provider.shutdown() {
            tracing::warn!("OpenTelemetry shutdown error: {}", e);
        }
    }
    tracing::debug!("OpenTelemetry shutdown complete");
}
