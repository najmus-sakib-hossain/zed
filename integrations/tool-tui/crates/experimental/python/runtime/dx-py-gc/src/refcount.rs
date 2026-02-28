//! Lock-free reference counting implementation
//!
//! Uses a 64-bit atomic with:
//! - High 32 bits: strong reference count
//! - Low 32 bits: weak reference count + flags

use std::sync::atomic::{AtomicU64, Ordering};

/// Lock-free reference count (64-bit atomic)
///
/// Layout:
/// - Bits 63-32: Strong reference count
/// - Bit 31: Marked for cycle detection
/// - Bits 30-0: Weak reference count
#[repr(C)]
pub struct LockFreeRefCount {
    count: AtomicU64,
}

impl LockFreeRefCount {
    /// Shift for strong reference count (high 32 bits)
    const STRONG_SHIFT: u64 = 32;

    /// Mask for weak reference count (low 31 bits)
    const WEAK_MASK: u64 = 0x7FFFFFFF;

    /// Bit flag for cycle detection marking
    const MARKED_BIT: u64 = 1 << 31;

    /// Create a new reference count with strong=1, weak=0
    pub fn new() -> Self {
        Self {
            count: AtomicU64::new(1 << Self::STRONG_SHIFT),
        }
    }

    /// Create a reference count with specified initial values
    pub fn with_counts(strong: u32, weak: u32) -> Self {
        let value = ((strong as u64) << Self::STRONG_SHIFT) | (weak as u64 & Self::WEAK_MASK);
        Self {
            count: AtomicU64::new(value),
        }
    }

    /// Increment strong reference count
    ///
    /// This is a relaxed operation as we only need to ensure the increment
    /// is atomic, not that it's visible to other threads immediately.
    #[inline]
    pub fn inc_strong(&self) {
        self.count.fetch_add(1 << Self::STRONG_SHIFT, Ordering::Relaxed);
    }

    /// Decrement strong reference count
    ///
    /// Returns `true` if the object should be deallocated (strong count reached 0).
    /// Uses Release ordering on decrement and Acquire fence before deallocation
    /// to ensure all writes are visible before deallocation.
    #[inline]
    pub fn dec_strong(&self) -> bool {
        let old = self.count.fetch_sub(1 << Self::STRONG_SHIFT, Ordering::Release);
        let strong = old >> Self::STRONG_SHIFT;

        if strong == 1 {
            // Synchronize with all previous decrements
            std::sync::atomic::fence(Ordering::Acquire);
            true // Object should be deallocated
        } else {
            false
        }
    }

    /// Increment weak reference count
    #[inline]
    pub fn inc_weak(&self) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement weak reference count
    ///
    /// Returns `true` if the weak storage should be deallocated
    /// (weak count reached 0 and strong count is already 0).
    #[inline]
    pub fn dec_weak(&self) -> bool {
        let old = self.count.fetch_sub(1, Ordering::Release);
        let weak = old & Self::WEAK_MASK;
        let strong = old >> Self::STRONG_SHIFT;

        weak == 1 && strong == 0
    }

    /// Mark this object for cycle detection
    ///
    /// Returns `true` if this is a new marking (wasn't marked before).
    pub fn mark_for_cycle(&self) -> bool {
        let old = self.count.fetch_or(Self::MARKED_BIT, Ordering::SeqCst);
        (old & Self::MARKED_BIT) == 0
    }

    /// Clear the cycle detection mark
    pub fn unmark(&self) {
        self.count.fetch_and(!Self::MARKED_BIT, Ordering::SeqCst);
    }

    /// Check if marked for cycle detection
    pub fn is_marked(&self) -> bool {
        (self.count.load(Ordering::SeqCst) & Self::MARKED_BIT) != 0
    }

    /// Get the current strong reference count
    #[inline]
    pub fn strong_count(&self) -> u32 {
        (self.count.load(Ordering::Relaxed) >> Self::STRONG_SHIFT) as u32
    }

    /// Get the current weak reference count
    #[inline]
    pub fn weak_count(&self) -> u32 {
        (self.count.load(Ordering::Relaxed) & Self::WEAK_MASK) as u32
    }

    /// Try to upgrade a weak reference to a strong reference
    ///
    /// Returns `true` if successful, `false` if the object is already deallocated.
    pub fn try_upgrade(&self) -> bool {
        let mut current = self.count.load(Ordering::Relaxed);

        loop {
            let strong = current >> Self::STRONG_SHIFT;

            if strong == 0 {
                return false; // Object already deallocated
            }

            let new = current + (1 << Self::STRONG_SHIFT);

            match self.count.compare_exchange_weak(
                current,
                new,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    /// Load the raw count value (for debugging)
    pub fn raw(&self) -> u64 {
        self.count.load(Ordering::SeqCst)
    }
}

impl Default for LockFreeRefCount {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for LockFreeRefCount {
    fn clone(&self) -> Self {
        // Clone creates a new reference count, not a copy of the atomic
        Self::with_counts(self.strong_count(), self.weak_count())
    }
}

impl std::fmt::Debug for LockFreeRefCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LockFreeRefCount")
            .field("strong", &self.strong_count())
            .field("weak", &self.weak_count())
            .field("marked", &self.is_marked())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_new() {
        let rc = LockFreeRefCount::new();
        assert_eq!(rc.strong_count(), 1);
        assert_eq!(rc.weak_count(), 0);
        assert!(!rc.is_marked());
    }

    #[test]
    fn test_inc_dec_strong() {
        let rc = LockFreeRefCount::new();

        rc.inc_strong();
        assert_eq!(rc.strong_count(), 2);

        rc.inc_strong();
        assert_eq!(rc.strong_count(), 3);

        assert!(!rc.dec_strong()); // 3 -> 2
        assert_eq!(rc.strong_count(), 2);

        assert!(!rc.dec_strong()); // 2 -> 1
        assert_eq!(rc.strong_count(), 1);

        assert!(rc.dec_strong()); // 1 -> 0, should deallocate
        assert_eq!(rc.strong_count(), 0);
    }

    #[test]
    fn test_inc_dec_weak() {
        let rc = LockFreeRefCount::new();

        rc.inc_weak();
        assert_eq!(rc.weak_count(), 1);

        rc.inc_weak();
        assert_eq!(rc.weak_count(), 2);

        assert!(!rc.dec_weak()); // Strong is still 1
        assert_eq!(rc.weak_count(), 1);
    }

    #[test]
    fn test_weak_dealloc() {
        let rc = LockFreeRefCount::new();

        rc.inc_weak();
        assert!(rc.dec_strong()); // Strong -> 0
        assert!(rc.dec_weak()); // Weak -> 0, should deallocate weak storage
    }

    #[test]
    fn test_mark_unmark() {
        let rc = LockFreeRefCount::new();

        assert!(!rc.is_marked());
        assert!(rc.mark_for_cycle()); // First mark returns true
        assert!(rc.is_marked());
        assert!(!rc.mark_for_cycle()); // Second mark returns false

        rc.unmark();
        assert!(!rc.is_marked());
    }

    #[test]
    fn test_try_upgrade() {
        let rc = LockFreeRefCount::new();
        rc.inc_weak();

        assert!(rc.try_upgrade());
        assert_eq!(rc.strong_count(), 2);

        // Decrement both strong refs
        rc.dec_strong();
        rc.dec_strong();

        // Now upgrade should fail
        assert!(!rc.try_upgrade());
    }

    #[test]
    fn test_concurrent_inc_dec() {
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
}
