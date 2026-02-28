#!/usr/bin/env python3
"""Direct comparison benchmark - same tests as DX-Py bench"""
import time
import sys

def measure(name, func, iterations=10000):
    """Measure with warmup"""
    # Warmup
    for _ in range(100):
        func()
    
    # Measure
    times = []
    for _ in range(iterations):
        start = time.perf_counter_ns()
        func()
        end = time.perf_counter_ns()
        times.append(end - start)
    
    mean = sum(times) / len(times)
    min_t = min(times)
    max_t = max(times)
    ops_per_sec = 1_000_000_000 / mean if mean > 0 else float('inf')
    
    return mean, min_t, max_t, ops_per_sec

def bench_startup():
    """Startup overhead (already running)"""
    pass

def bench_eval_int():
    """Simple integer arithmetic"""
    x = 1 + 2 * 3 - 4
    return x

def bench_builtin_len():
    """Built-in len() call"""
    lst = [1, 2, 3, 4, 5]
    return len(lst)

def bench_list_ops():
    """List operations"""
    lst = []
    for i in range(10):
        lst.append(i)
    lst.sort()
    lst.reverse()
    return lst

def bench_dict_ops():
    """Dictionary operations with 100 keys"""
    d = {}
    for i in range(100):
        d[f"key_{i}"] = i
    for i in range(100):
        _ = d.get(f"key_{i}")
    return d

def bench_string_ops():
    """String operations"""
    s = "hello world"
    s = s.upper()
    s = s.lower()
    _ = s.find("world")
    parts = s.split()
    return " ".join(parts)

def format_time(ns):
    if ns >= 1_000_000:
        return f"{ns/1_000_000:.3f}ms"
    elif ns >= 1_000:
        return f"{ns/1_000:.3f}µs"
    else:
        return f"{ns:.3f}ns"

def main():
    print(f"Running CPython {sys.version.split()[0]} benchmarks...")
    print()
    
    benchmarks = [
        ("startup", bench_startup, 100000),
        ("eval_int", bench_eval_int, 100000),
        ("builtin_len", bench_builtin_len, 100000),
        ("list_ops", bench_list_ops, 10000),
        ("dict_ops", bench_dict_ops, 1000),
        ("string_ops", bench_string_ops, 100000),
    ]
    
    print(f"{'Benchmark':<25} {'Mean':>12} {'Min':>12} {'Max':>12} {'Ops/sec':>12}")
    print("-" * 75)
    
    results = {}
    for name, func, iterations in benchmarks:
        mean, min_t, max_t, ops = measure(name, func, iterations)
        results[name] = {"mean": mean, "min": min_t, "max": max_t, "ops": ops}
        print(f"{name:<25} {format_time(mean):>12} {format_time(min_t):>12} {format_time(max_t):>12} {int(ops):>12}")
    
    return results

if __name__ == "__main__":
    main()
