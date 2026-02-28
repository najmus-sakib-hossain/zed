//! Sync benchmarks

use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_sync(c: &mut Criterion) {
    c.bench_function("sync_5_editors", |b| {
        b.iter(|| {
            // Simulate sync operation
            black_box(5)
        })
    });
}

criterion_group!(benches, benchmark_sync);
criterion_main!(benches);
