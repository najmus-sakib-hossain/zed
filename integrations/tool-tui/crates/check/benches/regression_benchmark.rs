//! Performance regression benchmarks for dx-check
//!
//! These benchmarks establish baseline performance metrics and detect regressions.
//! Run with: cargo bench --bench regression_benchmark
//!
//! **Validates: Requirement 12.7 - Performance regression tests**

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use dx_check::config::CheckerConfig;
use dx_check::engine::Checker;
use dx_check::rules::RuleRegistry;
use dx_check::scanner::PatternScanner;
use std::path::Path;
use std::time::Duration;
use tempfile::tempdir;

// ============================================================================
// Baseline Performance Targets
// ============================================================================

/// Target: Parse and lint a single file in under 5ms
const SINGLE_FILE_TARGET_MS: u64 = 5;

/// Target: Process 1000 files in under 10 seconds
const THOUSAND_FILES_TARGET_SECS: u64 = 10;

/// Target: Rule loading in under 1ms
const RULE_LOADING_TARGET_MS: u64 = 1;

/// Target: Scanner throughput of at least 1GB/s
const SCANNER_THROUGHPUT_TARGET_GBPS: f64 = 1.0;

// ============================================================================
// Sample Code for Benchmarks
// ============================================================================

const SMALL_JS: &str = r#"
const x = 1;
const y = 2;
const sum = x + y;
"#;

const MEDIUM_JS: &str = r#"
import { useState, useEffect } from 'react';

export function UserProfile({ userId }) {
    const [user, setUser] = useState(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        async function fetchUser() {
            const response = await fetch(`/api/users/${userId}`);
            const data = await response.json();
            setUser(data);
            setLoading(false);
        }
        fetchUser();
    }, [userId]);

    if (loading) {
        return <div>Loading...</div>;
    }

    return (
        <div className="user-profile">
            <h1>{user.name}</h1>
            <p>{user.email}</p>
            <p>{user.bio}</p>
        </div>
    );
}
"#;

const LARGE_JS: &str = include_str!("../tests/fixtures/large_sample.js");

// ============================================================================
// Regression Benchmarks
// ============================================================================

fn bench_single_file_regression(c: &mut Criterion) {
    let checker = Checker::new(CheckerConfig::default());

    let mut group = c.benchmark_group("regression/single_file");
    group.measurement_time(Duration::from_secs(10));

    // Small file benchmark
    group.throughput(Throughput::Bytes(SMALL_JS.len() as u64));
    group.bench_function("small_js", |b| {
        b.iter(|| black_box(checker.check_source(Path::new("test.js"), SMALL_JS)))
    });

    // Medium file benchmark
    group.throughput(Throughput::Bytes(MEDIUM_JS.len() as u64));
    group.bench_function("medium_js", |b| {
        b.iter(|| black_box(checker.check_source(Path::new("test.js"), MEDIUM_JS)))
    });

    group.finish();
}

fn bench_multi_file_regression(c: &mut Criterion) {
    let mut group = c.benchmark_group("regression/multi_file");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(20);

    for file_count in [10, 50, 100, 500].iter() {
        let temp_dir = tempdir().unwrap();

        // Create test files
        for i in 0..*file_count {
            let filename = format!("file_{}.js", i);
            let content = if i % 5 == 0 { MEDIUM_JS } else { SMALL_JS };
            std::fs::write(temp_dir.path().join(&filename), content).unwrap();
        }

        let checker = Checker::with_auto_detect(temp_dir.path());

        group.bench_with_input(BenchmarkId::new("parallel", file_count), file_count, |b, _| {
            b.iter(|| black_box(checker.check_path(temp_dir.path())))
        });
    }

    group.finish();
}

fn bench_rule_loading_regression(c: &mut Criterion) {
    let mut group = c.benchmark_group("regression/rule_loading");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("create_registry", |b| b.iter(|| black_box(RuleRegistry::new())));

    group.bench_function("create_registry_with_builtins", |b| {
        b.iter(|| black_box(RuleRegistry::with_builtins()))
    });

    group.bench_function("create_checker", |b| {
        b.iter(|| black_box(Checker::new(CheckerConfig::default())))
    });

    group.finish();
}

