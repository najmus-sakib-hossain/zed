//! Benchmarks for dx-security scanner

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dx_security::score::{ScanFindings, calculate_score};

fn bench_score_calculation(c: &mut Criterion) {
    let findings = ScanFindings {
        critical_cves: 2,
        high_cves: 5,
        medium_cves: 10,
        low_cves: 20,
        secrets_leaked: 1,
        binary_inoculation_active: true,
        supply_chain_xor_verified: true,
    };

    c.bench_function("calculate_score", |b| b.iter(|| calculate_score(black_box(&findings))));
}

criterion_group!(benches, bench_score_calculation);
criterion_main!(benches);
