//! Performance benchmarks for DX-Py runtime

use std::time::{Duration, Instant};

/// Benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u64,
    pub total_time: Duration,
    pub mean_time: Duration,
    pub min_time: Duration,
    pub max_time: Duration,
}

impl BenchmarkResult {
    pub fn ops_per_sec(&self) -> f64 {
        self.iterations as f64 / self.total_time.as_secs_f64()
    }
}

/// Run a benchmark
pub fn run_benchmark<F>(name: &str, iterations: u64, mut f: F) -> BenchmarkResult
where
    F: FnMut(),
{
    let mut times = Vec::with_capacity(iterations as usize);

    // Warmup
    for _ in 0..10 {
        f();
    }

    // Actual benchmark
    let start = Instant::now();
    for _ in 0..iterations {
        let iter_start = Instant::now();
        f();
        times.push(iter_start.elapsed());
    }
    let total_time = start.elapsed();

    let min_time = *times.iter().min().unwrap_or(&Duration::ZERO);
    let max_time = *times.iter().max().unwrap_or(&Duration::ZERO);
    let mean_time = total_time / iterations as u32;

    BenchmarkResult {
        name: name.to_string(),
        iterations,
        total_time,
        mean_time,
        min_time,
        max_time,
    }
}

/// Startup time benchmark
pub fn bench_startup() -> BenchmarkResult {
    use dx_py_interpreter::VirtualMachine;

    run_benchmark("startup", 1000, || {
        let _vm = VirtualMachine::new();
    })
}

/// Simple expression evaluation benchmark
pub fn bench_eval_int() -> BenchmarkResult {
    use dx_py_interpreter::VirtualMachine;

    let vm = VirtualMachine::new();

    run_benchmark("eval_int", 10000, || {
        let _ = vm.eval_expr("42");
    })
}

/// Builtin function call benchmark
pub fn bench_builtin_call() -> BenchmarkResult {
    use dx_py_core::pylist::PyValue;
    use dx_py_interpreter::VirtualMachine;

    let vm = VirtualMachine::new();

    run_benchmark("builtin_len", 10000, || {
        let _ = vm.call_builtin("len", &[PyValue::Str(std::sync::Arc::from("hello world"))]);
    })
}

/// List operations benchmark
pub fn bench_list_ops() -> BenchmarkResult {
    use dx_py_core::pylist::PyValue;
    use dx_py_core::PyList;

    run_benchmark("list_ops", 1000, || {
        let list = PyList::new();
        for i in 0..100 {
            list.append(PyValue::Int(i));
        }
        for i in 0..100 {
            let _ = list.getitem(i);
        }
    })
}

/// Dict operations benchmark
pub fn bench_dict_ops() -> BenchmarkResult {
    use dx_py_core::pydict::PyKey;
    use dx_py_core::pylist::PyValue;
    use dx_py_core::PyDict;

    run_benchmark("dict_ops", 1000, || {
        let dict = PyDict::new();
        for i in 0..100 {
            dict.setitem(PyKey::Int(i), PyValue::Int(i * 2));
        }
        for i in 0..100 {
            let _ = dict.getitem(&PyKey::Int(i));
        }
    })
}

/// String operations benchmark
pub fn bench_string_ops() -> BenchmarkResult {
    use dx_py_core::PyStr;

    run_benchmark("string_ops", 1000, || {
        let s1 = PyStr::new("hello");
        let s2 = PyStr::new(" world");
        let s3 = s1.concat(&s2);
        let _ = s3.upper();
        let _ = s3.find(&PyStr::new("world"));
    })
}

/// Run all benchmarks
pub fn run_all_benchmarks() -> Vec<BenchmarkResult> {
    println!("Running DX-Py benchmarks...\n");

    let benchmarks = vec![
        bench_startup(),
        bench_eval_int(),
        bench_builtin_call(),
        bench_list_ops(),
        bench_dict_ops(),
        bench_string_ops(),
    ];

    println!(
        "\n{:<20} {:>12} {:>12} {:>12} {:>12}",
        "Benchmark", "Mean", "Min", "Max", "Ops/sec"
    );
    println!("{}", "-".repeat(72));

    for result in &benchmarks {
        println!(
            "{:<20} {:>12.3?} {:>12.3?} {:>12.3?} {:>12.0}",
            result.name,
            result.mean_time,
            result.min_time,
            result.max_time,
            result.ops_per_sec(),
        );
    }

    benchmarks
}

/// Validate performance targets
pub fn validate_targets(results: &[BenchmarkResult]) -> bool {
    let mut all_pass = true;

    println!("\nPerformance Target Validation:");
    println!("{}", "-".repeat(50));

    for result in results {
        let (target, pass) = match result.name.as_str() {
            "startup" => {
                let target = Duration::from_millis(3);
                (target, result.mean_time < target)
            }
            _ => continue,
        };

        let status = if pass { "PASS" } else { "FAIL" };
        println!(
            "{:<20} target: {:>8.3?}  actual: {:>8.3?}  [{}]",
            result.name, target, result.mean_time, status
        );

        if !pass {
            all_pass = false;
        }
    }

    all_pass
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result() {
        let result = BenchmarkResult {
            name: "test".to_string(),
            iterations: 1000,
            total_time: Duration::from_secs(1),
            mean_time: Duration::from_millis(1),
            min_time: Duration::from_micros(500),
            max_time: Duration::from_millis(2),
        };

        assert_eq!(result.ops_per_sec(), 1000.0);
    }

    #[test]
    fn test_run_benchmark() {
        let mut counter = 0;
        let result = run_benchmark("counter", 100, || {
            counter += 1;
        });

        assert_eq!(result.name, "counter");
        assert_eq!(result.iterations, 100);
    }
}
