//! Benchmarks for the style-rewriting hot path used on every
//! `GET /styles/{id}/style.json` and `GET /styles/{id}/{z}/{x}/{y}.png`.
//!
//! `rewrite_style_for_api` rewrites every relative tile/source URL in a
//! MapLibre style JSON to absolute, forwards query strings (e.g. `?key=`),
//! and injects the MLT encoding hint where applicable. It runs once per
//! style.json request and once per native render, so its cost is on the
//! same critical path as tile delivery.
//!
//! Run with: `cargo bench --bench styles`

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tileserver_rs::{SourceManager, UrlQueryParams, rewrite_style_for_api};

const PROTOMAPS_LIGHT_STYLE: &str = include_str!("../../../data/styles/protomaps-light/style.json");

const TINY_STYLE: &str = r#"{
  "version": 8,
  "sources": {
    "openmaptiles": {
      "type": "vector",
      "url": "/data/openmaptiles.json"
    }
  },
  "sprite": "/sprites/openmaptiles",
  "glyphs": "/fonts/{fontstack}/{range}.pbf",
  "layers": []
}"#;

fn parsed(json: &str) -> serde_json::Value {
    serde_json::from_str(json).expect("style fixture must parse")
}

fn bench_rewrite(c: &mut Criterion) {
    let mut group = c.benchmark_group("style_rewrite");

    let base_url = "http://localhost:8080";
    let no_key = UrlQueryParams::default();
    let with_key = UrlQueryParams::with_key(Some("api_key_123".into()));
    let sources = SourceManager::new();

    let tiny = parsed(TINY_STYLE);
    group.bench_function("tiny_no_key", |b| {
        b.iter(|| {
            black_box(rewrite_style_for_api(
                black_box(&tiny),
                black_box(base_url),
                black_box(&no_key),
                black_box(&sources),
            ))
        });
    });

    group.bench_function("tiny_with_key", |b| {
        b.iter(|| {
            black_box(rewrite_style_for_api(
                black_box(&tiny),
                black_box(base_url),
                black_box(&with_key),
                black_box(&sources),
            ))
        });
    });

    let protomaps = parsed(PROTOMAPS_LIGHT_STYLE);
    group.bench_function("protomaps_light_no_key", |b| {
        b.iter(|| {
            black_box(rewrite_style_for_api(
                black_box(&protomaps),
                black_box(base_url),
                black_box(&no_key),
                black_box(&sources),
            ))
        });
    });

    group.bench_function("protomaps_light_with_key", |b| {
        b.iter(|| {
            black_box(rewrite_style_for_api(
                black_box(&protomaps),
                black_box(base_url),
                black_box(&with_key),
                black_box(&sources),
            ))
        });
    });

    group.finish();
}

criterion_group!(benches, bench_rewrite);
criterion_main!(benches);
