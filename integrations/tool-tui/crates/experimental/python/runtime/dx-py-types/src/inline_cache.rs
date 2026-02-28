//! Monomorphic inline cache for type prediction

use dx_py_jit::profile::PyType;
use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicU8, Ordering};

/// Cache state
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
    /// Not yet initialized
    Uninitialized = 0,
    /// Single type observed
    Monomorphic = 1,
    /// Multiple types observed (2-4)
    Polymorphic = 2,
    /// Too many types, use generic path
    Megamorphic = 3,
}

impl CacheState {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Uninitialized,
            1 => Self::Monomorphic,
            2 => Self::Polymorphic,
            3 => Self::Megamorphic,
            _ => Self::Uninitialized,
        }
    }
}

/// Inline cache for type prediction
///
/// Provides fast path lookup for monomorphic call sites.
#[repr(C)]
pub struct InlineCache {
    /// Cached type (PyType as u8)
    cached_type: AtomicU8,
    /// Cache state
    state: AtomicU8,
    /// Hit count for profiling
    hits: AtomicU32,
    /// Miss count for profiling
    misses: AtomicU32,
    /// Pointer to specialized code
    specialized_code: AtomicPtr<u8>,
}

impl InlineCache {
    /// Create a new uninitialized inline cache
    pub fn new() -> Self {
        Self {
            cached_type: AtomicU8::new(PyType::Unknown as u8),
            state: AtomicU8::new(CacheState::Uninitialized as u8),
            hits: AtomicU32::new(0),
            misses: AtomicU32::new(0),
            specialized_code: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    /// Fast path lookup
    ///
    /// Returns the specialized code pointer if the type matches,
    /// or None if the cache misses.
    #[inline(always)]
    pub fn lookup(&self, obj_type: PyType) -> Option<*const u8> {
        let cached = self.cached_type.load(Ordering::Relaxed);

        if cached == obj_type as u8 {
            self.hits.fetch_add(1, Ordering::Relaxed);
            let code = self.specialized_code.load(Ordering::Acquire);
            if !code.is_null() {
                return Some(code as *const u8);
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Update cache with a new type observation
    pub fn update(&self, obj_type: PyType, code: *const u8) {
        let state = CacheState::from_u8(self.state.load(Ordering::Relaxed));

        match state {
            CacheState::Uninitialized => {
                // First observation - become monomorphic
                self.cached_type.store(obj_type as u8, Ordering::Relaxed);
                self.specialized_code.store(code as *mut u8, Ordering::Release);
                self.state.store(CacheState::Monomorphic as u8, Ordering::Release);
            }
            CacheState::Monomorphic => {
                let cached = self.cached_type.load(Ordering::Relaxed);
                if cached != obj_type as u8 {
                    // Different type - transition to polymorphic
                    self.state.store(CacheState::Polymorphic as u8, Ordering::Release);
                }
            }
            CacheState::Polymorphic | CacheState::Megamorphic => {
                // Already polymorphic/megamorphic, no change
            }
        }
    }

    /// Get the current cache state
    pub fn state(&self) -> CacheState {
        CacheState::from_u8(self.state.load(Ordering::Relaxed))
    }

    /// Get the cached type (if monomorphic)
    pub fn cached_type(&self) -> Option<PyType> {
        if self.state() == CacheState::Monomorphic {
            Some(PyType::from_u8(self.cached_type.load(Ordering::Relaxed)))
        } else {
            None
        }
    }

    /// Get hit count
    pub fn hit_count(&self) -> u32 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Get miss count
    pub fn miss_count(&self) -> u32 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Get hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let misses = self.misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;
        if total > 0.0 {
            hits / total
        } else {
            0.0
        }
    }

    /// Reset the cache
    pub fn reset(&self) {
        self.cached_type.store(PyType::Unknown as u8, Ordering::Relaxed);
        self.state.store(CacheState::Uninitialized as u8, Ordering::Relaxed);
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.specialized_code.store(std::ptr::null_mut(), Ordering::Release);
    }

    /// Transition to megamorphic state
    pub fn go_megamorphic(&self) {
        self.state.store(CacheState::Megamorphic as u8, Ordering::Release);
        self.specialized_code.store(std::ptr::null_mut(), Ordering::Release);
    }
}

impl Default for InlineCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inline_cache_new() {
        let cache = InlineCache::new();
        assert_eq!(cache.state(), CacheState::Uninitialized);
        assert_eq!(cache.hit_count(), 0);
        assert_eq!(cache.miss_count(), 0);
    }

    #[test]
    fn test_inline_cache_monomorphic() {
        let cache = InlineCache::new();
        let code = 0x1234 as *const u8;

        cache.update(PyType::Int, code);

        assert_eq!(cache.state(), CacheState::Monomorphic);
        assert_eq!(cache.cached_type(), Some(PyType::Int));

        // Lookup should hit
        assert_eq!(cache.lookup(PyType::Int), Some(code));
        assert_eq!(cache.hit_count(), 1);

        // Different type should miss
        assert_eq!(cache.lookup(PyType::Float), None);
        assert_eq!(cache.miss_count(), 1);
    }

    #[test]
    fn test_inline_cache_polymorphic_transition() {
        let cache = InlineCache::new();

        cache.update(PyType::Int, std::ptr::null());
        assert_eq!(cache.state(), CacheState::Monomorphic);

        cache.update(PyType::Float, std::ptr::null());
        assert_eq!(cache.state(), CacheState::Polymorphic);
    }

    #[test]
    fn test_hit_rate() {
        let cache = InlineCache::new();
        let code = 0x1234 as *const u8;

        cache.update(PyType::Int, code);

        // 3 hits, 1 miss
        cache.lookup(PyType::Int);
        cache.lookup(PyType::Int);
        cache.lookup(PyType::Int);
        cache.lookup(PyType::Float);

        assert!((cache.hit_rate() - 0.75).abs() < 0.01);
    }
}
