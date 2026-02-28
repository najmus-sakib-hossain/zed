//! Core type benchmarks
//!
//! Benchmarks for object creation and method calls.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dx_py_core::pydict::PyKey;
use dx_py_core::pylist::PyValue;
use dx_py_core::{PyDict, PyInt, PyList, PyStr};
use std::sync::Arc;

fn bench_pyint_creation(c: &mut Criterion) {
    c.bench_function("PyInt::new", |b| b.iter(|| black_box(PyInt::new(42))));
}

fn bench_pyint_arithmetic(c: &mut Criterion) {
    let a = PyInt::new(1000);
    let b = PyInt::new(42);

    let mut group = c.benchmark_group("PyInt arithmetic");

    group.bench_function("add", |bench| bench.iter(|| black_box(a.add(&b).unwrap())));

    group.bench_function("mul", |bench| bench.iter(|| black_box(a.mul(&b).unwrap())));

    group.bench_function("floordiv", |bench| bench.iter(|| black_box(a.floordiv(&b).unwrap())));

    group.finish();
}

fn bench_pystr_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("PyStr creation");

    for size in [10, 100, 1000].iter() {
        let s: String = "a".repeat(*size);
        group.bench_with_input(BenchmarkId::new("new", size), &s, |b, s| {
            b.iter(|| black_box(PyStr::new(s.clone())))
        });
    }

    group.finish();
}

fn bench_pystr_operations(c: &mut Criterion) {
    let s = PyStr::new("hello world this is a test string");
    let needle = PyStr::new("test");

    let mut group = c.benchmark_group("PyStr operations");

    group.bench_function("len", |b| b.iter(|| black_box(s.len())));

    group.bench_function("contains", |b| b.iter(|| black_box(s.contains(&needle))));

    group.bench_function("find", |b| b.iter(|| black_box(s.find(&needle))));

    group.bench_function("upper", |b| b.iter(|| black_box(s.upper())));

    group.bench_function("lower", |b| b.iter(|| black_box(s.lower())));

    group.finish();
}

fn bench_pylist_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("PyList creation");

    group.bench_function("new", |b| b.iter(|| black_box(PyList::new())));

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("with_capacity", size), size, |b, &size| {
            b.iter(|| black_box(PyList::with_capacity(size)))
        });
    }

    group.finish();
}

fn bench_pylist_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("PyList operations");

    // Append benchmark
    group.bench_function("append", |b| {
        let list = PyList::new();
        b.iter(|| {
            list.append(PyValue::Int(42));
        })
    });

    // Get benchmark
    let list = PyList::from_values((0..1000).map(PyValue::Int).collect());
    group.bench_function("getitem", |b| b.iter(|| black_box(list.getitem(500).unwrap())));

    // Contains benchmark
    group.bench_function("contains", |b| b.iter(|| black_box(list.contains(&PyValue::Int(500)))));

    group.finish();
}

fn bench_pydict_creation(c: &mut Criterion) {
    c.bench_function("PyDict::new", |b| b.iter(|| black_box(PyDict::new())));
}

fn bench_pydict_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("PyDict operations");

    // Set benchmark
    group.bench_function("setitem", |b| {
        let dict = PyDict::new();
        let mut i = 0i64;
        b.iter(|| {
            dict.setitem(PyKey::Int(i), PyValue::Int(i));
            i += 1;
        })
    });

    // Get benchmark
    let dict = PyDict::new();
    for i in 0..1000 {
        dict.setitem(PyKey::Int(i), PyValue::Int(i));
    }

    group.bench_function("getitem", |b| {
        b.iter(|| black_box(dict.getitem(&PyKey::Int(500)).unwrap()))
    });

    // Contains benchmark
    group.bench_function("contains", |b| b.iter(|| black_box(dict.contains(&PyKey::Int(500)))));

    // String key benchmark
    let dict_str = PyDict::new();
    for i in 0..1000 {
        dict_str.setitem(PyKey::Str(Arc::from(format!("key{}", i))), PyValue::Int(i));
    }

    group.bench_function("getitem_str", |b| {
        let key = PyKey::Str(Arc::from("key500"));
        b.iter(|| black_box(dict_str.getitem(&key).unwrap()))
    });

    group.finish();
}

fn bench_refcount(c: &mut Criterion) {
    use dx_py_core::header::{ObjectFlags, TypeTag};
    use dx_py_core::PyObjectHeader;

    let header = PyObjectHeader::new(TypeTag::Int, ObjectFlags::IMMUTABLE);

    let mut group = c.benchmark_group("RefCount");

    group.bench_function("incref", |b| {
        b.iter(|| {
            header.incref();
        })
    });

    group.bench_function("decref", |b| {
        // First incref to avoid dropping to 0
        header.incref();
        b.iter(|| {
            header.incref();
            black_box(header.decref());
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_pyint_creation,
    bench_pyint_arithmetic,
    bench_pystr_creation,
    bench_pystr_operations,
    bench_pylist_creation,
    bench_pylist_operations,
    bench_pydict_creation,
    bench_pydict_operations,
    bench_refcount,
);

criterion_main!(benches);
