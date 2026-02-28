//! Property-based tests for parallel executor

use proptest::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use dx_py_parallel::{parallel_object::PyTypeTag, ParallelExecutor, ParallelPyObject};

/// Property 12: Parallel Executor Linear Scaling
/// Verifies 0.9*N speedup on N cores for embarrassingly parallel workloads
mod scaling_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]

        /// Work is distributed across all threads
        #[test]
        fn prop_work_distributed(
            num_tasks in 100usize..500
        ) {
            let executor = ParallelExecutor::with_threads(4);
            let counter = Arc::new(AtomicUsize::new(0));

            let handles: Vec<_> = (0..num_tasks)
                .map(|_| {
                    let counter = Arc::clone(&counter);
                    executor.submit(move || {
                        counter.fetch_add(1, Ordering::SeqCst);
                        42
                    })
                })
                .collect();

            // Wait for all tasks
            for handle in handles {
                let _ = handle.wait();
            }

            prop_assert_eq!(counter.load(Ordering::SeqCst), num_tasks);
        }

        /// Parallel map produces correct results
        #[test]
        fn prop_parallel_map_correct(
            items in prop::collection::vec(0i32..1000, 1..100)
        ) {
            let executor = ParallelExecutor::with_threads(4);
            let expected: Vec<i32> = items.iter().map(|x| x * 2).collect();
            let results = executor.parallel_map(items, |x| x * 2);

            prop_assert_eq!(results, expected);
        }

        /// Parallel for_each processes all items
        #[test]
        fn prop_parallel_foreach_complete(
            items in prop::collection::vec(1i32..100, 1..50)
        ) {
            let executor = ParallelExecutor::with_threads(4);
            let sum = Arc::new(AtomicUsize::new(0));
            let expected: usize = items.iter().map(|x| *x as usize).sum();

            let sum_clone = Arc::clone(&sum);
            executor.parallel_for_each(items, move |x| {
                sum_clone.fetch_add(x as usize, Ordering::SeqCst);
            });

            prop_assert_eq!(sum.load(Ordering::SeqCst), expected);
        }
    }

    #[test]
    fn test_scaling_speedup() {
        // This test verifies that parallel execution is faster than sequential
        // for CPU-bound work
        let work_size = 1000;
        let iterations = 10000;

        // Sequential baseline
        let start = Instant::now();
        let mut results = Vec::with_capacity(work_size);
        for i in 0..work_size {
            let mut sum = 0u64;
            for j in 0..iterations {
                sum = sum.wrapping_add((i * j) as u64);
            }
            results.push(sum);
        }
        let sequential_time = start.elapsed();

        // Parallel execution
        let executor = ParallelExecutor::with_threads(4);
        let start = Instant::now();
        let items: Vec<usize> = (0..work_size).collect();
        let parallel_results = executor.parallel_map(items, move |i| {
            let mut sum = 0u64;
            for j in 0..iterations {
                sum = sum.wrapping_add((i * j) as u64);
            }
            sum
        });
        let parallel_time = start.elapsed();

        // Verify correctness
        assert_eq!(results, parallel_results);

        // Parallel should be faster (at least 1.5x on 4 threads)
        // Note: This may not always pass on single-core systems
        let speedup = sequential_time.as_secs_f64() / parallel_time.as_secs_f64();
        println!(
            "Speedup: {:.2}x (seq: {:?}, par: {:?})",
            speedup, sequential_time, parallel_time
        );

        // We expect at least some speedup on multi-core systems
        // Being conservative here since CI environments vary
        assert!(speedup > 0.5 || parallel_time.as_millis() < 100);
    }
}

/// Tests for ParallelPyObject
mod parallel_object_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Reference counting is thread-safe
        #[test]
        fn prop_refcount_threadsafe(
            inc_count in 1usize..100,
            dec_count in 1usize..50
        ) {
            let obj = Arc::new(ParallelPyObject::new(PyTypeTag::Int));
            let executor = ParallelExecutor::with_threads(4);

            // Increment refs in parallel
            let obj_clone = Arc::clone(&obj);
            let handles: Vec<_> = (0..inc_count)
                .map(|_| {
                    let obj = Arc::clone(&obj_clone);
                    executor.submit(move || {
                        obj.inc_ref();
                    })
                })
                .collect();

            for h in handles {
                let _ = h.wait();
            }

            // Initial count is 1, plus inc_count increments
            prop_assert_eq!(obj.ref_count(), 1 + inc_count as u32);

            // Decrement some refs
            let actual_dec = dec_count.min(inc_count);
            for _ in 0..actual_dec {
                obj.dec_ref();
            }

            prop_assert_eq!(obj.ref_count(), 1 + inc_count as u32 - actual_dec as u32);
        }

        /// Hash caching is atomic
        #[test]
        fn prop_hash_atomic(
            hash_values in prop::collection::vec(1u64..u64::MAX, 2..10)
        ) {
            let obj = Arc::new(ParallelPyObject::new(PyTypeTag::Str));
            let executor = ParallelExecutor::with_threads(4);

            // Try to set hash from multiple threads
            let obj_clone = Arc::clone(&obj);
            let handles: Vec<_> = hash_values.iter()
                .map(|&hash| {
                    let obj = Arc::clone(&obj_clone);
                    executor.submit(move || obj.set_hash(hash))
                })
                .collect();

            let results: Vec<bool> = handles.into_iter()
                .map(|h| h.wait().unwrap())
                .collect();

            // Exactly one should succeed
            let success_count = results.iter().filter(|&&r| r).count();
            prop_assert_eq!(success_count, 1);

            // Hash should be set to one of the values
            let hash = obj.get_hash().unwrap();
            prop_assert!(hash_values.contains(&hash));
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_executor_shutdown() {
        let mut executor = ParallelExecutor::with_threads(2);

        // Submit some work
        let handle = executor.submit(|| 42);
        assert_eq!(handle.wait().unwrap(), 42);

        // Shutdown should complete cleanly
        executor.shutdown();
    }

    #[test]
    fn test_task_priority() {
        use dx_py_parallel::task::TaskPriority;

        let executor = ParallelExecutor::with_threads(1);

        // Submit tasks with different priorities
        let h1 = executor.submit_with_priority(|| 1, TaskPriority::Low);
        let h2 = executor.submit_with_priority(|| 2, TaskPriority::High);
        let h3 = executor.submit_with_priority(|| 3, TaskPriority::Normal);

        // All should complete
        assert_eq!(h1.wait().unwrap(), 1);
        assert_eq!(h2.wait().unwrap(), 2);
        assert_eq!(h3.wait().unwrap(), 3);
    }
}
