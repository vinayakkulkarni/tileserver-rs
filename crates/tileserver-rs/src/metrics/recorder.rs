//! Public facade for application-side metric recording.
//!
//! Application code only ever calls functions in this module; it never
//! imports `opentelemetry::*` directly. Cardinality enforcement and
//! pre-built label arrays live in [`super::labels`].

use std::sync::OnceLock;
use std::time::Duration;

use opentelemetry::KeyValue;

use super::instruments::{
    HTTP_REQUEST_DURATION_SECONDS, HTTP_REQUESTS_IN_FLIGHT, HTTP_REQUESTS_TOTAL,
    RENDER_DURATION_SECONDS, RENDER_ERRORS_TOTAL, TILE_CACHE_HITS_TOTAL, TILE_CACHE_MISSES_TOTAL,
    TILE_REQUEST_BYTES, TILE_REQUEST_DURATION_SECONDS, TILE_REQUESTS_TOTAL,
};
use super::labels::{Cardinality, LabelBank};
use crate::sources::TileFormat;

static LABEL_BANK: OnceLock<LabelBank> = OnceLock::new();

/// Initialize the global label bank with the configured cardinality.
///
/// Idempotent — second and subsequent calls are silently ignored. Call
/// once during startup before any application code emits metrics.
pub fn init(cardinality: Cardinality) {
    let _ = LABEL_BANK.set(LabelBank::new(cardinality));
}

fn bank() -> &'static LabelBank {
    LABEL_BANK.get_or_init(|| LabelBank::new(Cardinality::Strict))
}

/// Outcome of a tile request, recorded as a label on tile metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileOutcome {
    /// Returned from in-process cache without hitting the source.
    Hit,
    /// Fetched from source successfully.
    Miss,
    /// Source returned `Ok(None)` — tile does not exist.
    NotFound,
    /// Source returned an error.
    Error,
}

impl TileOutcome {
    fn as_label(self) -> &'static str {
        match self {
            Self::Hit => "hit",
            Self::Miss => "miss",
            Self::NotFound => "not_found",
            Self::Error => "error",
        }
    }
}

/// Outcome of a raster render request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderOutcome {
    Success,
    Error,
}

/// HTTP response status class (2xx, 3xx, 4xx, 5xx).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpStatusClass {
    Success,
    Redirection,
    ClientError,
    ServerError,
    Unknown,
}

impl HttpStatusClass {
    #[must_use]
    pub fn from_code(code: u16) -> Self {
        match code {
            200..=299 => Self::Success,
            300..=399 => Self::Redirection,
            400..=499 => Self::ClientError,
            500..=599 => Self::ServerError,
            _ => Self::Unknown,
        }
    }

    fn as_label(self) -> &'static str {
        match self {
            Self::Success => "2xx",
            Self::Redirection => "3xx",
            Self::ClientError => "4xx",
            Self::ServerError => "5xx",
            Self::Unknown => "other",
        }
    }
}

/// Typed event emitted on every completed tile request.
#[derive(Debug)]
pub struct TileEvent<'a> {
    pub source: &'a str,
    pub format: TileFormat,
    pub z: u8,
    pub bytes: usize,
    pub duration: Duration,
    pub outcome: TileOutcome,
}

/// Typed event emitted on every completed HTTP request.
#[derive(Debug)]
pub struct HttpEvent<'a> {
    pub method: &'a str,
    pub route: &'a str,
    pub status: u16,
    pub duration: Duration,
}

/// Typed event emitted on every completed raster render.
#[derive(Debug)]
pub struct RenderEvent<'a> {
    pub style: &'a str,
    pub format: TileFormat,
    pub duration: Duration,
    pub outcome: RenderOutcome,
    pub error_reason: Option<&'a str>,
}

pub fn tile_request_recorded(event: TileEvent<'_>) {
    let labels = bank().tile_labels(
        event.source,
        event.format,
        event.z,
        event.outcome.as_label(),
    );
    let bytes_labels = bank().tile_bytes_labels(event.source, event.format);

    TILE_REQUESTS_TOTAL.add(1, &labels);
    TILE_REQUEST_DURATION_SECONDS.record(event.duration.as_secs_f64(), &labels);
    if event.bytes > 0 {
        TILE_REQUEST_BYTES.record(event.bytes as u64, &bytes_labels);
    }
}

pub fn cache_hit_recorded(source: &str) {
    let labels = bank().cache_labels(source);
    TILE_CACHE_HITS_TOTAL.add(1, &labels);
}

pub fn cache_miss_recorded(source: &str) {
    let labels = bank().cache_labels(source);
    TILE_CACHE_MISSES_TOTAL.add(1, &labels);
}

pub fn http_request_recorded(event: HttpEvent<'_>) {
    let status_class = HttpStatusClass::from_code(event.status).as_label();
    let attrs: [KeyValue; 3] = [
        KeyValue::new("method", event.method.to_string()),
        KeyValue::new("route", event.route.to_string()),
        KeyValue::new("status_class", status_class),
    ];
    let dur_attrs: [KeyValue; 2] = [
        KeyValue::new("method", event.method.to_string()),
        KeyValue::new("route", event.route.to_string()),
    ];
    HTTP_REQUESTS_TOTAL.add(1, &attrs);
    HTTP_REQUEST_DURATION_SECONDS.record(event.duration.as_secs_f64(), &dur_attrs);
}

