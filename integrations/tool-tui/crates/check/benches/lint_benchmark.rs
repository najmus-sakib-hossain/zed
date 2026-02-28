//! Benchmarks for dx-check
//!
//! Run with: cargo bench
//!
//! **Feature: dx-check-production, Task 15.4**
//! **Validates: Requirements 9.5**

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use dx_check::config::CheckerConfig;
use dx_check::engine::Checker;
use dx_check::rules::RuleRegistry;
use dx_check::scanner::PatternScanner;
use std::path::Path;
use tempfile::tempdir;

/// Sample JavaScript code for benchmarking
const SAMPLE_JS: &str = r#"
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

/// Sample code with issues
const SAMPLE_WITH_ISSUES: &str = r#"
// Bad code for testing
var x = 1;
console.log(x);
debugger;
if (x == 1) {
    eval('alert("hello")');
}
"#;

/// Sample TypeScript code
const SAMPLE_TS: &str = r#"
interface User {
    id: number;
    name: string;
    email: string;
}

async function fetchUser(id: number): Promise<User> {
    const response = await fetch(`/api/users/${id}`);
    return response.json();
}

export class UserService {
    private cache: Map<number, User> = new Map();

    async getUser(id: number): Promise<User> {
        if (this.cache.has(id)) {
            return this.cache.get(id)!;
        }
        const user = await fetchUser(id);
        this.cache.set(id, user);
        return user;
    }
}
"#;

/// Sample JSX code
const SAMPLE_JSX: &str = r#"
import React from 'react';

function Button({ onClick, children, disabled }) {
    return (
        <button
            className="btn btn-primary"
            onClick={onClick}
            disabled={disabled}
            aria-label={typeof children === 'string' ? children : 'button'}
        >
            {children}
        </button>
    );
}

export function App() {
    const [count, setCount] = React.useState(0);
    
    return (
        <div className="app">
            <h1>Counter: {count}</h1>
            <Button onClick={() => setCount(c => c + 1)}>
                Increment
            </Button>
            <Button onClick={() => setCount(c => c - 1)}>
                Decrement
            </Button>
        </div>
    );
}
"#;

fn bench_parse_and_lint(c: &mut Criterion) {
    let checker = Checker::new(CheckerConfig::default());

    c.bench_function("parse_and_lint_simple", |b| {
        b.iter(|| black_box(checker.check_source(Path::new("test.js"), SAMPLE_JS)))
    });

    c.bench_function("parse_and_lint_with_issues", |b| {
        b.iter(|| black_box(checker.check_source(Path::new("test.js"), SAMPLE_WITH_ISSUES)))
    });
}

fn bench_simd_scanner(c: &mut Criterion) {
    let scanner = PatternScanner::new();

    // Generate larger samples
    let large_sample = SAMPLE_JS.repeat(100);

    let mut group = c.benchmark_group("simd_scanner");

    group.throughput(Throughput::Bytes(large_sample.len() as u64));

    group.bench_function("scan_large_file", |b| {
        b.iter(|| black_box(scanner.scan(large_sample.as_bytes())))
    });

    group.bench_function("has_any_match_clean", |b| {
        b.iter(|| black_box(scanner.has_any_match(SAMPLE_JS.as_bytes())))
    });

    group.bench_function("has_any_match_with_issues", |b| {
        b.iter(|| black_box(scanner.has_any_match(SAMPLE_WITH_ISSUES.as_bytes())))
    });

    group.finish();
}

fn bench_scaling(c: &mut Criterion) {
    let checker = Checker::new(CheckerConfig::default());

    let mut group = c.benchmark_group("scaling");

    for size in [1, 10, 100, 1000].iter() {
        let code = SAMPLE_JS.repeat(*size);
        group.throughput(Throughput::Bytes(code.len() as u64));

        group.bench_with_input(BenchmarkId::new("lint", size), &code, |b, code| {
            b.iter(|| black_box(checker.check_source(Path::new("test.js"), code)))
        });
    }

    group.finish();
}

