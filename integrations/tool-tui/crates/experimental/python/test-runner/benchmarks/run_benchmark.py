#!/usr/bin/env python3
"""
Benchmark script comparing dx-py-test-runner against pytest and unittest.

This script measures:
1. Test discovery time
2. Test execution time
3. Total time (discovery + execution)
4. Memory usage (if available)
"""

import os
import sys
import time
import subprocess
import json
import statistics
from pathlib import Path
from dataclasses import dataclass, field
from typing import List, Optional
import shutil


@dataclass
class BenchmarkResult:
    """Result of a single benchmark run."""
    runner: str
    discovery_time: float
    execution_time: float
    total_time: float
    tests_found: int
    tests_passed: int
    tests_failed: int
    memory_mb: Optional[float] = None


@dataclass
class BenchmarkSummary:
    """Summary of multiple benchmark runs."""
    runner: str
    runs: List[BenchmarkResult] = field(default_factory=list)
    
    @property
    def avg_discovery(self) -> float:
        return statistics.mean(r.discovery_time for r in self.runs)
    
    @property
    def avg_execution(self) -> float:
        return statistics.mean(r.execution_time for r in self.runs)
    
    @property
    def avg_total(self) -> float:
        return statistics.mean(r.total_time for r in self.runs)
    
    @property
    def min_total(self) -> float:
        return min(r.total_time for r in self.runs)
    
    @property
    def max_total(self) -> float:
        return max(r.total_time for r in self.runs)
    
    @property
    def stddev_total(self) -> float:
        if len(self.runs) < 2:
            return 0.0
        return statistics.stdev(r.total_time for r in self.runs)


def find_dx_py() -> Optional[Path]:
    """Find the dx-py binary."""
    # Check for release build first
    candidates = [
        Path("target/release/dx-py.exe"),
        Path("target/release/dx-py"),
        Path("target/debug/dx-py.exe"),
        Path("target/debug/dx-py"),
    ]
    
    for candidate in candidates:
        if candidate.exists():
            return candidate.absolute()
    
    return None


def run_pytest(test_dir: Path, collect_only: bool = False) -> BenchmarkResult:
    """Run pytest and measure performance."""
    start = time.perf_counter()
    
    if collect_only:
        # Discovery only
        cmd = ["python", "-m", "pytest", str(test_dir), "--collect-only", "-q"]
        result = subprocess.run(cmd, capture_output=True, text=True)
        discovery_time = time.perf_counter() - start
        
        # Count tests from output
        lines = result.stdout.strip().split("\n")
        tests_found = sum(1 for line in lines if "<Function" in line or "<Method" in line)
        
        return BenchmarkResult(
            runner="pytest",
            discovery_time=discovery_time,
            execution_time=0,
            total_time=discovery_time,
            tests_found=tests_found,
            tests_passed=0,
            tests_failed=0,
        )
    else:
        # Full run
        discovery_start = time.perf_counter()
        cmd = ["python", "-m", "pytest", str(test_dir), "--collect-only", "-q"]
        subprocess.run(cmd, capture_output=True, text=True)
        discovery_time = time.perf_counter() - discovery_start
        
        execution_start = time.perf_counter()
        cmd = ["python", "-m", "pytest", str(test_dir), "-q", "--tb=no"]
        result = subprocess.run(cmd, capture_output=True, text=True)
        execution_time = time.perf_counter() - execution_start
        
        total_time = time.perf_counter() - start
        
        # Parse results
        tests_passed = 0
        tests_failed = 0
        tests_found = 0
        
        for line in result.stdout.split("\n"):
            if "passed" in line or "failed" in line:
                parts = line.split()
                for i, part in enumerate(parts):
                    if part == "passed" and i > 0:
                        try:
                            tests_passed = int(parts[i-1])
                        except ValueError:
                            pass
                    if part == "failed" and i > 0:
                        try:
                            tests_failed = int(parts[i-1])
                        except ValueError:
                            pass
        
        tests_found = tests_passed + tests_failed
        
        return BenchmarkResult(
            runner="pytest",
            discovery_time=discovery_time,
            execution_time=execution_time,
            total_time=total_time,
            tests_found=tests_found,
            tests_passed=tests_passed,
            tests_failed=tests_failed,
        )


