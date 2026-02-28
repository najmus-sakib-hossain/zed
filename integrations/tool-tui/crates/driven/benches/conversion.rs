//! Conversion benchmarks

use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_conversion(c: &mut Criterion) {
    c.bench_function("markdown_to_binary", |b| {
        let markdown = r#"
## Persona
You are an expert engineer.

## Standards
### Style
- Use consistent formatting
"#;
        b.iter(|| black_box(markdown.len()))
    });
}

criterion_group!(benches, benchmark_conversion);
criterion_main!(benches);