/// Benchmark single file check time for different file types
fn bench_single_file_check(c: &mut Criterion) {
    let checker = Checker::new(CheckerConfig::default());

    let mut group = c.benchmark_group("single_file_check");

    // JavaScript
    group.throughput(Throughput::Bytes(SAMPLE_JS.len() as u64));
    group.bench_function("javascript", |b| {
        b.iter(|| black_box(checker.check_source(Path::new("test.js"), SAMPLE_JS)))
    });

    // TypeScript
    group.throughput(Throughput::Bytes(SAMPLE_TS.len() as u64));
    group.bench_function("typescript", |b| {
        b.iter(|| black_box(checker.check_source(Path::new("test.ts"), SAMPLE_TS)))
    });

    // JSX
    group.throughput(Throughput::Bytes(SAMPLE_JSX.len() as u64));
    group.bench_function("jsx", |b| {
        b.iter(|| black_box(checker.check_source(Path::new("test.jsx"), SAMPLE_JSX)))
    });

    group.finish();
}

/// Benchmark multi-file check time
fn bench_multi_file_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_file_check");

    for file_count in [5, 10, 25, 50].iter() {
        let temp_dir = tempdir().unwrap();

        // Create test files
        for i in 0..*file_count {
            let filename = format!("file_{}.js", i);
            let content = if i % 3 == 0 {
                SAMPLE_WITH_ISSUES
            } else {
                SAMPLE_JS
            };
            std::fs::write(temp_dir.path().join(&filename), content).unwrap();
        }

        let checker = Checker::with_auto_detect(temp_dir.path());

        group.bench_with_input(BenchmarkId::new("files", file_count), file_count, |b, _| {
            b.iter(|| black_box(checker.check_path(temp_dir.path())))
        });
    }

    group.finish();
}

/// Benchmark rule loading time
fn bench_rule_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("rule_loading");

    // Benchmark creating a new rule registry
    group.bench_function("create_registry", |b| b.iter(|| black_box(RuleRegistry::new())));

    // Benchmark creating a checker (includes rule loading)
    group.bench_function("create_checker", |b| {
        b.iter(|| black_box(Checker::new(CheckerConfig::default())))
    });

    group.finish();
}

/// Benchmark configuration parsing
fn bench_config_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_parsing");

    // Benchmark default config creation
    group.bench_function("default_config", |b| b.iter(|| black_box(CheckerConfig::default())));

    // Benchmark TOML parsing
    let toml_config = r#"
[check]
enabled = true
include = ["**/*.ts", "**/*.js"]
exclude = ["**/node_modules/**"]

[check.rules]
no-debugger = "error"
no-console = "warn"

[check.format]
indent_width = 2
line_width = 100
"#;

    group.bench_function("parse_toml", |b| {
        b.iter(|| black_box(toml::from_str::<CheckerConfig>(toml_config)))
    });

    group.finish();
}

/// Benchmark diagnostic creation
fn bench_diagnostics(c: &mut Criterion) {
    use dx_check::diagnostics::{Diagnostic, DiagnosticBuilder, Fix, Span};
    use std::path::PathBuf;

    let mut group = c.benchmark_group("diagnostics");

    // Benchmark direct diagnostic creation
    group.bench_function("create_diagnostic", |b| {
        b.iter(|| {
            black_box(Diagnostic::error(
                PathBuf::from("test.js"),
                Span::new(0, 10),
                "no-console",
                "Unexpected console statement",
            ))
        })
    });

    // Benchmark builder pattern
    group.bench_function("builder_pattern", |b| {
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

    // Benchmark diagnostic with fix
    group.bench_function("diagnostic_with_fix", |b| {
        b.iter(|| {
            black_box(
                Diagnostic::warn(
                    PathBuf::from("test.js"),
                    Span::new(0, 9),
                    "no-debugger",
                    "Unexpected debugger statement",
                )
                .with_fix(Fix::delete("Remove debugger", Span::new(0, 10))),
            )
        })
    });

    group.finish();
}

/// Benchmark fix application
fn bench_fix_application(c: &mut Criterion) {
    use dx_check::diagnostics::{Fix, Span};
    use dx_check::fix::FixEngine;

    let engine = FixEngine::new();
    let source = b"var x = 1; var y = 2; var z = 3;";

    let mut group = c.benchmark_group("fix_application");

    // Single fix
    let single_fix = Fix::replace("Replace var with let", Span::new(0, 3), "let");
    group.bench_function("single_fix", |b| {
        b.iter(|| black_box(engine.apply_fix(source, &single_fix)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_and_lint,
    bench_simd_scanner,
    bench_scaling,
    bench_single_file_check,
    bench_multi_file_check,
    bench_rule_loading,
    bench_config_parsing,
    bench_diagnostics,
    bench_fix_application,
);

criterion_main!(benches);
