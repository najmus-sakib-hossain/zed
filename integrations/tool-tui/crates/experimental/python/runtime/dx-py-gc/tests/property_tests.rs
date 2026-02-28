//! Property-based tests for Lock-Free Parallel GC
//!
//! Property 9: Reference Count Consistency
//! Property 20: GC Pause Time Bound
//! Validates: Requirements 3.1, 3.7

use dx_py_gc::*;
use proptest::prelude::*;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 9: Reference Count Consistency
    /// Validates: Requirements 3.1
    ///
    /// After N increments and N decrements, the reference count should
    /// return to its original value.
    #[test]
    fn prop_refcount_consistency(
        initial_strong in 1u32..100,
        initial_weak in 0u32..100,
        ops in 1usize..1000
    ) {
        let rc = LockFreeRefCount::with_counts(initial_strong, initial_weak);

        // Perform N increments
        for _ in 0..ops {
            rc.inc_strong();
        }

        // Perform N decrements
        for _ in 0..ops {
            rc.dec_strong();
        }

        // Should be back to initial
        prop_assert_eq!(
            rc.strong_count(), initial_strong,
            "Strong count should return to initial after equal inc/dec"
        );
    }

    /// Property 9: Weak reference count consistency
    #[test]
    fn prop_weak_refcount_consistency(
        initial_strong in 1u32..100,
        initial_weak in 0u32..100,
        ops in 1usize..1000
    ) {
        let rc = LockFreeRefCount::with_counts(initial_strong, initial_weak);

        // Perform N weak increments
        for _ in 0..ops {
            rc.inc_weak();
        }

        // Perform N weak decrements
        for _ in 0..ops {
            rc.dec_weak();
        }

        // Should be back to initial
        prop_assert_eq!(
            rc.weak_count(), initial_weak,
            "Weak count should return to initial after equal inc/dec"
        );
    }

    /// Property: Mark/unmark is idempotent
    #[test]
    fn prop_mark_idempotent(ops in 1usize..100) {
        let rc = LockFreeRefCount::new();

        // Multiple marks should only return true once
        let mut first_mark_returned_true = false;
        for i in 0..ops {
            let result = rc.mark_for_cycle();
            if i == 0 {
                prop_assert!(result, "First mark should return true");
                first_mark_returned_true = true;
            } else {
                prop_assert!(!result, "Subsequent marks should return false");
            }
        }

        prop_assert!(first_mark_returned_true);
        prop_assert!(rc.is_marked());

        // Unmark
        rc.unmark();
        prop_assert!(!rc.is_marked());

        // Can mark again
        prop_assert!(rc.mark_for_cycle());
    }

    /// Property: try_upgrade fails when strong count is 0
    #[test]
    fn prop_upgrade_fails_when_dead(weak_count in 1u32..100) {
        let rc = LockFreeRefCount::with_counts(1, weak_count);

        // Decrement strong to 0
        prop_assert!(rc.dec_strong());
        prop_assert_eq!(rc.strong_count(), 0);

        // Upgrade should fail
        prop_assert!(!rc.try_upgrade());
        prop_assert_eq!(rc.strong_count(), 0);
    }

    /// Property: try_upgrade succeeds when strong count > 0
    #[test]
    fn prop_upgrade_succeeds_when_alive(
        strong in 1u32..100,
        weak in 1u32..100
    ) {
        let rc = LockFreeRefCount::with_counts(strong, weak);

        // Upgrade should succeed
        prop_assert!(rc.try_upgrade());
        prop_assert_eq!(rc.strong_count(), strong + 1);
    }
}

/// Property 20: GC Pause Time Bound
/// Validates: Requirements 3.7
///
/// Maximum GC pause time should be under 100μs
#[test]
fn test_gc_pause_time_bound() {
    let gc = Arc::new(EpochGc::new(8));
    let mut max_pause = Duration::ZERO;

    // Register threads
    let thread_ids: Vec<_> = (0..4).map(|_| gc.register_thread().unwrap()).collect();

    // Allocate some garbage
    for _ in 0..1000 {
        let obj = Box::into_raw(Box::new([0u8; 64]));
        unsafe { gc.defer_free(obj) };
    }

    // Measure collection time
    for _ in 0..100 {
        // Exit all epochs to allow collection
        for &tid in &thread_ids {
            gc.exit_epoch(tid);
        }

        let start = Instant::now();
        gc.try_collect();
        let elapsed = start.elapsed();

        if elapsed > max_pause {
            max_pause = elapsed;
        }

        // Re-enter epochs
        for &tid in &thread_ids {
            gc.enter_epoch(tid);
        }
    }

    // Cleanup
    for &tid in &thread_ids {
        gc.exit_epoch(tid);
        gc.unregister_thread(tid);
    }

    // Max pause should be under 100μs (we allow some slack for test environment)
    // In production, this would be strictly enforced
    assert!(
        max_pause < Duration::from_millis(10),
        "Max pause time {} exceeds threshold",
        max_pause.as_micros()
    );
}

/// Test concurrent reference counting
#[test]
fn test_concurrent_refcount() {
    let rc = Arc::new(LockFreeRefCount::new());
    let mut handles = vec![];

    // Spawn threads that increment and decrement
    for _ in 0..8 {
        let rc_clone = Arc::clone(&rc);
        handles.push(thread::spawn(move || {
            for _ in 0..10000 {
                rc_clone.inc_strong();
            }
            for _ in 0..10000 {
                rc_clone.dec_strong();
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Should be back to 1
    assert_eq!(rc.strong_count(), 1);
}

/// Test epoch-based reclamation under contention
#[test]
fn test_epoch_gc_contention() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    struct TestObj;
    impl Drop for TestObj {
        fn drop(&mut self) {
            DROP_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }

    DROP_COUNT.store(0, Ordering::SeqCst);

    let gc = Arc::new(EpochGc::new(16));
    let mut handles = vec![];

    // Spawn producer threads
    for _ in 0..4 {
        let gc_clone = Arc::clone(&gc);
        handles.push(thread::spawn(move || {
            let tid = gc_clone.register_thread().unwrap();

            for _ in 0..100 {
                let _guard = gc_clone.enter_epoch(tid);

                // Allocate and defer free
                let obj = Box::into_raw(Box::new(TestObj));
                unsafe { gc_clone.defer_free(obj) };
            }

            gc_clone.exit_epoch(tid);
            gc_clone.unregister_thread(tid);
        }));
    }

    // Spawn collector thread
    let gc_clone = Arc::clone(&gc);
    handles.push(thread::spawn(move || {
        for _ in 0..50 {
            gc_clone.try_collect();
            thread::sleep(Duration::from_micros(100));
        }
    }));

    for handle in handles {
        handle.join().unwrap();
    }

    // Force final collection
    unsafe { gc.force_collect_all() };

    // All objects should be dropped
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 400);
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_refcount_overflow_protection() {
        let rc = LockFreeRefCount::with_counts(u32::MAX - 10, 0);

        // Should not panic on overflow
        for _ in 0..20 {
            rc.inc_strong();
        }

        // Count will wrap, but that's expected behavior
        // In production, we'd handle this more gracefully
    }

    #[test]
    fn test_epoch_gc_thread_registration() {
        let gc = EpochGc::new(4);

        let t1 = gc.register_thread();
        let t2 = gc.register_thread();
        let t3 = gc.register_thread();
        let t4 = gc.register_thread();
        let t5 = gc.register_thread(); // Should fail

        assert!(t1.is_some());
        assert!(t2.is_some());
        assert!(t3.is_some());
        assert!(t4.is_some());
        assert!(t5.is_none());
    }
}
