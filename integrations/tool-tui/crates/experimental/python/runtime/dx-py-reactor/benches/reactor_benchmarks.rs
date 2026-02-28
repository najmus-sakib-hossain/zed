//! Benchmarks for dx-py-reactor async I/O performance
//!
//! Run with: cargo bench -p dx-py-reactor

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;

use dx_py_reactor::{
    create_basic_reactor, CompletionHandler, IoBuffer, IoOperation, PyFuture, ReactorFeature,
};

/// Benchmark reactor creation
fn bench_reactor_creation(c: &mut Criterion) {
    c.bench_function("reactor_creation", |b| {
        b.iter(|| {
            let reactor = create_basic_reactor().unwrap();
            black_box(reactor)
        })
    });
}

/// Benchmark NOP operation submission (measures submission overhead)
fn bench_nop_submission(c: &mut Criterion) {
    let mut reactor = create_basic_reactor().unwrap();

    c.bench_function("nop_submit", |b| {
        b.iter(|| {
            let op = IoOperation::Nop { user_data: 1 };
            let _ = reactor.submit(black_box(op));
        })
    });
}

/// Benchmark batch NOP submission
fn bench_batch_nop_submission(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_nop_submit");

    for batch_size in [1, 10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(batch_size), batch_size, |b, &size| {
            let mut reactor = create_basic_reactor().unwrap();
            b.iter(|| {
                let ops: Vec<IoOperation> = (0..size)
                    .map(|i| IoOperation::Nop {
                        user_data: i as u64,
                    })
                    .collect();
                let _ = reactor.submit_batch(black_box(ops));
            })
        });
    }

    group.finish();
}

/// Benchmark IoBuffer creation
fn bench_io_buffer_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("io_buffer_creation");

    for size in [1024, 4096, 16384, 65536].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let buf = IoBuffer::new(size);
                black_box(buf)
            })
        });
    }

    group.finish();
}

/// Benchmark IoBuffer from_vec (zero-copy)
fn bench_io_buffer_from_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("io_buffer_from_vec");

    for size in [1024, 4096, 16384, 65536].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let data = vec![0u8; size];
                let buf = IoBuffer::from_vec(data);
                black_box(buf)
            })
        });
    }

    group.finish();
}

/// Benchmark PyFuture creation and resolution
fn bench_py_future(c: &mut Criterion) {
    c.bench_function("py_future_create", |b| {
        b.iter(|| {
            let future: PyFuture<i32> = PyFuture::new();
            black_box(future)
        })
    });

    c.bench_function("py_future_set_result", |b| {
        b.iter(|| {
            let future: PyFuture<i32> = PyFuture::new();
            future.set_result(42);
            black_box(future.try_get())
        })
    });

    c.bench_function("py_future_clone_and_resolve", |b| {
        b.iter(|| {
            let future: PyFuture<i32> = PyFuture::new();
            let clone = future.clone();
            future.set_result(42);
            black_box(clone.try_get())
        })
    });
}

/// Benchmark CompletionHandler
fn bench_completion_handler(c: &mut Criterion) {
    use dx_py_reactor::Completion;

    c.bench_function("completion_handler_register", |b| {
        b.iter(|| {
            let mut handler: CompletionHandler<usize> = CompletionHandler::new();
            for i in 0..100 {
                handler.register(i, PyFuture::new());
            }
            black_box(handler.pending_count())
        })
    });

    c.bench_function("completion_handler_process", |b| {
        b.iter(|| {
            let mut handler: CompletionHandler<usize> = CompletionHandler::new();
            for i in 0..100 {
                handler.register(i, PyFuture::new());
            }
            for i in 0..100 {
                let completion = Completion::success(i, 1024);
                handler.process(&completion, |c| Ok(c.bytes()));
            }
            black_box(handler.pending_count())
        })
    });
}

/// Benchmark reactor poll (non-blocking)
fn bench_reactor_poll(c: &mut Criterion) {
    let mut reactor = create_basic_reactor().unwrap();

    c.bench_function("reactor_poll_empty", |b| {
        b.iter(|| {
            let completions = reactor.poll();
            black_box(completions)
        })
    });
}

/// Benchmark reactor wait with timeout
fn bench_reactor_wait(c: &mut Criterion) {
    let mut reactor = create_basic_reactor().unwrap();

    c.bench_function("reactor_wait_1ms", |b| {
        b.iter(|| {
            let completions = reactor.wait(Duration::from_millis(1));
            black_box(completions)
        })
    });
}

/// Benchmark feature checking
fn bench_feature_check(c: &mut Criterion) {
    let reactor = create_basic_reactor().unwrap();

    c.bench_function("feature_check", |b| {
        b.iter(|| {
            let _ = reactor.supports(ReactorFeature::Timeouts);
            let _ = reactor.supports(ReactorFeature::MultishotAccept);
            let _ = reactor.supports(ReactorFeature::ZeroCopySend);
            black_box(reactor.supports(ReactorFeature::Cancellation))
        })
    });
}

criterion_group!(
    benches,
    bench_reactor_creation,
    bench_nop_submission,
    bench_batch_nop_submission,
    bench_io_buffer_creation,
    bench_io_buffer_from_vec,
    bench_py_future,
    bench_completion_handler,
    bench_reactor_poll,
    bench_reactor_wait,
    bench_feature_check,
);

criterion_main!(benches);