pub fn http_in_flight_inc() {
    HTTP_REQUESTS_IN_FLIGHT.add(1, &[]);
}

pub fn http_in_flight_dec() {
    HTTP_REQUESTS_IN_FLIGHT.add(-1, &[]);
}

pub fn render_recorded(event: RenderEvent<'_>) {
    let labels = bank().render_labels(event.style, event.format);
    RENDER_DURATION_SECONDS.record(event.duration.as_secs_f64(), &labels);
    if event.outcome == RenderOutcome::Error {
        let reason = event.error_reason.unwrap_or("unknown");
        let err_labels = bank().render_error_labels(event.style, reason);
        RENDER_ERRORS_TOTAL.add(1, &err_labels);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_status_class_buckets() {
        assert_eq!(HttpStatusClass::from_code(200), HttpStatusClass::Success);
        assert_eq!(HttpStatusClass::from_code(204), HttpStatusClass::Success);
        assert_eq!(
            HttpStatusClass::from_code(301),
            HttpStatusClass::Redirection
        );
        assert_eq!(
            HttpStatusClass::from_code(404),
            HttpStatusClass::ClientError
        );
        assert_eq!(
            HttpStatusClass::from_code(500),
            HttpStatusClass::ServerError
        );
        assert_eq!(HttpStatusClass::from_code(199), HttpStatusClass::Unknown);
        assert_eq!(HttpStatusClass::from_code(600), HttpStatusClass::Unknown);
    }

    #[test]
    fn tile_outcome_labels_stable() {
        assert_eq!(TileOutcome::Hit.as_label(), "hit");
        assert_eq!(TileOutcome::Miss.as_label(), "miss");
        assert_eq!(TileOutcome::NotFound.as_label(), "not_found");
        assert_eq!(TileOutcome::Error.as_label(), "error");
    }

    #[test]
    fn http_status_class_label_strings() {
        assert_eq!(HttpStatusClass::Success.as_label(), "2xx");
        assert_eq!(HttpStatusClass::Redirection.as_label(), "3xx");
        assert_eq!(HttpStatusClass::ClientError.as_label(), "4xx");
        assert_eq!(HttpStatusClass::ServerError.as_label(), "5xx");
        assert_eq!(HttpStatusClass::Unknown.as_label(), "other");
    }

    #[test]
    fn render_outcome_equality() {
        assert_eq!(RenderOutcome::Success, RenderOutcome::Success);
        assert_eq!(RenderOutcome::Error, RenderOutcome::Error);
        assert_ne!(RenderOutcome::Success, RenderOutcome::Error);
    }

    #[test]
    fn init_is_idempotent() {
        init(Cardinality::Strict);
        init(Cardinality::Verbose);
        init(Cardinality::Standard);
    }

    #[test]
    fn tile_request_recorded_all_outcomes_with_noop_meter() {
        init(Cardinality::Strict);
        for outcome in [
            TileOutcome::Hit,
            TileOutcome::Miss,
            TileOutcome::NotFound,
            TileOutcome::Error,
        ] {
            tile_request_recorded(TileEvent {
                source: "noop-test",
                format: TileFormat::Pbf,
                z: 14,
                bytes: 1024,
                duration: Duration::from_millis(5),
                outcome,
            });
        }
    }

    #[test]
    fn tile_request_recorded_zero_bytes_skips_bytes_histogram() {
        init(Cardinality::Strict);
        tile_request_recorded(TileEvent {
            source: "zero-bytes",
            format: TileFormat::Pbf,
            z: 0,
            bytes: 0,
            duration: Duration::from_millis(1),
            outcome: TileOutcome::NotFound,
        });
    }

    #[test]
    fn cache_hit_and_miss_recorded_with_noop_meter() {
        init(Cardinality::Strict);
        cache_hit_recorded("source-x");
        cache_hit_recorded("source-x");
        cache_miss_recorded("source-x");
        cache_miss_recorded("source-y");
    }

    #[test]
    fn http_in_flight_inc_dec_balances() {
        init(Cardinality::Strict);
        http_in_flight_inc();
        http_in_flight_inc();
        http_in_flight_dec();
        http_in_flight_dec();
    }

    #[test]
    fn http_request_recorded_all_status_classes() {
        init(Cardinality::Strict);
        for status in [200u16, 301, 404, 500, 100] {
            http_request_recorded(HttpEvent {
                method: "GET",
                route: "/some/route",
                status,
                duration: Duration::from_millis(10),
            });
        }
    }

    #[test]
    fn render_recorded_success_and_error_paths() {
        init(Cardinality::Strict);
        render_recorded(RenderEvent {
            style: "ok-style",
            format: TileFormat::Png,
            duration: Duration::from_millis(50),
            outcome: RenderOutcome::Success,
            error_reason: None,
        });
        render_recorded(RenderEvent {
            style: "fail-style",
            format: TileFormat::Webp,
            duration: Duration::from_millis(75),
            outcome: RenderOutcome::Error,
            error_reason: Some("oom"),
        });
        render_recorded(RenderEvent {
            style: "fail-no-reason",
            format: TileFormat::Jpeg,
            duration: Duration::from_millis(20),
            outcome: RenderOutcome::Error,
            error_reason: None,
        });
    }
}
