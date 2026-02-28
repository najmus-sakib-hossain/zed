#!/usr/bin/env python3
"""Fresh DX-Py vs CPython Benchmark - Real Measurements Only"""
import time
import statistics
import json
import sys

def measure(name, func, iterations=1000, warmup=100):
    """Measure execution time with warmup"""
    # Warmup
    for _ in range(warmup):
        func()
    
    # Measure
    times = []
    for _ in range(iterations):
        start = time.perf_counter_ns()
        func()
        end = time.perf_counter_ns()
        times.append(end - start)
    
    mean = statistics.mean(times)
    std = statistics.stdev(times) if len(times) > 1 else 0
    return mean, std

# ============================================================================
# Benchmark Functions
# ============================================================================

def bench_int_arithmetic():
    """Integer arithmetic - 10000 operations"""
    result = 0
    for i in range(10000):
        result += i * 2 - i // 3 + i % 7
        result = result ^ (i << 2)
        result = result & 0xFFFFFFFF
    return result

def bench_string_ops():
    """String operations"""
    s = "hello world " * 100
    result = s.upper()
    result = result.lower()
    result = result.replace("world", "python")
    parts = result.split()
    result = "-".join(parts)
    return result

def bench_list_ops():
    """List operations"""
    lst = list(range(1000))
    lst.reverse()
    lst.sort()
    lst.append(1001)
    lst.insert(500, 999)
    lst.pop()
    lst.remove(500)
    result = [x * 2 for x in lst if x % 2 == 0]
    return result

def bench_dict_ops():
    """Dictionary operations - 1000 keys"""
    d = {str(i): i * 2 for i in range(1000)}
    for i in range(100):
        d[f"new_{i}"] = i * 3
    keys = list(d.keys())
    values = list(d.values())
    items = list(d.items())
    result = {k: v for k, v in d.items() if v % 2 == 0}
    return len(result)

def bench_function_calls():
    """Function call overhead"""
    def add(a, b):
        return a + b
    
    result = 0
    for i in range(10000):
        result = add(result, i)
    return result

def bench_object_creation():
    """Object creation"""
    class Point:
        def __init__(self, x, y):
            self.x = x
            self.y = y
    
    points = []
    for i in range(1000):
        points.append(Point(i, i * 2))
    return len(points)

def bench_list_comprehension():
    """List comprehension with filter"""
    result = [x * 2 for x in range(10000) if x % 3 == 0]
    return len(result)

def bench_sum_builtin():
    """Built-in sum function"""
    lst = list(range(100000))
    return sum(lst)

def bench_json_encode():
    """JSON encoding"""
    import json
    data = {
        "users": [{"id": i, "name": f"user_{i}", "active": i % 2 == 0} for i in range(100)],
        "metadata": {"version": "1.0", "count": 100}
    }
    return json.dumps(data)

def bench_json_decode():
    """JSON decoding"""
    import json
    json_str = '{"users": [' + ','.join([f'{{"id": {i}, "name": "user_{i}", "active": {str(i % 2 == 0).lower()}}}' for i in range(100)]) + '], "metadata": {"version": "1.0", "count": 100}}'
    return json.loads(json_str)

def bench_regex():
    """Regular expression matching"""
    import re
    pattern = re.compile(r'\b\w+@\w+\.\w+\b')
    text = "Contact us at test@example.com or support@company.org for help. Invalid: @bad .com"
    matches = []
    for _ in range(100):
        matches = pattern.findall(text * 10)
    return len(matches)

def bench_file_io():
    """File I/O operations"""
    import tempfile
    import os
    
    with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.txt') as f:
        fname = f.name
        for i in range(500):
            f.write(f"Line {i}: " + "x" * 50 + "\n")
    
    with open(fname, 'r') as f:
        lines = f.readlines()
    
    os.unlink(fname)
    return len(lines)

# ============================================================================
# Main
# ============================================================================

BENCHMARKS = [
    ("int_arithmetic", bench_int_arithmetic, 100),
    ("string_ops", bench_string_ops, 500),
    ("list_ops", bench_list_ops, 200),
    ("dict_ops", bench_dict_ops, 100),
    ("function_calls", bench_function_calls, 100),
    ("object_creation", bench_object_creation, 200),
    ("list_comprehension", bench_list_comprehension, 200),
    ("sum_builtin", bench_sum_builtin, 100),
    ("json_encode", bench_json_encode, 100),
    ("json_decode", bench_json_decode, 100),
    ("regex", bench_regex, 50),
    ("file_io", bench_file_io, 30),
]

def main():
    runtime = sys.argv[1] if len(sys.argv) > 1 else "unknown"
    
    results = {"runtime": runtime, "version": sys.version.split()[0], "benchmarks": []}
    
    print(f"Running benchmarks on {runtime} ({sys.version.split()[0]})")
    print("=" * 60)
    
    for name, func, iterations in BENCHMARKS:
        print(f"  {name}...", end=" ", flush=True)
        try:
            mean, std = measure(name, func, iterations=iterations, warmup=20)
            results["benchmarks"].append({
                "name": name,
                "mean_ns": mean,
                "std_ns": std,
                "iterations": iterations
            })
            
            if mean >= 1_000_000_000:
                print(f"{mean/1e9:.3f}s")
            elif mean >= 1_000_000:
                print(f"{mean/1e6:.3f}ms")
            elif mean >= 1_000:
                print(f"{mean/1e3:.3f}µs")
            else:
                print(f"{mean:.0f}ns")
        except Exception as e:
            print(f"ERROR: {e}")
            results["benchmarks"].append({
                "name": name,
                "error": str(e)
            })
    
    # Output JSON to stdout for capture
    print("\n---JSON_START---")
    print(json.dumps(results))
    print("---JSON_END---")

if __name__ == "__main__":
    main()