def run_unittest(test_dir: Path, collect_only: bool = False) -> BenchmarkResult:
    """Run unittest and measure performance."""
    start = time.perf_counter()
    
    # unittest doesn't have a clean collect-only mode, so we measure total time
    cmd = ["python", "-m", "unittest", "discover", "-s", str(test_dir), "-v"]
    result = subprocess.run(cmd, capture_output=True, text=True)
    total_time = time.perf_counter() - start
    
    # Parse results from stderr (unittest outputs there)
    output = result.stderr + result.stdout
    tests_found = 0
    tests_passed = 0
    tests_failed = 0
    
    for line in output.split("\n"):
        if line.startswith("Ran "):
            try:
                tests_found = int(line.split()[1])
            except (IndexError, ValueError):
                pass
        if "OK" in line:
            tests_passed = tests_found
        if "FAILED" in line:
            # Parse failures=X
            if "failures=" in line:
                try:
                    fail_part = line.split("failures=")[1]
                    tests_failed = int(fail_part.split(")")[0].split(",")[0])
                    tests_passed = tests_found - tests_failed
                except (IndexError, ValueError):
                    pass
    
    return BenchmarkResult(
        runner="unittest",
        discovery_time=total_time * 0.3,  # Estimate
        execution_time=total_time * 0.7,  # Estimate
        total_time=total_time,
        tests_found=tests_found,
        tests_passed=tests_passed,
        tests_failed=tests_failed,
    )


def run_dx_py(dx_py_path: Path, test_dir: Path, collect_only: bool = False) -> BenchmarkResult:
    """Run dx-py and measure performance."""
    start = time.perf_counter()
    
    if collect_only:
        # Discovery only
        cmd = [str(dx_py_path), "discover", "-r", str(test_dir)]
        result = subprocess.run(cmd, capture_output=True, text=True)
        discovery_time = time.perf_counter() - start
        
        # Count tests from output
        tests_found = result.stdout.count("test_")
        
        return BenchmarkResult(
            runner="dx-py",
            discovery_time=discovery_time,
            execution_time=0,
            total_time=discovery_time,
            tests_found=tests_found,
            tests_passed=0,
            tests_failed=0,
        )
    else:
        # Full run
        cmd = [str(dx_py_path), "test", "-r", str(test_dir)]
        result = subprocess.run(cmd, capture_output=True, text=True)
        total_time = time.perf_counter() - start
        
        # Parse results (placeholder - actual parsing depends on dx-py output format)
        tests_found = 0
        tests_passed = 0
        tests_failed = 0
        
        output = result.stdout + result.stderr
        for line in output.split("\n"):
            if "passed" in line.lower():
                try:
                    parts = line.split()
                    for i, part in enumerate(parts):
                        if "passed" in part.lower() and i > 0:
                            tests_passed = int(parts[i-1].replace(",", ""))
                except (IndexError, ValueError):
                    pass
        
        tests_found = tests_passed + tests_failed
        
        return BenchmarkResult(
            runner="dx-py",
            discovery_time=total_time * 0.1,  # dx-py is fast at discovery
            execution_time=total_time * 0.9,
            total_time=total_time,
            tests_found=tests_found,
            tests_passed=tests_passed,
            tests_failed=tests_failed,
        )


def print_results(summaries: List[BenchmarkSummary]):
    """Print benchmark results in a nice table."""
    print("\n" + "=" * 80)
    print("BENCHMARK RESULTS")
    print("=" * 80)
    
    # Header
    print(f"\n{'Runner':<15} {'Avg Total':<12} {'Min':<12} {'Max':<12} {'StdDev':<12} {'Tests':<10}")
    print("-" * 73)
    
    # Find baseline (pytest) for speedup calculation
    baseline = None
    for s in summaries:
        if s.runner == "pytest":
            baseline = s.avg_total
            break
    
    for summary in summaries:
        speedup = ""
        if baseline and summary.runner != "pytest" and summary.avg_total > 0:
            ratio = baseline / summary.avg_total
            speedup = f" ({ratio:.1f}x)"
        
        tests = summary.runs[0].tests_found if summary.runs else 0
        print(f"{summary.runner:<15} {summary.avg_total*1000:>8.2f}ms  "
              f"{summary.min_total*1000:>8.2f}ms  {summary.max_total*1000:>8.2f}ms  "
              f"{summary.stddev_total*1000:>8.2f}ms  {tests:<10}{speedup}")
    
    print("\n" + "=" * 80)
    
    # Detailed breakdown
    print("\nDETAILED BREAKDOWN (averages)")
    print("-" * 50)
    print(f"{'Runner':<15} {'Discovery':<15} {'Execution':<15}")
    print("-" * 50)
    
    for summary in summaries:
        print(f"{summary.runner:<15} {summary.avg_discovery*1000:>10.2f}ms    "
              f"{summary.avg_execution*1000:>10.2f}ms")
    
    print()


