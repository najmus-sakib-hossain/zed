//! Epoch-based garbage collector
//!
//! Uses epoch-based reclamation to safely deallocate memory without
//! stop-the-world pauses.

use crossbeam::queue::SegQueue;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

/// Epoch-based garbage collector
pub struct EpochGc {
    /// Global epoch counter
    global_epoch: AtomicU64,
    /// Per-thread epoch tracking
    thread_epochs: Vec<AtomicU64>,
    /// Garbage lists for each epoch (circular buffer of 3)
    garbage_lists: [SegQueue<GarbageItem>; 3],
    /// Number of registered threads
    thread_count: AtomicUsize,
    /// Maximum threads supported
    max_threads: usize,
}

/// An item waiting to be garbage collected
pub struct GarbageItem {
    /// Pointer to the object
    ptr: *mut u8,
    /// Size of the object in bytes (kept for potential future use)
    #[allow(dead_code)]
    size: usize,
    /// Drop function to call
    drop_fn: unsafe fn(*mut u8),
}

// Safety: GarbageItem is Send because we only access the pointer
// after ensuring no other threads can access it (via epoch mechanism)
unsafe impl Send for GarbageItem {}
unsafe impl Sync for GarbageItem {}

impl EpochGc {
    /// Create a new epoch-based GC with the specified max thread count
    pub fn new(max_threads: usize) -> Self {
        let thread_epochs: Vec<AtomicU64> =
            (0..max_threads).map(|_| AtomicU64::new(u64::MAX)).collect();

        Self {
            global_epoch: AtomicU64::new(0),
            thread_epochs,
            garbage_lists: [SegQueue::new(), SegQueue::new(), SegQueue::new()],
            thread_count: AtomicUsize::new(0),
            max_threads,
        }
    }

    /// Register a new thread and return its ID
    pub fn register_thread(&self) -> Option<usize> {
        let id = self.thread_count.fetch_add(1, Ordering::SeqCst);
        if id >= self.max_threads {
            self.thread_count.fetch_sub(1, Ordering::SeqCst);
            return None;
        }
        Some(id)
    }

    /// Unregister a thread
    pub fn unregister_thread(&self, thread_id: usize) {
        if thread_id < self.max_threads {
            self.thread_epochs[thread_id].store(u64::MAX, Ordering::SeqCst);
        }
    }

    /// Enter a critical section (must call exit_epoch when done)
    ///
    /// Returns the current epoch number.
    pub fn enter_epoch(&self, thread_id: usize) -> u64 {
        let epoch = self.global_epoch.load(Ordering::SeqCst);
        if thread_id < self.thread_epochs.len() {
            self.thread_epochs[thread_id].store(epoch, Ordering::SeqCst);
        }
        epoch
    }

    /// Exit critical section
    pub fn exit_epoch(&self, thread_id: usize) {
        if thread_id < self.thread_epochs.len() {
            self.thread_epochs[thread_id].store(u64::MAX, Ordering::SeqCst);
        }
    }

    /// Add garbage to the appropriate epoch list for deferred reclamation
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - `ptr` is a valid pointer that was allocated
    /// - `drop_fn` correctly deallocates the object
    /// - The object is no longer accessible through any strong reference
    pub unsafe fn defer_free<T>(&self, ptr: *mut T) {
        let epoch = self.global_epoch.load(Ordering::SeqCst);
        let epoch_idx = (epoch % 3) as usize;

        self.garbage_lists[epoch_idx].push(GarbageItem {
            ptr: ptr as *mut u8,
            size: std::mem::size_of::<T>(),
            drop_fn: |p| {
                let _ = Box::from_raw(p as *mut T);
            },
        });
    }

    /// Add garbage with a custom drop function
    pub fn defer_free_with_drop(&self, ptr: *mut u8, size: usize, drop_fn: unsafe fn(*mut u8)) {
        let epoch = self.global_epoch.load(Ordering::SeqCst);
        let epoch_idx = (epoch % 3) as usize;

        self.garbage_lists[epoch_idx].push(GarbageItem { ptr, size, drop_fn });
    }

    /// Try to advance epoch and reclaim garbage
    ///
    /// Returns the number of objects reclaimed.
    pub fn try_collect(&self) -> usize {
        let current = self.global_epoch.load(Ordering::SeqCst);

        // Find the minimum epoch that any thread is in
        let min_epoch = self
            .thread_epochs
            .iter()
            .take(self.thread_count.load(Ordering::SeqCst))
            .map(|e| e.load(Ordering::SeqCst))
            .min()
            .unwrap_or(u64::MAX);

        // We can only reclaim garbage from epochs that all threads have passed
        // With 3 epoch lists, we can reclaim from 2 epochs ago
        if min_epoch > current.saturating_sub(2) {
            // Safe to reclaim garbage from 2 epochs ago
            let reclaim_epoch = current.saturating_sub(2);
            let reclaim_idx = (reclaim_epoch % 3) as usize;

            let mut reclaimed = 0;
            while let Some(item) = self.garbage_lists[reclaim_idx].pop() {
                unsafe {
                    (item.drop_fn)(item.ptr);
                }
                reclaimed += 1;
            }

            // Advance epoch
            self.global_epoch.fetch_add(1, Ordering::SeqCst);

            reclaimed
        } else {
            0
        }
    }

