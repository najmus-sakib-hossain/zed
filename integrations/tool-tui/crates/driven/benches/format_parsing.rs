//! Format parsing benchmarks

use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_format_parsing(c: &mut Criterion) {
    c.bench_function("binary_header_parse", |b| {
        // Simulate header parsing
        let header = [
            0x44, 0x52, 0x56, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        b.iter(|| black_box(&header[0..4]))
    });

    c.bench_function("string_table_lookup", |b| {
        let strings = vec!["test", "example", "rule"];
        b.iter(|| black_box(strings.get(1)))
    });
}

criterion_group!(benches, benchmark_format_parsing);
criterion_main!(benches);
