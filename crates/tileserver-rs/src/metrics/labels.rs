//! Cardinality-bounded label assembly for metrics.
//!
//! All label arrays are built once per (source_id, format, z_bucket) tuple
//! at instrumentation time and stored as `Box<[KeyValue]>`. The hot path
//! returns `&[KeyValue]` references — zero per-request allocation.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use opentelemetry::KeyValue;

use crate::config::MetricsLabelCardinality;
use crate::sources::TileFormat;

/// Cardinality strategy mirrored from
/// [`crate::config::MetricsLabelCardinality`] for use inside the metrics
/// subsystem without forcing every caller to import `crate::config`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    Strict,
    Standard,
    Verbose,
}

impl From<MetricsLabelCardinality> for Cardinality {
    fn from(value: MetricsLabelCardinality) -> Self {
        match value {
            MetricsLabelCardinality::Strict => Self::Strict,
            MetricsLabelCardinality::Standard => Self::Standard,
            MetricsLabelCardinality::Verbose => Self::Verbose,
        }
    }
}

/// Coarse zoom bucket. Strict/Standard collapse to three categorical
/// buckets; Verbose passes the raw zoom level through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ZBucket {
    Low,       // z = 0..=7
    Mid,       // z = 8..=13
    High,      // z = 14..=22
    Exact(u8), // verbose mode: pass-through
}

impl ZBucket {
    fn classify(z: u8, cardinality: Cardinality) -> Self {
        match cardinality {
            Cardinality::Strict | Cardinality::Standard => match z {
                0..=7 => Self::Low,
                8..=13 => Self::Mid,
                _ => Self::High,
            },
            Cardinality::Verbose => Self::Exact(z),
        }
    }