    /// Force collection of all garbage (for shutdown)
    ///
    /// # Safety
    /// This should only be called when no threads are accessing GC-managed objects.
    pub unsafe fn force_collect_all(&self) -> usize {
        let mut total = 0;

        for list in &self.garbage_lists {
            while let Some(item) = list.pop() {
                (item.drop_fn)(item.ptr);
                total += 1;
            }
        }

        total
    }

    /// Get the current global epoch
    pub fn current_epoch(&self) -> u64 {
        self.global_epoch.load(Ordering::SeqCst)
    }

    /// Get the number of items waiting for collection
    pub fn pending_count(&self) -> usize {
        self.garbage_lists.iter().map(|l| l.len()).sum()
    }
}

impl Default for EpochGc {
    fn default() -> Self {
        Self::new(64) // Default to 64 threads
    }
}

/// RAII guard for epoch entry/exit
pub struct EpochGuard<'a> {
    gc: &'a EpochGc,
    thread_id: usize,
}

impl<'a> EpochGuard<'a> {
    /// Create a new epoch guard
    pub fn new(gc: &'a EpochGc, thread_id: usize) -> Self {
        gc.enter_epoch(thread_id);
        Self { gc, thread_id }
    }
}

impl<'a> Drop for EpochGuard<'a> {
    fn drop(&mut self) {
        self.gc.exit_epoch(self.thread_id);
    }
}

/// Thread-local GC handle for convenient access
pub struct GcHandle {
    gc: Arc<EpochGc>,
    thread_id: usize,
}

impl GcHandle {
    /// Create a new GC handle for the current thread
    pub fn new(gc: Arc<EpochGc>) -> Option<Self> {
        let thread_id = gc.register_thread()?;
        Some(Self { gc, thread_id })
    }

    /// Enter an epoch and return a guard
    pub fn enter(&self) -> EpochGuard<'_> {
        EpochGuard::new(&self.gc, self.thread_id)
    }

    /// Defer freeing an object
    ///
    /// # Safety
    /// See `EpochGc::defer_free`
    pub unsafe fn defer_free<T>(&self, ptr: *mut T) {
        self.gc.defer_free(ptr);
    }

    /// Try to collect garbage
    pub fn try_collect(&self) -> usize {
        self.gc.try_collect()
    }
}

impl Drop for GcHandle {
    fn drop(&mut self) {
        self.gc.unregister_thread(self.thread_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::thread;

    #[test]
    fn test_epoch_gc_basic() {
        let gc = EpochGc::new(4);

        let thread_id = gc.register_thread().unwrap();

        // Enter epoch
        let epoch = gc.enter_epoch(thread_id);
        assert_eq!(epoch, 0);

        // Exit epoch
        gc.exit_epoch(thread_id);

        gc.unregister_thread(thread_id);
    }

    #[test]
    fn test_defer_free() {
        static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

        struct TestObj;
        impl Drop for TestObj {
            fn drop(&mut self) {
                DROP_COUNT.fetch_add(1, Ordering::SeqCst);
            }
        }

        let gc = EpochGc::new(4);
        let thread_id = gc.register_thread().unwrap();

        // Allocate and defer free
        let obj = Box::into_raw(Box::new(TestObj));
        unsafe { gc.defer_free(obj) };

        assert_eq!(gc.pending_count(), 1);

        // Exit epoch and advance
        gc.exit_epoch(thread_id);

        // Need to advance epoch multiple times to trigger collection
        for _ in 0..3 {
            gc.try_collect();
        }

        // Object should be dropped
        assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1);

        gc.unregister_thread(thread_id);
    }

    #[test]
    fn test_concurrent_epochs() {
        let gc = Arc::new(EpochGc::new(8));
        let mut handles = vec![];

        for _ in 0..4 {
            let gc_clone = Arc::clone(&gc);
            handles.push(thread::spawn(move || {
                let thread_id = gc_clone.register_thread().unwrap();

                for _ in 0..100 {
                    let _guard = EpochGuard::new(&gc_clone, thread_id);
                    // Simulate some work
                    std::hint::spin_loop();
                }

                gc_clone.unregister_thread(thread_id);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_gc_handle() {
        let gc = Arc::new(EpochGc::new(4));
        let handle = GcHandle::new(Arc::clone(&gc)).unwrap();

        {
            let _guard = handle.enter();
            // Do some work in the epoch
        }

        handle.try_collect();
    }
}
