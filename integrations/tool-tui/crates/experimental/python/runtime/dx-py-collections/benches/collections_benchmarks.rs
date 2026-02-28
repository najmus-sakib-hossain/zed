//! Benchmarks for dx-py-collections
//!
//! Run with: cargo bench -p dx-py-collections

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dx_py_collections::{SimdList, SwissDict};

fn bench_simd_list_sum(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_list_sum");

    for size in [100, 1000, 10000, 100000].iter() {
        let values: Vec<i64> = (0..*size).collect();
        let list = SimdList::from_ints(values);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(list.sum()))
        });
    }

    group.finish();
}

fn bench_simd_list_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_list_filter");

    for size in [100, 1000, 10000].iter() {
        let values: Vec<i64> = (0..*size).collect();
        let list = SimdList::from_ints(values);
        let threshold = *size / 2;

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(list.filter_gt_int(threshold)))
        });
    }

    group.finish();
}

fn bench_simd_list_map(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_list_map");

    for size in [100, 1000, 10000].iter() {
        let values: Vec<i64> = (0..*size).collect();
        let list = SimdList::from_ints(values);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(list.map_mul2_int()))
        });
    }

    group.finish();
}

fn bench_simd_list_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_list_index");

    for size in [100, 1000, 10000].iter() {
        let values: Vec<i64> = (0..*size).collect();
        let list = SimdList::from_ints(values);
        let target = *size / 2;

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(list.index_int(target)))
        });
    }

    group.finish();
}

fn bench_simd_list_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_list_count");

    for size in [100, 1000, 10000].iter() {
        // Create list with some repeated values
        let values: Vec<i64> = (0..*size).map(|i| i % 100).collect();
        let list = SimdList::from_ints(values);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| black_box(list.count_int(50)))
        });
    }

    group.finish();
}

fn bench_swiss_dict_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("swiss_dict_insert");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut dict = SwissDict::new();
                for i in 0..size {
                    dict.insert(i, i * 2);
                }
                black_box(dict)
            })
        });
    }

    group.finish();
}

fn bench_swiss_dict_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("swiss_dict_get");

    for size in [100, 1000, 10000].iter() {
        let mut dict = SwissDict::new();
        for i in 0..*size {
            dict.insert(i, i * 2);
        }

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                for i in 0..size {
                    black_box(dict.get(&i));
                }
            })
        });
    }

    group.finish();
}

fn bench_swiss_dict_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("swiss_dict_contains");

    for size in [100, 1000, 10000].iter() {
        let mut dict = SwissDict::new();
        for i in 0..*size {
            dict.insert(i, i * 2);
        }

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                for i in 0..size {
                    black_box(dict.contains_key(&i));
                }
            })
        });
    }

    group.finish();
}

fn bench_swiss_dict_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("swiss_dict_remove");

    for size in [100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut dict = SwissDict::new();
                    for i in 0..size {
                        dict.insert(i, i * 2);
                    }
                    dict
                },
                |mut dict| {
                    for i in 0..size {
                        black_box(dict.remove(&i));
                    }
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_simd_list_sum,
    bench_simd_list_filter,
    bench_simd_list_map,
    bench_simd_list_index,
    bench_simd_list_count,
    bench_swiss_dict_insert,
    bench_swiss_dict_get,
    bench_swiss_dict_contains,
    bench_swiss_dict_remove,
);

criterion_main!(benches);
