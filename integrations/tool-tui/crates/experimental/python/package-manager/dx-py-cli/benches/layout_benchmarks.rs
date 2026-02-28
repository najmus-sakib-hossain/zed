//! Performance benchmarks for Layout Cache and Package Store
//!
//! These benchmarks verify the performance targets:
//! - Warm install: <10ms
//! - DPL lookup: <0.01ms
//! - Package store access: <1ms

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::Arc;
use tempfile::TempDir;

use dx_py_layout::{LayoutCache, ResolvedPackage};
use dx_py_package_manager::{DplBuilder, DplLockFile};
use dx_py_store::PackageStore;

fn create_test_packages(count: usize) -> Vec<ResolvedPackage> {
    (0..count)
        .map(|i| {
            let mut hash = [0u8; 32];
            hash[0] = (i % 256) as u8;
            hash[1] = ((i / 256) % 256) as u8;
            ResolvedPackage {
                name: format!("package-{}", i),
                version: format!("{}.{}.{}", i % 10, (i / 10) % 10, i % 100),
                hash,
            }
        })
        .collect()
}

fn setup_store_with_packages(temp: &TempDir, packages: &[ResolvedPackage]) -> Arc<PackageStore> {
    let store = Arc::new(PackageStore::open(temp.path().join("store")).unwrap());

    for pkg in packages {
        let files = [
            (format!("{}/__init__.py", pkg.name), b"# init".to_vec()),
            (format!("{}/module.py", pkg.name), b"def hello(): pass".to_vec()),
        ];
        let file_refs: Vec<(&str, &[u8])> =
            files.iter().map(|(p, d)| (p.as_str(), d.as_slice())).collect();
        let _ = store.store_package(&pkg.hash, &file_refs);
    }

    store
}

fn bench_project_hash_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("project_hash");

    for size in [10, 50, 100, 500].iter() {
        let packages = create_test_packages(*size);

        group.bench_with_input(BenchmarkId::new("compute_hash", size), &packages, |b, pkgs| {
            b.iter(|| black_box(LayoutCache::compute_project_hash(pkgs)));
        });
    }

    group.finish();
}

fn bench_layout_cache_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout_cache_lookup");

    for size in [10, 50, 100].iter() {
        let temp = TempDir::new().unwrap();
        let packages = create_test_packages(*size);
        let store = setup_store_with_packages(&temp, &packages);

        let mut cache = LayoutCache::open(temp.path().join("layouts"), Arc::clone(&store)).unwrap();
        let project_hash = LayoutCache::compute_project_hash(&packages);

        // Build the layout first
        cache.build_layout(&project_hash, &packages).unwrap();

        group.bench_with_input(BenchmarkId::new("lookup", size), &project_hash, |b, hash| {
            b.iter(|| black_box(cache.get(hash)));
        });
    }

    group.finish();
}

fn bench_dpl_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("dpl_lookup");

    for size in [10, 50, 100, 500, 1000].iter() {
        let mut builder = DplBuilder::new("3.12.0", "any");

        for i in 0..*size {
            let mut hash = [0u8; 32];
            hash[0] = (i % 256) as u8;
            builder.add_package(&format!("package-{}", i), &format!("{}.0.0", i), hash);
        }

        let data = builder.build();
        let lock_file = DplLockFile::from_bytes(data).unwrap();

        // Lookup middle package
        let lookup_name = format!("package-{}", size / 2);

        group.bench_with_input(BenchmarkId::new("lookup", size), &lookup_name, |b, name| {
            b.iter(|| black_box(lock_file.lookup(name)));
        });
    }

    group.finish();
}

fn bench_package_store_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_store");

    let temp = TempDir::new().unwrap();
    let store = PackageStore::open(temp.path()).unwrap();

    // Store a test package
    let hash = [42u8; 32];
    let files = vec![
        ("test/__init__.py", b"# init" as &[u8]),
        ("test/module.py", b"def hello(): pass"),
    ];
    store.store_package(&hash, &files).unwrap();

    group.bench_function("contains", |b| {
        b.iter(|| black_box(store.contains(&hash)));
    });

    group.bench_function("get_path", |b| {
        b.iter(|| black_box(store.get_path(&hash)));
    });

    group.bench_function("get_file", |b| {
        b.iter(|| black_box(store.get_file(&hash, "test/__init__.py")));
    });

    group.finish();
}

fn bench_warm_install_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("warm_install");
    group.sample_size(50); // Reduce sample size for slower benchmarks

    for size in [10, 50, 100].iter() {
        let temp = TempDir::new().unwrap();
        let packages = create_test_packages(*size);
        let store = setup_store_with_packages(&temp, &packages);

        let mut cache = LayoutCache::open(temp.path().join("layouts"), Arc::clone(&store)).unwrap();
        let project_hash = LayoutCache::compute_project_hash(&packages);

        // Build the layout first
        cache.build_layout(&project_hash, &packages).unwrap();

        group.bench_with_input(
            BenchmarkId::new("install_cached", size),
            &project_hash,
            |b, hash| {
                b.iter_with_setup(
                    || {
                        let target = temp.path().join(format!("venv_{}", rand::random::<u32>()));
                        std::fs::create_dir_all(&target).unwrap();
                        target
                    },
                    |target| black_box(cache.install_cached(hash, &target)),
                );
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_project_hash_computation,
    bench_layout_cache_lookup,
    bench_dpl_lookup,
    bench_package_store_access,
    bench_warm_install_simulation,
);

criterion_main!(benches);