fn bench_scanner_regression(c: &mut Criterion) {
    let scanner = PatternScanner::new();

    let mut group = c.benchmark_group("regression/scanner");
    group.measurement_time(Duration::from_secs(10));

    // Test with various sizes
    for size_kb in [1, 10, 100, 1000].iter() {
        let data = MEDIUM_JS.repeat(*size_kb * 1024 / MEDIUM_JS.len());
        let data_bytes = data.as_bytes();

        group.throughput(Throughput::Bytes(data_bytes.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("scan", format!("{}KB", size_kb)),
            data_bytes,
            |b, data| b.iter(|| black_box(scanner.scan(data))),
        );
    }

    group.finish();
}

fn bench_memory_regression(c: &mut Criterion) {
    let mut group = c.benchmark_group("regression/memory");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);

    // Benchmark with increasing file counts to detect memory leaks
    for file_count in [100, 500, 1000].iter() {
        let temp_dir = tempdir().unwrap();

        for i in 0..*file_count {
            let filename = format!("file_{}.js", i);
            std::fs::write(temp_dir.path().join(&filename), SMALL_JS).unwrap();
        }

        let checker = Checker::with_auto_detect(temp_dir.path());

        group.bench_with_input(BenchmarkId::new("check_files", file_count), file_count, |b, _| {
            b.iter(|| {
                let result = checker.check_path(temp_dir.path());
                black_box(result)
            })
        });
    }

    group.finish();
}

fn bench_cache_regression(c: &mut Criterion) {
    use dx_check::cache::AstCache;

    let mut group = c.benchmark_group("regression/cache");
    group.measurement_time(Duration::from_secs(5));

    let temp_dir = tempdir().unwrap();
    let cache = AstCache::new(temp_dir.path().to_path_buf(), 100 * 1024 * 1024).unwrap();

    // Benchmark cache operations
    group.bench_function("cache_put", |b| {
        let path = std::path::PathBuf::from("test.js");
        let content = MEDIUM_JS.as_bytes();
        b.iter(|| {
            cache.put(&path, content, vec![]);
            black_box(())
        })
    });

    // Pre-populate cache for get benchmark
    let path = std::path::PathBuf::from("cached.js");
    cache.put(&path, MEDIUM_JS.as_bytes(), vec![]);

    group.bench_function("cache_get_hit", |b| {
        b.iter(|| black_box(cache.get(&path, MEDIUM_JS.as_bytes())))
    });

    group.bench_function("cache_get_miss", |b| {
        let miss_path = std::path::PathBuf::from("not_cached.js");
        b.iter(|| black_box(cache.get(&miss_path, MEDIUM_JS.as_bytes())))
    });

    group.finish();
}

fn bench_fix_application_regression(c: &mut Criterion) {
    use dx_check::diagnostics::{Fix, Span};
    use dx_check::fix::FixEngine;

    let engine = FixEngine::new();

    let mut group = c.benchmark_group("regression/fix");
    group.measurement_time(Duration::from_secs(5));

    // Single fix
    let source = MEDIUM_JS.as_bytes();
    let fix = Fix::replace("Replace", Span::new(0, 5), "const");

    group.bench_function("single_fix", |b| b.iter(|| black_box(engine.apply_fix(source, &fix))));

    // Multiple fixes
    let fixes = vec![
        Fix::replace("Fix 1", Span::new(0, 5), "const"),
        Fix::replace("Fix 2", Span::new(50, 55), "const"),
        Fix::replace("Fix 3", Span::new(100, 105), "const"),
    ];

    group.bench_function("multiple_fixes", |b| {
        b.iter(|| {
            let mut result = source.to_vec();
            for fix in &fixes {
                result = engine.apply_fix(&result, fix);
            }
            black_box(result)
        })
    });

    group.finish();
}

fn bench_diagnostic_creation_regression(c: &mut Criterion) {
    use dx_check::diagnostics::{Diagnostic, DiagnosticBuilder, Span};
    use std::path::PathBuf;

    let mut group = c.benchmark_group("regression/diagnostics");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("create_error", |b| {
        b.iter(|| {
            black_box(Diagnostic::error(
                PathBuf::from("test.js"),
                Span::new(0, 10),
                "no-console",
                "Unexpected console statement",
            ))
        })
    });

    group.bench_function("create_with_builder", |b| {
        b.iter(|| {
            black_box(
                DiagnosticBuilder::error()
                    .file("test.js")
                    .span_range(0, 10)
                    .rule_id("no-console")
                    .message("Unexpected console statement")
                    .build(),
            )
        })
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    name = regression_benches;
    config = Criterion::default()
        .significance_level(0.05)
        .noise_threshold(0.02)
        .warm_up_time(Duration::from_secs(3));
    targets =
        bench_single_file_regression,
        bench_multi_file_regression,
        bench_rule_loading_regression,
        bench_scanner_regression,
        bench_memory_regression,
        bench_cache_regression,
        bench_fix_application_regression,
        bench_diagnostic_creation_regression,
);

criterion_main!(regression_benches);
