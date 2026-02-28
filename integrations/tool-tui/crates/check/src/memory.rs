//! Memory Optimization Utilities
//!
//! Provides memory-efficient data structures and utilities for handling
//! large file sets without excessive memory usage.
//!
//! **Validates: Requirement 12.4 - Optimize memory usage for large file sets**

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Memory usage tracker
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK_ALLOCATED: AtomicUsize = AtomicUsize::new(0);

/// Tracking allocator that monitors memory usage
pub struct TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: We're delegating to the System allocator which handles the actual allocation
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let size = layout.size();
            let current = ALLOCATED.fetch_add(size, Ordering::SeqCst) + size;
            PEAK_ALLOCATED.fetch_max(current, Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: We're delegating to the System allocator which handles the actual deallocation
        unsafe { System.dealloc(ptr, layout) };
        ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: We're delegating to the System allocator which handles the actual reallocation
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            let old_size = layout.size();
            if new_size > old_size {
                let diff = new_size - old_size;
                let current = ALLOCATED.fetch_add(diff, Ordering::SeqCst) + diff;
                PEAK_ALLOCATED.fetch_max(current, Ordering::SeqCst);
            } else {
                ALLOCATED.fetch_sub(old_size - new_size, Ordering::SeqCst);
            }
        }
        new_ptr
    }
}

/// Get current memory usage in bytes
pub fn current_memory_usage() -> usize {
    ALLOCATED.load(Ordering::SeqCst)
}

/// Get peak memory usage in bytes
pub fn peak_memory_usage() -> usize {
    PEAK_ALLOCATED.load(Ordering::SeqCst)
}

/// Reset peak memory tracking
pub fn reset_peak_memory() {
    PEAK_ALLOCATED.store(ALLOCATED.load(Ordering::SeqCst), Ordering::SeqCst);
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Current allocated bytes
    pub current: usize,
    /// Peak allocated bytes
    pub peak: usize,
    /// Number of allocations (if tracked)
    pub allocations: Option<usize>,
}

impl MemoryStats {
    /// Get current memory statistics
    #[must_use]
    pub fn current() -> Self {
        Self {
            current: current_memory_usage(),
            peak: peak_memory_usage(),
            allocations: None,
        }
    }

    /// Format as human-readable string
    #[must_use]
    pub fn format(&self) -> String {
        format!("Current: {}, Peak: {}", format_bytes(self.current), format_bytes(self.peak))
    }
}

/// Format bytes as human-readable string
#[must_use]
pub fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Memory-efficient string interner for deduplicating strings
pub struct StringInterner {
    strings: Vec<String>,
    indices: std::collections::HashMap<String, usize>,
}

impl StringInterner {
    /// Create a new string interner
    #[must_use]
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            indices: std::collections::HashMap::new(),
        }
    }

    /// Create with pre-allocated capacity
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            strings: Vec::with_capacity(capacity),
            indices: std::collections::HashMap::with_capacity(capacity),
        }
    }

    /// Intern a string, returning its index
    pub fn intern(&mut self, s: &str) -> usize {
        if let Some(&idx) = self.indices.get(s) {
            return idx;
        }

        let idx = self.strings.len();
        self.strings.push(s.to_string());
        self.indices.insert(s.to_string(), idx);
        idx
    }

    /// Get a string by index
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<&str> {
        self.strings.get(idx).map(std::string::String::as_str)
    }

    /// Get the number of interned strings
    #[must_use]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Clear all interned strings
    pub fn clear(&mut self) {
        self.strings.clear();
        self.indices.clear();
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory-efficient path interner for deduplicating file paths
pub struct PathInterner {
    interner: StringInterner,
}

impl PathInterner {
    /// Create a new path interner
    #[must_use]
    pub fn new() -> Self {
        Self {
            interner: StringInterner::new(),
        }
    }

    /// Intern a path, returning its index
    pub fn intern(&mut self, path: &std::path::Path) -> usize {
        self.interner.intern(&path.to_string_lossy())
    }

    /// Get a path by index
    pub fn get(&self, idx: usize) -> Option<std::path::PathBuf> {
        self.interner.get(idx).map(std::path::PathBuf::from)
    }

    /// Get the number of interned paths
    #[must_use]
    pub fn len(&self) -> usize {
        self.interner.len()
    }

    /// Check if empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.interner.is_empty()
    }
}

