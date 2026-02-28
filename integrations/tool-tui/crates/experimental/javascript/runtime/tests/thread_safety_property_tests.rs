//! Property tests for Thread Safety Under Concurrent Execution
//!
//! Feature: production-readiness
//! Property 13: Thread Safety Under Concurrent Execution
//!
//! These tests verify that:
//! - Multiple JavaScript contexts can run in parallel without data races
//! - Each context maintains independent state
//! - Global compiler state is properly synchronized
//! - Results are deterministic regardless of execution interleaving
//!
//! **Validates: Requirements 7.1, 7.2, 7.3, 7.4**

use dx_js_runtime::runtime::context::{ContextConfig, ContextId, JsContext};
use dx_js_runtime::value::Value;
use proptest::prelude::*;
use std::sync::{Arc, Barrier};
use std::thread;

// ============================================================================
// Property 13.1: Context ID Uniqueness Across Threads
// Context IDs SHALL be unique even when created concurrently in multiple threads.
// **Validates: Requirements 7.1, 7.2**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Context IDs are unique across concurrent creation
    #[test]
    fn prop_context_ids_unique_across_threads(
        num_threads in 2usize..8,
        contexts_per_thread in 1usize..10
    ) {
        let barrier = Arc::new(Barrier::new(num_threads));
        let mut handles = Vec::new();

        for _ in 0..num_threads {
            let barrier = Arc::clone(&barrier);
            let handle = thread::spawn(move || {
                // Wait for all threads to be ready
                barrier.wait();

                // Create contexts and collect their IDs
                let mut ids = Vec::new();
                for _ in 0..contexts_per_thread {
                    let ctx = JsContext::default_context();
                    ids.push(ctx.id().as_u64());
                }
                ids
            });
            handles.push(handle);
        }

        // Collect all IDs from all threads
        let mut all_ids: Vec<u64> = Vec::new();
        for handle in handles {
            let ids = handle.join().expect("Thread panicked");
            all_ids.extend(ids);
        }

        // Property: All IDs should be unique
        let unique_count = {
            let mut sorted = all_ids.clone();
            sorted.sort();
            sorted.dedup();
            sorted.len()
        };

        prop_assert_eq!(
            unique_count,
            all_ids.len(),
            "All context IDs should be unique, but found duplicates"
        );
    }
}

// ============================================================================
// Property 13.2: Independent Context State
// Each context SHALL maintain independent state that is not affected by
// operations in other contexts running in parallel.
// **Validates: Requirements 7.1, 7.3**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Context globals are independent across threads
    #[test]
    fn prop_context_globals_independent(
        num_threads in 2usize..6,
        global_value in 1i64..1000
    ) {
        let barrier = Arc::new(Barrier::new(num_threads));
        let mut handles = Vec::new();

        for thread_id in 0..num_threads {
            let barrier = Arc::clone(&barrier);
            let expected_value = global_value + thread_id as i64;

            let handle = thread::spawn(move || {
                // Wait for all threads to be ready
                barrier.wait();

                // Create context and set a thread-specific global
                let mut ctx = JsContext::default_context();
                ctx.set_global("thread_value", Value::Number(expected_value as f64));

                // Do some work to allow interleaving
                for _ in 0..100 {
                    std::hint::spin_loop();
                }

                // Verify the global is still our value
                let actual = ctx.get_global("thread_value");

                (expected_value, actual.cloned())
            });
            handles.push(handle);
        }

        // Verify each thread saw its own value
        for handle in handles {
            let (expected, actual) = handle.join().expect("Thread panicked");

            prop_assert_eq!(
                actual,
                Some(Value::Number(expected as f64)),
                "Context global should maintain thread-specific value"
            );
        }
    }
}

// ============================================================================
// Property 13.3: Concurrent Context Operations
// Multiple contexts performing operations concurrently SHALL NOT interfere
// with each other's state or cause data races.
// **Validates: Requirements 7.1, 7.2, 7.3**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Property: Concurrent context operations are isolated
    #[test]
    fn prop_concurrent_context_operations_isolated(
        num_threads in 2usize..5,
        operations_per_thread in 5usize..20
    ) {
        let barrier = Arc::new(Barrier::new(num_threads));
        let mut handles = Vec::new();

        for thread_id in 0..num_threads {
            let barrier = Arc::clone(&barrier);

            let handle = thread::spawn(move || {
                // Wait for all threads to be ready
                barrier.wait();

                let mut ctx = JsContext::default_context();
                let mut results = Vec::new();

                // Perform multiple operations
                for i in 0..operations_per_thread {
                    let key = format!("var_{}", i);
                    let value = (thread_id * 1000 + i) as f64;

                    ctx.set_global(&key, Value::Number(value));

                    // Verify immediately
                    if let Some(Value::Number(v)) = ctx.get_global(&key) {
                        results.push((*v, value));
                    }
                }

                (thread_id, results)
            });
            handles.push(handle);
        }

        // Verify all operations succeeded with correct values
        for handle in handles {
            let (thread_id, results) = handle.join().expect("Thread panicked");

            for (actual, expected) in results {
                prop_assert!(
                    (actual - expected).abs() < f64::EPSILON,
                    "Thread {} should see its own values: expected {}, got {}",
                    thread_id, expected, actual
                );
            }
        }
    }
}