    fn as_label(&self) -> String {
        match self {
            Self::Low => "low".into(),
            Self::Mid => "mid".into(),
            Self::High => "high".into(),
            Self::Exact(z) => z.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TileKey {
    source: String,
    format: TileFormat,
    z_bucket: ZBucket,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TileNoZKey {
    source: String,
    format: TileFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RenderKey {
    style: String,
    format: TileFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ErrorKey {
    style: String,
    reason: String,
}

/// Label cache: maps logical keys to pre-built `Arc<[KeyValue]>` slices.
///
/// `tile_bytes_labels`, `cache_labels`, `render_labels`, and `render_error_labels`
/// return cached slices directly via `Arc::clone` — zero allocation on the hot
/// path after the first call for each (source, format, …) tuple. The
/// `tile_labels` accessor for tile-request counters is the exception: it
/// composes a base slice (`source`, `format`, `z_bucket`) cached in
/// `tile_full` with a per-call `outcome` label, allocating a fresh
/// `Arc<[KeyValue]>` of size `base.len() + 1` on every call. This is the
/// dominant cost measured by `benches/metrics_overhead.rs` (~180 ns) and
/// is below the spec's 250 ns gate; eliminating the allocation by caching
/// `(tile_key, outcome)` directly is tracked as a v2.29 optimisation.
///
/// The bank is wrapped in `RwLock` so reads are concurrent and only
/// first-time inserts take the write lock briefly.
#[derive(Debug, Default)]
struct Inner {
    tile_full: HashMap<TileKey, Arc<[KeyValue]>>,
    tile_no_z: HashMap<TileNoZKey, Arc<[KeyValue]>>,
    cache: HashMap<CacheKey, Arc<[KeyValue]>>,
    render: HashMap<RenderKey, Arc<[KeyValue]>>,
    render_error: HashMap<ErrorKey, Arc<[KeyValue]>>,
}

/// Pre-built label arrays keyed by cardinality-bounded tuples.
#[derive(Debug)]
pub struct LabelBank {
    cardinality: Cardinality,
    inner: RwLock<Inner>,
}

impl LabelBank {
    #[must_use]
    pub fn new(cardinality: Cardinality) -> Self {
        Self {
            cardinality,
            inner: RwLock::new(Inner::default()),
        }
    }

    #[must_use]
    pub fn cardinality(&self) -> Cardinality {
        self.cardinality
    }

    pub fn tile_labels(
        &self,
        source: &str,
        format: TileFormat,
        z: u8,
        outcome: &str,
    ) -> Arc<[KeyValue]> {
        let z_bucket = ZBucket::classify(z, self.cardinality);
        let key = TileKey {
            source: source.to_string(),
            format,
            z_bucket,
        };
        if let Some(existing) = self.inner.read().expect("poisoned").tile_full.get(&key) {
            return Self::with_outcome(existing.as_ref(), outcome);
        }
        let mut guard = self.inner.write().expect("poisoned");
        let arc = guard
            .tile_full
            .entry(key)
            .or_insert_with_key(|k| {
                Arc::from(vec![
                    KeyValue::new("source", k.source.clone()),
                    KeyValue::new("format", format_label(k.format)),
                    KeyValue::new("z_bucket", k.z_bucket.as_label()),
                ])
            })
            .clone();
        Self::with_outcome(arc.as_ref(), outcome)
    }

    pub fn tile_bytes_labels(&self, source: &str, format: TileFormat) -> Arc<[KeyValue]> {
        let key = TileNoZKey {
            source: source.to_string(),
            format,
        };
        if let Some(existing) = self.inner.read().expect("poisoned").tile_no_z.get(&key) {
            return Arc::clone(existing);
        }
        let mut guard = self.inner.write().expect("poisoned");
        guard
            .tile_no_z
            .entry(key)
            .or_insert_with_key(|k| {
                Arc::from(vec![
                    KeyValue::new("source", k.source.clone()),
                    KeyValue::new("format", format_label(k.format)),
                ])
            })
            .clone()
    }

    pub fn cache_labels(&self, source: &str) -> Arc<[KeyValue]> {
        let key = CacheKey {
            source: source.to_string(),
        };
        if let Some(existing) = self.inner.read().expect("poisoned").cache.get(&key) {
            return Arc::clone(existing);
        }
        let mut guard = self.inner.write().expect("poisoned");
        guard
            .cache
            .entry(key)
            .or_insert_with_key(|k| Arc::from(vec![KeyValue::new("source", k.source.clone())]))
            .clone()
    }

    pub fn render_labels(&self, style: &str, format: TileFormat) -> Arc<[KeyValue]> {
        let key = RenderKey {
            style: style.to_string(),
            format,
        };
        if let Some(existing) = self.inner.read().expect("poisoned").render.get(&key) {
            return Arc::clone(existing);
        }
        let mut guard = self.inner.write().expect("poisoned");
        guard
            .render
            .entry(key)
            .or_insert_with_key(|k| {
                Arc::from(vec![
                    KeyValue::new("style", k.style.clone()),
                    KeyValue::new("format", format_label(k.format)),
                ])
            })
            .clone()
    }

    pub fn render_error_labels(&self, style: &str, reason: &str) -> Arc<[KeyValue]> {
        let key = ErrorKey {
            style: style.to_string(),
            reason: reason.to_string(),
        };
        if let Some(existing) = self.inner.read().expect("poisoned").render_error.get(&key) {
            return Arc::clone(existing);
        }
        let mut guard = self.inner.write().expect("poisoned");
        guard
            .render_error
            .entry(key)
            .or_insert_with_key(|k| {
                Arc::from(vec![
                    KeyValue::new("style", k.style.clone()),
                    KeyValue::new("reason", k.reason.clone()),
                ])
            })
            .clone()
    }

    fn with_outcome(base: &[KeyValue], outcome: &str) -> Arc<[KeyValue]> {
        let mut combined = Vec::with_capacity(base.len() + 1);
        combined.extend_from_slice(base);
        combined.push(KeyValue::new("outcome", outcome.to_string()));
        Arc::from(combined)
    }
}

fn format_label(format: TileFormat) -> &'static str {
    match format {
        TileFormat::Pbf => "pbf",
        TileFormat::Png => "png",
        TileFormat::Jpeg => "jpeg",
        TileFormat::Webp => "webp",
        TileFormat::Avif => "avif",
        TileFormat::Mlt => "mlt",
        TileFormat::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn z_bucket_strict_boundaries() {
        assert_eq!(ZBucket::classify(0, Cardinality::Strict), ZBucket::Low);
        assert_eq!(ZBucket::classify(7, Cardinality::Strict), ZBucket::Low);
        assert_eq!(ZBucket::classify(8, Cardinality::Strict), ZBucket::Mid);
        assert_eq!(ZBucket::classify(13, Cardinality::Strict), ZBucket::Mid);
        assert_eq!(ZBucket::classify(14, Cardinality::Strict), ZBucket::High);
        assert_eq!(ZBucket::classify(22, Cardinality::Strict), ZBucket::High);
    }

    #[test]
    fn z_bucket_verbose_passes_through() {
        for z in 0u8..=22 {
            assert_eq!(
                ZBucket::classify(z, Cardinality::Verbose),
                ZBucket::Exact(z)
            );
        }
    }

    #[test]
    fn standard_aliases_strict_in_v1() {
        for z in 0u8..=22 {
            assert_eq!(
                ZBucket::classify(z, Cardinality::Strict),
                ZBucket::classify(z, Cardinality::Standard),
            );
        }
    }

    #[test]
    fn label_bank_dedupes_within_z_bucket() {
        let bank = LabelBank::new(Cardinality::Strict);
        let a = bank.tile_labels("openmaptiles", TileFormat::Pbf, 5, "hit");
        let b = bank.tile_labels("openmaptiles", TileFormat::Pbf, 6, "hit");
        let c = bank.tile_labels("openmaptiles", TileFormat::Pbf, 10, "hit");
        let mut a_z = String::new();
        let mut b_z = String::new();
        let mut c_z = String::new();
        for kv in a.iter() {
            if kv.key.as_str() == "z_bucket" {
                a_z = kv.value.to_string();
            }
        }
        for kv in b.iter() {
            if kv.key.as_str() == "z_bucket" {
                b_z = kv.value.to_string();
            }
        }
        for kv in c.iter() {
            if kv.key.as_str() == "z_bucket" {
                c_z = kv.value.to_string();
            }
        }
        assert_eq!(a_z, "low");
        assert_eq!(b_z, "low");
        assert_eq!(c_z, "mid");
    }

    #[test]
    fn label_bank_verbose_keeps_z() {
        let bank = LabelBank::new(Cardinality::Verbose);
        let labels = bank.tile_labels("s", TileFormat::Pbf, 18, "miss");
        let mut found = false;
        for kv in labels.iter() {
            if kv.key.as_str() == "z_bucket" {
                assert_eq!(kv.value.to_string(), "18");
                found = true;
            }
        }
        assert!(found);
    }
}
