//! Benchmarks for dx-py-jit
//!
//! Run with: cargo bench -p dx-py-jit

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dx_py_jit::{CompilationTier, FunctionId, FunctionProfile, PyType, TieredJit, TypeFeedback};
use std::sync::Arc;

fn bench_profile_record_call(c: &mut Criterion) {
    let mut group = c.benchmark_group("profile_record_call");

    group.bench_function("single_thread", |b| {
        let profile = FunctionProfile::new(100, 10);
        b.iter(|| {
            profile.record_call();
            black_box(profile.get_call_count())
        })
    });

    group.finish();
}

fn bench_profile_record_type(c: &mut Criterion) {
    let mut group = c.benchmark_group("profile_record_type");

    group.bench_function("monomorphic", |b| {
        let profile = FunctionProfile::new(100, 10);
        b.iter(|| {
            profile.record_type(50, PyType::Int);
        })
    });

    group.bench_function("polymorphic", |b| {
        let profile = FunctionProfile::new(100, 10);
        let types = [PyType::Int, PyType::Float, PyType::Str];
        let mut i = 0;
        b.iter(|| {
            profile.record_type(50, types[i % 3]);
            i += 1;
        })
    });

    group.finish();
}

fn bench_profile_branch(c: &mut Criterion) {
    let mut group = c.benchmark_group("profile_branch");

    group.bench_function("record_taken", |b| {
        let profile = FunctionProfile::new(100, 10);
        b.iter(|| {
            profile.record_branch_taken(5);
        })
    });

    group.bench_function("get_probability", |b| {
        let profile = FunctionProfile::new(100, 10);
        for _ in 0..1000 {
            profile.record_branch_taken(5);
        }
        for _ in 0..500 {
            profile.record_branch_not_taken(5);
        }

        b.iter(|| black_box(profile.get_branch_probability(5)))
    });

    group.finish();
}

fn bench_type_feedback(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_feedback");

    group.bench_function("record_new_type", |b| {
        b.iter_batched(
            TypeFeedback::new,
            |feedback| {
                feedback.record(PyType::Int);
                black_box(feedback)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("is_monomorphic", |b| {
        let feedback = TypeFeedback::new();
        feedback.record(PyType::Int);

        b.iter(|| black_box(feedback.is_monomorphic()))
    });

    group.bench_function("get_types", |b| {
        let feedback = TypeFeedback::new();
        feedback.record(PyType::Int);
        feedback.record(PyType::Float);

        b.iter(|| black_box(feedback.get_types()))
    });

    group.finish();
}

fn bench_jit_get_profile(c: &mut Criterion) {
    let mut group = c.benchmark_group("jit_get_profile");

    group.bench_function("new_profile", |b| {
        let jit = TieredJit::new();
        let mut id = 0u64;

        b.iter(|| {
            id += 1;
            black_box(jit.get_profile(FunctionId(id), 100, 10))
        })
    });

    group.bench_function("existing_profile", |b| {
        let jit = TieredJit::new();
        let func_id = FunctionId(1);
        jit.get_profile(func_id, 100, 10);

        b.iter(|| black_box(jit.get_profile(func_id, 100, 10)))
    });

    group.finish();
}

fn bench_jit_check_promotion(c: &mut Criterion) {
    let mut group = c.benchmark_group("jit_check_promotion");

    group.bench_function("no_promotion", |b| {
        let jit = TieredJit::new();
        let func_id = FunctionId(1);
        let profile = jit.get_profile(func_id, 100, 10);

        for _ in 0..50 {
            profile.record_call();
        }

        b.iter(|| black_box(jit.check_promotion(func_id)))
    });

    group.bench_function("with_promotion", |b| {
        let jit = TieredJit::new();
        let func_id = FunctionId(1);
        let profile = jit.get_profile(func_id, 100, 10);

        for _ in 0..100 {
            profile.record_call();
        }

        b.iter(|| black_box(jit.check_promotion(func_id)))
    });

    group.finish();
}

fn bench_tier_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("tier_operations");

    group.bench_function("threshold", |b| {
        b.iter(|| black_box(CompilationTier::OptimizingJit.threshold()))
    });

    group.bench_function("next", |b| b.iter(|| black_box(CompilationTier::BaselineJit.next())));

    group.bench_function("is_jit", |b| {
        b.iter(|| black_box(CompilationTier::OptimizingJit.is_jit()))
    });

    group.finish();
}

fn bench_concurrent_profiling(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_profiling");

    for num_threads in [2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_threads),
            num_threads,
            |b, &num_threads| {
                #[allow(clippy::arc_with_non_send_sync)]
                let jit = Arc::new(TieredJit::new());
                let func_id = FunctionId(1);
                let profile = jit.get_profile(func_id, 100, 10);

                b.iter(|| {
                    let handles: Vec<_> = (0..num_threads)
                        .map(|_| {
                            let profile = Arc::clone(&profile);
                            std::thread::spawn(move || {
                                for _ in 0..100 {
                                    profile.record_call();
                                    profile.record_type(50, PyType::Int);
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

criterion_group!(
    benches,
    bench_profile_record_call,
    bench_profile_record_type,
    bench_profile_branch,
    bench_type_feedback,
    bench_jit_get_profile,
    bench_jit_check_promotion,
    bench_tier_operations,
    bench_concurrent_profiling,
);

criterion_main!(benches);
