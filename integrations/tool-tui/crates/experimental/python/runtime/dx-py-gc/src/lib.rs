//! DX-Py GC - Lock-Free Parallel Garbage Collector
//!
//! This crate implements a high-performance garbage collector for the DX-Py runtime
//! with sub-100μs maximum pause times.
//!
//! ## Features
//!
//! - [`LockFreeRefCount`]: Atomic reference counting for immediate reclamation
//! - [`EpochGc`]: Epoch-based reclamation for safe memory deallocation
//! - [`CycleDetector`]: Concurrent cycle detection without stop-the-world pauses
//!
//! ## Architecture
//!
//! The GC uses a hybrid approach:
//!
//! 1. **Reference Counting**: Most objects are reclaimed immediately when their
//!    reference count drops to zero. Uses lock-free atomics for thread safety.
//!
//! 2. **Epoch-Based Reclamation**: Objects that might be accessed by other threads
//!    are deferred to epoch-based collection for safe deallocation.
//!
//! 3. **Cycle Detection**: Reference cycles are detected using a concurrent
//!    Bacon-Rajan algorithm with parallel tracing.
//!
//! ## Usage
//!
//! ```rust
//! use dx_py_gc::{LockFreeRefCount, EpochGc, GcConfig};
//!
//! // Reference counting
//! let rc = LockFreeRefCount::new();
//! rc.inc_strong();
//! assert_eq!(rc.strong_count(), 2);
//! rc.dec_strong();
//! assert_eq!(rc.strong_count(), 1);
//!
//! // Epoch-based GC
//! let gc = EpochGc::new(4);
//! let thread_id = gc.register_thread().unwrap();
//! let _epoch = gc.enter_epoch(thread_id);
//! // ... do work ...
//! gc.exit_epoch(thread_id);
//! gc.try_collect();
//! ```
//!
//! ## Performance
//!
//! - Reference count operations: ~5ns per inc/dec
//! - Epoch enter/exit: ~10ns
//! - Collection pause: <100μs (no stop-the-world)

pub mod cycle;
pub mod epoch;
pub mod refcount;

pub use cycle::CycleDetector;
pub use epoch::EpochGc;
pub use refcount::LockFreeRefCount;

/// GC configuration
///
/// Controls the behavior of the garbage collector.
///
/// # Example
///
/// ```rust
/// use dx_py_gc::GcConfig;
///
/// let config = GcConfig {
///     epoch_count: 3,
///     max_garbage_per_epoch: 10000,
///     enable_cycle_detection: true,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct GcConfig {
    /// Number of epochs to keep before reclaiming (default: 3)
    pub epoch_count: usize,
    /// Maximum objects per garbage list before triggering collection (default: 10000)
    pub max_garbage_per_epoch: usize,
    /// Enable cycle detection (default: true)
    pub enable_cycle_detection: bool,
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            epoch_count: 3,
            max_garbage_per_epoch: 10000,
            enable_cycle_detection: true,
        }
    }
}