// ============================================================================
// Property 13.4: Context Reset Independence
// Resetting one context SHALL NOT affect other contexts running in parallel.
// **Validates: Requirements 7.1, 7.3**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Context reset does not affect other contexts
    #[test]
    fn prop_context_reset_independent(
        num_threads in 2usize..6
    ) {
        let barrier = Arc::new(Barrier::new(num_threads));
        let reset_barrier = Arc::new(Barrier::new(num_threads));
        let mut handles = Vec::new();

        for thread_id in 0..num_threads {
            let barrier = Arc::clone(&barrier);
            let reset_barrier = Arc::clone(&reset_barrier);

            let handle = thread::spawn(move || {
                // Wait for all threads to be ready
                barrier.wait();

                let mut ctx = JsContext::default_context();
                let value = (thread_id * 100) as f64;

                // Set a global
                ctx.set_global("my_value", Value::Number(value));

                // Wait for all threads to set their globals
                reset_barrier.wait();

                // Only thread 0 resets its context
                if thread_id == 0 {
                    ctx.reset();
                }

                // All other threads should still have their values
                let has_value = ctx.get_global("my_value").is_some();

                (thread_id, has_value)
            });
            handles.push(handle);
        }

        // Verify results
        for handle in handles {
            let (thread_id, has_value) = handle.join().expect("Thread panicked");

            if thread_id == 0 {
                // Thread 0 reset its context, so it should NOT have the value
                prop_assert!(
                    !has_value,
                    "Thread 0 should not have value after reset"
                );
            } else {
                // Other threads should still have their values
                prop_assert!(
                    has_value,
                    "Thread {} should still have its value after thread 0 reset",
                    thread_id
                );
            }
        }
    }
}

// ============================================================================
// Property 13.5: Concurrent Context Creation and Destruction
// Creating and destroying contexts concurrently SHALL NOT cause data races
// or memory corruption.
// **Validates: Requirements 7.1, 7.2, 7.3**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    /// Property: Concurrent context lifecycle is safe
    #[test]
    fn prop_concurrent_context_lifecycle_safe(
        num_threads in 2usize..6,
        iterations in 5usize..15
    ) {
        let barrier = Arc::new(Barrier::new(num_threads));
        let mut handles = Vec::new();

        for thread_id in 0..num_threads {
            let barrier = Arc::clone(&barrier);

            let handle = thread::spawn(move || {
                // Wait for all threads to be ready
                barrier.wait();

                let mut context_ids = Vec::new();

                for i in 0..iterations {
                    // Create a context
                    let mut ctx = JsContext::new(ContextConfig {
                        name: Some(format!("thread-{}-iter-{}", thread_id, i)),
                        ..Default::default()
                    });

                    context_ids.push(ctx.id().as_u64());

                    // Use the context
                    ctx.set_global("iteration", Value::Number(i as f64));

                    // Verify
                    let value = ctx.get_global("iteration");
                    assert_eq!(value, Some(&Value::Number(i as f64)));

                    // Terminate the context
                    ctx.terminate();

                    // Context is dropped here
                }

                context_ids
            });
            handles.push(handle);
        }

        // Collect all context IDs
        let mut all_ids: Vec<u64> = Vec::new();
        for handle in handles {
            let ids = handle.join().expect("Thread panicked");
            all_ids.extend(ids);
        }

        // Property: All context IDs should be unique
        let unique_count = {
            let mut sorted = all_ids.clone();
            sorted.sort();
            sorted.dedup();
            sorted.len()
        };

        prop_assert_eq!(
            unique_count,
            all_ids.len(),
            "All context IDs should be unique across all threads and iterations"
        );
    }
}

