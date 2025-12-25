//! OpenTelemetry integration for distributed tracing and metrics.
//!
//! This module provides a unified setup for exporting traces and metrics
//! to an OpenTelemetry collector via OTLP (gRPC).

use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace::Sampler, Resource};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use tracing::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{registry::LookupSpan, Layer};

use crate::config::TelemetryConfig;

/// Initialize OpenTelemetry with the given configuration.
///
/// Returns a tracing layer that can be composed with other layers.
pub fn init_telemetry<S>(config: &TelemetryConfig) -> Option<Box<dyn Layer<S> + Send + Sync>>
where
    S: Subscriber + for<'span> LookupSpan<'span> + Send + Sync,
{
    if !config.enabled {
        tracing::info!("OpenTelemetry disabled");
        return None;
    }

    let resource = Resource::new(vec![
        KeyValue::new(SERVICE_NAME, config.service_name.clone()),
        KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
    ]);

    // Build the OTLP trace exporter
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

    // Build the tracer provider using the new builder API
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_sampler(Sampler::TraceIdRatioBased(config.sample_rate))
        .with_resource(resource)
        .build();

    let tracer = provider.tracer("tileserver-rs");

    // Set the global tracer provider
    opentelemetry::global::set_tracer_provider(provider);

    tracing::info!(
        endpoint = %config.endpoint,
        service_name = %config.service_name,
        sample_rate = config.sample_rate,
        "OpenTelemetry initialized"
    );

    Some(Box::new(OpenTelemetryLayer::new(tracer)))
}

/// Shutdown OpenTelemetry gracefully.
///
/// This should be called when the application is shutting down to ensure
/// all pending traces are exported.
pub fn shutdown_telemetry() {
    opentelemetry::global::shutdown_tracer_provider();
    tracing::debug!("OpenTelemetry shutdown complete");
}
