//! GC integration tests

use dx_py_gc::{LockFreeRefCount, EpochGc, CycleDetector, GcConfig};
use std::sync::Arc;
use std::thread;

#[test]
fn test_refcount_basic() {
    let rc = LockFreeRefCount::new();
    
    assert_eq!(rc.strong_count(), 1);
    assert_eq!(rc.weak_count(), 0);
    
    rc.inc_strong();
    assert_eq!(rc.strong_count(), 2);
    
    rc.dec_strong();
    assert_eq!(rc.strong_count(), 1);
}

#[test]
fn test_refcount_weak() {
    let rc = LockFreeRefCount::new();
    
    rc.inc_weak();
    assert_eq!(rc.weak_count(), 1);
    
    rc.inc_weak();
    assert_eq!(rc.weak_count(), 2);
    
    rc.dec_weak();
    assert_eq!(rc.weak_count(), 1);
}

#[test]
fn test_refcount_concurrent() {
    let rc = Arc::new(LockFreeRefCount::new());
    let mut handles = vec![];
    
    // Spawn threads that increment and decrement
    for _ in 0..10 {
        let rc_clone = Arc::clone(&rc);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                rc_clone.inc_strong();
            }
            for _ in 0..1000 {
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

#[test]
fn test_epoch_gc_basic() {
    let gc = EpochGc::new();
    
    // Enter and exit epoch
    let guard = gc.enter_epoch();
    drop(guard);
    
    // Should be able to advance epoch
    gc.try_collect();
}

#[test]
fn test_epoch_gc_defer_free() {
    let gc = EpochGc::new();
    
    // Defer some allocations
    let data = Box::new(42i64);
    let ptr = Box::into_raw(data);
    
    gc.defer_free(ptr as *mut u8, std::mem::size_of::<i64>());
    
    // Advance epochs to trigger collection
    for _ in 0..5 {
        gc.try_collect();
    }
}

#[test]
fn test_cycle_detector_basic() {
    let detector = CycleDetector::new();
    
    // Add some roots
    let ptr1 = 0x1000 as *mut u8;
    let ptr2 = 0x2000 as *mut u8;
    
    detector.add_root(ptr1);
    detector.add_root(ptr2);
    
    // Detect cycles (should find none with no actual cycles)
    let cycles = detector.detect_cycles();
    assert!(cycles.is_empty());
}

#[test]
fn test_gc_config() {
    let config = GcConfig::default();
    
    assert_eq!(config.epoch_count, 3);
    assert_eq!(config.max_garbage_per_epoch, 10000);
    assert!(config.enable_cycle_detection);
}

#[test]
fn test_refcount_mark_for_cycle() {
    let rc = LockFreeRefCount::new();
    
    assert!(!rc.is_marked());
    
    rc.mark_for_cycle();
    assert!(rc.is_marked());
    
    rc.unmark();
    assert!(!rc.is_marked());
}

#[test]
fn test_refcount_try_upgrade() {
    let rc = LockFreeRefCount::new();
    
    // Add weak reference
    rc.inc_weak();
    
    // Should be able to upgrade while strong count > 0
    assert!(rc.try_upgrade());
    assert_eq!(rc.strong_count(), 2);
    
    // Clean up
    rc.dec_strong();
    rc.dec_weak();
}

#[test]
fn test_epoch_gc_concurrent() {
    let gc = Arc::new(EpochGc::new());
    let mut handles = vec![];
    
    // Spawn threads that enter/exit epochs
    for _ in 0..10 {
        let gc_clone = Arc::clone(&gc);
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                let _guard = gc_clone.enter_epoch();
                // Do some work
                std::thread::yield_now();
            }
        }));
    }
    
    // Collector thread
    let gc_clone = Arc::clone(&gc);
    handles.push(thread::spawn(move || {
        for _ in 0..50 {
            gc_clone.try_collect();
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
    }));
    
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_gc_stress() {
    let gc = EpochGc::new();
    
    // Allocate and defer many objects
    for i in 0..1000 {
        let data = Box::new(i as i64);
        let ptr = Box::into_raw(data);
        gc.defer_free(ptr as *mut u8, std::mem::size_of::<i64>());
        
        // Periodically collect
        if i % 100 == 0 {
            gc.try_collect();
        }
    }
    
    // Final collection
    for _ in 0..10 {
        gc.try_collect();
    }
}