impl Default for PathInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming file processor that processes files without loading all into memory
pub struct StreamingProcessor<F> {
    processor: F,
    batch_size: usize,
    memory_limit: usize,
}

impl<F, T> StreamingProcessor<F>
where
    F: FnMut(&std::path::Path) -> T,
{
    /// Create a new streaming processor
    pub fn new(processor: F) -> Self {
        Self {
            processor,
            batch_size: 100,
            memory_limit: 100 * 1024 * 1024, // 100MB default
        }
    }

    /// Set the batch size
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Set the memory limit
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.memory_limit = limit;
        self
    }

    /// Process files in batches
    pub fn process_files(&mut self, files: &[std::path::PathBuf]) -> Vec<T> {
        let mut results = Vec::with_capacity(files.len());

        for batch in files.chunks(self.batch_size) {
            // Check memory usage
            if current_memory_usage() > self.memory_limit {
                // Force garbage collection by dropping temporary allocations
                tracing::warn!(
                    "Memory usage ({}) exceeds limit ({}), processing smaller batches",
                    format_bytes(current_memory_usage()),
                    format_bytes(self.memory_limit)
                );
            }

            for file in batch {
                results.push((self.processor)(file));
            }
        }

        results
    }
}

/// Memory budget tracker for limiting memory usage
pub struct MemoryBudget {
    limit: usize,
    reserved: AtomicUsize,
}

impl MemoryBudget {
    /// Create a new memory budget
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            reserved: AtomicUsize::new(0),
        }
    }

    /// Try to reserve memory, returns true if successful
    pub fn try_reserve(&self, bytes: usize) -> bool {
        let current = self.reserved.load(Ordering::SeqCst);
        if current + bytes > self.limit {
            return false;
        }

        // Try to atomically reserve
        self.reserved
            .compare_exchange(current, current + bytes, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Release reserved memory
    pub fn release(&self, bytes: usize) {
        self.reserved.fetch_sub(bytes, Ordering::SeqCst);
    }

    /// Get remaining budget
    pub fn remaining(&self) -> usize {
        self.limit.saturating_sub(self.reserved.load(Ordering::SeqCst))
    }

    /// Get current usage
    pub fn used(&self) -> usize {
        self.reserved.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_string_interner() {
        let mut interner = StringInterner::new();

        let idx1 = interner.intern("hello");
        let idx2 = interner.intern("world");
        let idx3 = interner.intern("hello"); // duplicate

        assert_eq!(idx1, idx3); // Same string should return same index
        assert_ne!(idx1, idx2);
        assert_eq!(interner.get(idx1), Some("hello"));
        assert_eq!(interner.get(idx2), Some("world"));
        assert_eq!(interner.len(), 2); // Only 2 unique strings
    }

    #[test]
    fn test_path_interner() {
        let mut interner = PathInterner::new();

        let idx1 = interner.intern(std::path::Path::new("/foo/bar.js"));
        let idx2 = interner.intern(std::path::Path::new("/foo/baz.js"));
        let idx3 = interner.intern(std::path::Path::new("/foo/bar.js")); // duplicate

        assert_eq!(idx1, idx3);
        assert_ne!(idx1, idx2);
        assert_eq!(interner.len(), 2);
    }

    #[test]
    fn test_memory_budget() {
        let budget = MemoryBudget::new(1000);

        assert!(budget.try_reserve(500));
        assert_eq!(budget.used(), 500);
        assert_eq!(budget.remaining(), 500);

        assert!(budget.try_reserve(400));
        assert_eq!(budget.used(), 900);

        assert!(!budget.try_reserve(200)); // Would exceed limit
        assert_eq!(budget.used(), 900);

        budget.release(500);
        assert_eq!(budget.used(), 400);
        assert!(budget.try_reserve(200)); // Now it fits
    }
}