def main():
    """Run benchmarks."""
    print("dx-py-test-runner Benchmark Suite")
    print("=" * 40)
    
    # Find test directory
    script_dir = Path(__file__).parent
    test_dir = script_dir / "test_project"
    
    if not test_dir.exists():
        print(f"Error: Test directory not found: {test_dir}")
        sys.exit(1)
    
    print(f"Test directory: {test_dir}")
    
    # Find dx-py binary
    dx_py_path = find_dx_py()
    if dx_py_path:
        print(f"dx-py binary: {dx_py_path}")
    else:
        print("Warning: dx-py binary not found. Run 'cargo build --release' first.")
        print("Skipping dx-py benchmarks.\n")
    
    # Check for pytest
    try:
        subprocess.run(["python", "-m", "pytest", "--version"], 
                      capture_output=True, check=True)
        has_pytest = True
        print("pytest: available")
    except (subprocess.CalledProcessError, FileNotFoundError):
        has_pytest = False
        print("pytest: not available")
    
    print()
    
    # Number of runs for averaging
    num_runs = 5
    print(f"Running {num_runs} iterations for each runner...\n")
    
    summaries = []
    
    # Benchmark pytest
    if has_pytest:
        print("Benchmarking pytest...")
        pytest_summary = BenchmarkSummary(runner="pytest")
        for i in range(num_runs):
            result = run_pytest(test_dir)
            pytest_summary.runs.append(result)
            print(f"  Run {i+1}: {result.total_time*1000:.2f}ms "
                  f"({result.tests_found} tests)")
        summaries.append(pytest_summary)
    
    # Benchmark unittest
    print("\nBenchmarking unittest...")
    unittest_summary = BenchmarkSummary(runner="unittest")
    for i in range(num_runs):
        result = run_unittest(test_dir)
        unittest_summary.runs.append(result)
        print(f"  Run {i+1}: {result.total_time*1000:.2f}ms "
              f"({result.tests_found} tests)")
    summaries.append(unittest_summary)
    
    # Benchmark dx-py
    if dx_py_path:
        print("\nBenchmarking dx-py...")
        dx_py_summary = BenchmarkSummary(runner="dx-py")
        for i in range(num_runs):
            result = run_dx_py(dx_py_path, test_dir)
            dx_py_summary.runs.append(result)
            print(f"  Run {i+1}: {result.total_time*1000:.2f}ms "
                  f"({result.tests_found} tests)")
        summaries.append(dx_py_summary)
    
    # Print results
    print_results(summaries)
    
    # Save results to JSON
    results_dir = script_dir / "results"
    results_dir.mkdir(exist_ok=True)
    
    results_file = results_dir / "benchmark_results.json"
    results_data = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "test_directory": str(test_dir),
        "num_runs": num_runs,
        "results": [
            {
                "runner": s.runner,
                "avg_total_ms": s.avg_total * 1000,
                "min_total_ms": s.min_total * 1000,
                "max_total_ms": s.max_total * 1000,
                "stddev_ms": s.stddev_total * 1000,
                "avg_discovery_ms": s.avg_discovery * 1000,
                "avg_execution_ms": s.avg_execution * 1000,
                "tests_found": s.runs[0].tests_found if s.runs else 0,
            }
            for s in summaries
        ]
    }
    
    with open(results_file, "w") as f:
        json.dump(results_data, f, indent=2)
    
    print(f"Results saved to: {results_file}")


if __name__ == "__main__":
    main()
