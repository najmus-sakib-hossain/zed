//! dx-py Performance Benchmarks
//!
//! Comprehensive benchmark suite measuring:
//! - Version resolution with SIMD acceleration
//! - Lock file read/write operations
//! - Cache operations
//! - Package installation simulation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use dx_py_core::version::PackedVersion;
use dx_py_package_manager::{
    cache::GlobalCache,
    formats::dpl::{DplBuilder, DplLockFile},
    resolver::{Dependency, HintCache, InMemoryProvider, Resolver, VersionConstraint},
};
use tempfile::TempDir;

/// Benchmark version resolution with varying numbers of packages
fn bench_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolution");

    for num_packages in [10, 50, 100, 500].iter() {
        // Create a provider with many versions per package
        let mut provider = InMemoryProvider::new();
        for i in 0..*num_packages {
            let name = format!("package-{}", i);
            for v in 1..=100 {
                provider.add_package(&name, &format!("{}.0.0", v), vec![]);
            }
        }

        // Create dependencies requesting latest compatible versions
        let deps: Vec<Dependency> = (0..*num_packages)
            .map(|i| {
                Dependency::new(
                    &format!("package-{}", i),
                    VersionConstraint::Gte(PackedVersion::new(50, 0, 0)),
                )
            })
            .collect();

        group.throughput(Throughput::Elements(*num_packages as u64));
        group.bench_with_input(BenchmarkId::new("packages", num_packages), num_packages, |b, _| {
            b.iter(|| {
                let mut resolver = Resolver::new(provider.clone());
                let result = resolver.resolve(black_box(&deps));
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark SIMD vs scalar version comparison
fn bench_version_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_comparison");

    let candidates: Vec<PackedVersion> =
        (0..1000).map(|i| PackedVersion::new(i / 100, (i / 10) % 10, i % 10)).collect();
    let threshold = PackedVersion::new(5, 0, 0);

    group.throughput(Throughput::Elements(candidates.len() as u64));

    group.bench_function("filter_1000_versions", |b| {
        b.iter(|| {
            let filtered: Vec<_> =
                candidates.iter().filter(|v| **v >= threshold).cloned().collect();
            black_box(filtered)
        });
    });

    group.finish();
}

/// Benchmark lock file operations
fn bench_lockfile(c: &mut Criterion) {
    let mut group = c.benchmark_group("lockfile");

    for num_packages in [10, 100, 500, 1000].iter() {
        let temp_dir = TempDir::new().unwrap();
        let lock_path = temp_dir.path().join("test.dpl");

        // Build a lock file with many packages
        let mut builder = DplBuilder::new("3.12.0", "manylinux_2_17_x86_64");
        for i in 0..*num_packages {
            builder.add_package(&format!("package-{}", i), &format!("1.0.{}", i % 100), [0u8; 32]);
        }
        builder.write_to_file(&lock_path).unwrap();

        // Benchmark reading
        group.throughput(Throughput::Elements(*num_packages as u64));
        group.bench_with_input(BenchmarkId::new("read", num_packages), &lock_path, |b, path| {
            b.iter(|| {
                let lockfile = DplLockFile::open(path).unwrap();
                black_box(lockfile)
            });
        });

        // Benchmark lookup
        let lockfile = DplLockFile::open(&lock_path).unwrap();
        group.bench_with_input(BenchmarkId::new("lookup", num_packages), num_packages, |b, n| {
            b.iter(|| {
                for i in 0..*n {
                    let name = format!("package-{}", i);
                    black_box(lockfile.lookup(&name));
                }
            });
        });
    }

    group.finish();
}

/// Benchmark hint cache operations
fn bench_hint_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("hint_cache");

    let mut cache = HintCache::new();

    // Pre-populate cache with resolutions
    let deps: Vec<Dependency> = (0..100)
        .map(|i| Dependency::new(&format!("pkg-{}", i), VersionConstraint::Any))
        .collect();

    // Compute hash for deps
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for dep in &deps {
        dep.name.hash(&mut hasher);
    }
    let input_hash = hasher.finish();

    let resolved: Vec<_> = (0..100)
        .map(|i| {
            dx_py_package_manager::resolver::ResolvedPackage::new(
                &format!("pkg-{}", i),
                PackedVersion::new(1, 0, 0),
                "1.0.0",
            )
        })
        .collect();

    let resolution = dx_py_package_manager::resolver::Resolution::new(resolved, 10);
    cache.store(input_hash, &resolution);

    group.bench_function("cache_lookup_hit", |b| {
        b.iter(|| {
            let result = cache.lookup(black_box(input_hash));
            black_box(result)
        });
    });

    let unknown_hash = 99999u64;

    group.bench_function("cache_lookup_miss", |b| {
        b.iter(|| {
            let result = cache.lookup(black_box(unknown_hash));
            black_box(result)
        });
    });

    group.finish();
}

/// Benchmark cache operations
fn bench_global_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("global_cache");

    let temp_dir = TempDir::new().unwrap();
    let cache = GlobalCache::new(temp_dir.path()).unwrap();

    // Store some test content
    let content = vec![0u8; 1024 * 1024]; // 1MB
    let hash = *blake3::hash(&content).as_bytes();
    cache.store(&hash, &content).unwrap();

    group.throughput(Throughput::Bytes(content.len() as u64));

    group.bench_function("cache_get_path_1mb", |b| {
        b.iter(|| {
            let path = cache.get_path(black_box(&hash));
            black_box(path)
        });
    });

    group.bench_function("cache_contains", |b| {
        b.iter(|| {
            let exists = cache.contains(black_box(&hash));
            black_box(exists)
        });
    });

    group.finish();
}

/// Benchmark DPL builder (lock file creation)
fn bench_dpl_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("dpl_builder");

    for num_packages in [100, 500, 1000].iter() {
        let temp_dir = TempDir::new().unwrap();

        group.throughput(Throughput::Elements(*num_packages as u64));
        group.bench_with_input(
            BenchmarkId::new("build_and_write", num_packages),
            num_packages,
            |b, n| {
                b.iter(|| {
                    let mut builder = DplBuilder::new("3.12.0", "manylinux_2_17_x86_64");
                    for i in 0..*n {
                        builder.add_package(
                            &format!("package-{}", i),
                            &format!("{}.{}.{}", i / 100, (i / 10) % 10, i % 10),
                            [i as u8; 32],
                        );
                    }
                    let path = temp_dir.path().join(format!("bench-{}.dpl", n));
                    builder.write_to_file(&path).unwrap();
                    black_box(path)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_resolution,
    bench_version_comparison,
    bench_lockfile,
    bench_hint_cache,
    bench_global_cache,
    bench_dpl_builder,
);

criterion_main!(benches);
