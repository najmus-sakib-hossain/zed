"""Benchmark CPython for comparison with dx-py-runtime"""
import time
import sys

def measure(name, func, iterations=10000):
    """Measure execution time of a function"""
    # Warmup
    for _ in range(100):
        func()
    
    # Measure
    start = time.perf_counter_ns()
    for _ in range(iterations):
        func()
    end = time.perf_counter_ns()
    
    mean_ns = (end - start) / iterations
    return name, mean_ns

def bench_startup():
    """Measure startup overhead (already started, so minimal)"""
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
    """Dictionary operations"""
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

def main():
    print(f"CPython {sys.version.split()[0]} Benchmarks")
    print("=" * 60)
    print()
    
    results = []
    
    # Run benchmarks
    results.append(measure("startup", bench_startup, 100000))
    results.append(measure("eval_int", bench_eval_int, 100000))
    results.append(measure("builtin_len", bench_builtin_len, 100000))
    results.append(measure("list_ops", bench_list_ops, 10000))
    results.append(measure("dict_ops", bench_dict_ops, 1000))
    results.append(measure("string_ops", bench_string_ops, 100000))
    
    print(f"{'Benchmark':<20} {'Mean':>15} {'Ops/sec':>15}")
    print("-" * 60)
    
    for name, mean_ns in results:
        if mean_ns > 0:
            ops_per_sec = 1_000_000_000 / mean_ns
            if mean_ns >= 1_000_000:
                mean_str = f"{mean_ns/1_000_000:.3f}ms"
            elif mean_ns >= 1_000:
                mean_str = f"{mean_ns/1_000:.3f}µs"
            else:
                mean_str = f"{mean_ns:.0f}ns"
            print(f"{name:<20} {mean_str:>15} {ops_per_sec:>15,.0f}")
        else:
            print(f"{name:<20} {'<1ns':>15} {'∞':>15}")
    
    print()
    return results

if __name__ == "__main__":
    main()
