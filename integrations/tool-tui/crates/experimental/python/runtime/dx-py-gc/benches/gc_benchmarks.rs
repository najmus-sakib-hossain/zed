//! Benchmarks for dx-py-gc
//!
//! Run with: cargo bench -p dx-py-gc

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dx_py_gc::{CycleDetector, EpochGc, LockFreeRefCount};
use std::sync::Arc;

fn bench_refcount_inc_dec(c: &mut Criterion) {
    let mut group = c.benchmark_group("refcount_inc_dec");

    group.bench_function("single_thread", |b| {
        let rc = LockFreeRefCount::new();
        b.iter(|| {
            rc.inc_strong();
            black_box(rc.dec_strong());
        })
    });

    group.finish();
}

fn bench_refcount_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("refcount_concurrent");

    for num_threads in [2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_threads),
            num_threads,
            |b, &num_threads| {
                let rc = Arc::new(LockFreeRefCount::new());
                b.iter(|| {
                    let handles: Vec<_> = (0..num_threads)
                        .map(|_| {
                            let rc = Arc::clone(&rc);
                            std::thread::spawn(move || {
                                for _ in 0..100 {
                                    rc.inc_strong();
                                    rc.dec_strong();
                                }
                            })
                        })
                        .collect();

                    for h in handles {
                        h.join().unwrap();
                    }
                })
            },
        );
    }

    group.finish();
}

fn bench_epoch_enter_exit(c: &mut Criterion) {
    let mut group = c.benchmark_group("epoch_enter_exit");

    group.bench_function("single_thread", |b| {
        let gc = EpochGc::new(4);
        let thread_id = gc.register_thread().unwrap();

        b.iter(|| {
            let epoch = gc.enter_epoch(thread_id);
            black_box(epoch);
            gc.exit_epoch(thread_id);
        });

        gc.unregister_thread(thread_id);
    });

    group.finish();
}

fn bench_epoch_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("epoch_concurrent");

    for num_threads in [2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_threads),
            num_threads,
            |b, &num_threads| {
                let gc = Arc::new(EpochGc::new(16));

                b.iter(|| {
                    let handles: Vec<_> = (0..num_threads)
                        .map(|_| {
                            let gc = Arc::clone(&gc);
                            std::thread::spawn(move || {
                                let thread_id = gc.register_thread().unwrap();
                                for _ in 0..100 {
                                    let epoch = gc.enter_epoch(thread_id);
                                    black_box(epoch);
                                    gc.exit_epoch(thread_id);
                                }
                                gc.unregister_thread(thread_id);
                            })
                        })
                        .collect();

                    for h in handles {
                        h.join().unwrap();
                    }
                })
            },
        );
    }

    group.finish();
}

fn bench_allocation_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_rate");

    for size in [64, 256, 1024, 4096].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let data: Vec<u8> = vec![0u8; size];
                black_box(data);
            })
        });
    }

    group.finish();
}

fn bench_defer_free(c: &mut Criterion) {
    let mut group = c.benchmark_group("defer_free");

    group.bench_function("single_object", |b| {
        let gc = EpochGc::new(4);
        let thread_id = gc.register_thread().unwrap();

        b.iter(|| {
            let obj = Box::into_raw(Box::new(42u64));
            unsafe { gc.defer_free(obj) };
        });

        // Cleanup
        unsafe { gc.force_collect_all() };
        gc.unregister_thread(thread_id);
    });

    group.finish();
}

fn bench_try_collect(c: &mut Criterion) {
    let mut group = c.benchmark_group("try_collect");

    for num_objects in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_objects),
            num_objects,
            |b, &num_objects| {
                b.iter_batched(
                    || {
                        let gc = EpochGc::new(4);
                        let thread_id = gc.register_thread().unwrap();

                        // Add objects to garbage list
                        for _ in 0..num_objects {
                            let obj = Box::into_raw(Box::new(42u64));
                            unsafe { gc.defer_free(obj) };
                        }

                        gc.exit_epoch(thread_id);
                        (gc, thread_id)
                    },
                    |(gc, thread_id)| {
                        // Advance epochs to trigger collection
                        for _ in 0..3 {
                            black_box(gc.try_collect());
                        }
                        gc.unregister_thread(thread_id);
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

fn bench_cycle_detector_creation(c: &mut Criterion) {
    c.bench_function("cycle_detector_creation", |b| b.iter(|| black_box(CycleDetector::new())));
}

fn bench_cycle_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("cycle_detection");

    for num_workers in [1, 2, 4].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_workers),
            num_workers,
            |b, &num_workers| {
                let detector = CycleDetector::new();

                b.iter(|| black_box(detector.detect_cycles(num_workers)))
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_refcount_inc_dec,
    bench_refcount_concurrent,
    bench_epoch_enter_exit,
    bench_epoch_concurrent,
    bench_allocation_rate,
    bench_defer_free,
    bench_try_collect,
    bench_cycle_detector_creation,
    bench_cycle_detection,
);

criterion_main!(benches);
