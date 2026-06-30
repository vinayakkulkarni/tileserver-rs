//! Benchmarks for DEM (Digital Elevation Model) hot paths.
//!
//! Initialised as a placeholder so `cargo bench --bench dem --features dem`
//! resolves the manifest entry. Real benchmarks for the Terrarium / Mapbox-RGB
//! encoders + a full DEM tile encode land in a follow-up wave once the
//! encoder pure fns settle. See `sources/dem.rs::encode_*`.

use criterion::{Criterion, criterion_group, criterion_main};

fn bench_placeholder(c: &mut Criterion) {
    c.bench_function("dem_placeholder", |b| b.iter(|| 1 + 1));
}

criterion_group!(benches, bench_placeholder);
criterion_main!(benches);
