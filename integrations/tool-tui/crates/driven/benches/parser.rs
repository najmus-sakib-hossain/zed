//! Parser benchmarks

use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_parsers(c: &mut Criterion) {
    c.bench_function("parse_markdown", |b| {
        let input = r#"
## Persona
You are an expert.

## Standards
### Style
- Format code
"#;
        b.iter(|| black_box(input.lines().count()))
    });
}

criterion_group!(benches, benchmark_parsers);
criterion_main!(benches);