// ============================================================================
// Property 13.6: State Isolation Under High Contention
// Under high contention, each context SHALL maintain correct state.
// **Validates: Requirements 7.1, 7.2, 7.3, 7.4**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// Property: State isolation under high contention
    #[test]
    fn prop_state_isolation_high_contention(
        num_threads in 4usize..8,
        operations in 50usize..100
    ) {
        let barrier = Arc::new(Barrier::new(num_threads));
        let mut handles = Vec::new();

        for thread_id in 0..num_threads {
            let barrier = Arc::clone(&barrier);

            let handle = thread::spawn(move || {
                // Wait for all threads to be ready
                barrier.wait();

                let mut ctx = JsContext::default_context();
                let mut sum = 0i64;

                // Perform many rapid operations
                for i in 0..operations {
                    let value = (thread_id as i64 * 10000) + (i as i64);
                    ctx.set_global("counter", Value::Number(value as f64));

                    // Immediately read back
                    if let Some(Value::Number(v)) = ctx.get_global("counter") {
                        sum += *v as i64;
                    }
                }

                // Calculate expected sum
                let expected_sum: i64 = (0..operations as i64)
                    .map(|i| (thread_id as i64 * 10000) + i)
                    .sum();

                (thread_id, sum, expected_sum)
            });
            handles.push(handle);
        }

        // Verify each thread got correct results
        for handle in handles {
            let (thread_id, actual_sum, expected_sum) = handle.join().expect("Thread panicked");

            prop_assert_eq!(
                actual_sum,
                expected_sum,
                "Thread {} should have correct sum: expected {}, got {}",
                thread_id, expected_sum, actual_sum
            );
        }
    }
}

// ============================================================================
// Unit Tests for Edge Cases
// ============================================================================

#[test]
fn test_context_id_atomic_increment() {
    // Create many context IDs rapidly and verify uniqueness
    let ids: Vec<u64> = (0..1000).map(|_| ContextId::new().as_u64()).collect();

    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();

    assert_eq!(sorted.len(), ids.len(), "All context IDs should be unique");
}

#[test]
fn test_parallel_context_creation() {
    let num_threads = 4;
    let contexts_per_thread = 100;

    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            thread::spawn(move || {
                (0..contexts_per_thread)
                    .map(|_| JsContext::default_context().id().as_u64())
                    .collect::<Vec<_>>()
            })
        })
        .collect();

    let all_ids: Vec<u64> = handles.into_iter().flat_map(|h| h.join().unwrap()).collect();

    let mut sorted = all_ids.clone();
    sorted.sort();
    sorted.dedup();

    assert_eq!(
        sorted.len(),
        all_ids.len(),
        "All {} context IDs should be unique",
        all_ids.len()
    );
}

#[test]
fn test_context_state_isolation() {
    let num_threads = 4;
    let barrier = Arc::new(Barrier::new(num_threads));

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();

                let mut ctx = JsContext::default_context();
                let value = thread_id as f64 * 1000.0;

                ctx.set_global("x", Value::Number(value));

                // Spin to allow interleaving
                for _ in 0..1000 {
                    std::hint::spin_loop();
                }

                match ctx.get_global("x") {
                    Some(Value::Number(v)) => *v,
                    _ => -1.0,
                }
            })
        })
        .collect();

    for (thread_id, handle) in handles.into_iter().enumerate() {
        let result = handle.join().unwrap();
        let expected = thread_id as f64 * 1000.0;
        assert!(
            (result - expected).abs() < f64::EPSILON,
            "Thread {} should see value {}, got {}",
            thread_id,
            expected,
            result
        );
    }
}

#[test]
fn test_context_terminate_isolation() {
    let num_threads = 4;
    let barrier = Arc::new(Barrier::new(num_threads));
    let terminate_barrier = Arc::new(Barrier::new(num_threads));

    let handles: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let barrier = Arc::clone(&barrier);
            let terminate_barrier = Arc::clone(&terminate_barrier);

            thread::spawn(move || {
                barrier.wait();

                let mut ctx = JsContext::default_context();
                ctx.set_global("value", Value::Number(thread_id as f64));

                terminate_barrier.wait();

                // Thread 0 terminates its context
                if thread_id == 0 {
                    ctx.terminate();
                }

                // Check if context is still usable
                ctx.is_ready()
            })
        })
        .collect();

    for (thread_id, handle) in handles.into_iter().enumerate() {
        let is_ready = handle.join().unwrap();

        if thread_id == 0 {
            assert!(!is_ready, "Thread 0's context should be terminated");
        } else {
            assert!(is_ready, "Thread {}'s context should still be ready", thread_id);
        }
    }
}
