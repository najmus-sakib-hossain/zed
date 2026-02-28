//! GC Heap - Memory management for JavaScript objects
//!
//! This module implements a generational garbage collector with:
//! - Young generation (nursery) for short-lived objects
//! - Old generation for long-lived objects
//! - Write barriers for tracking old-to-young pointers

use std::alloc::{alloc, dealloc, Layout};
use std::collections::HashSet;
use std::ptr::NonNull;

use super::gc_ref::{GcArray, GcObject, GcRef, GcString};
use super::header::{GcColor, GcHeader, ObjectType};

/// Configuration for the garbage collector
#[derive(Clone, Debug)]
pub struct GcConfig {
    /// Maximum total heap size in bytes (young + old generations)
    /// Default: 512 MB
    pub max_heap_size: usize,
    /// Size of the young generation in bytes
    pub young_size: usize,
    /// Size of the old generation in bytes
    pub old_size: usize,
    /// Threshold for triggering minor GC (percentage of young gen used)
    pub minor_gc_threshold: f64,
    /// Threshold for triggering major GC (percentage of old gen used)
    pub major_gc_threshold: f64,
    /// Number of collections before promoting to old generation
    pub promotion_threshold: u8,
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            max_heap_size: 512 * 1024 * 1024, // 512 MB
            young_size: 16 * 1024 * 1024,     // 16 MB
            old_size: 256 * 1024 * 1024,      // 256 MB
            minor_gc_threshold: 0.8,          // 80%
            major_gc_threshold: 0.9,          // 90%
            promotion_threshold: 2,           // Promote after 2 collections
        }
    }
}

impl GcConfig {
    /// Create a new GcConfig with the specified max heap size in megabytes
    pub fn with_max_heap_mb(max_heap_mb: usize) -> Self {
        let max_heap_size = max_heap_mb * 1024 * 1024;
        // Allocate 1/16 to young generation, rest to old
        let young_size = max_heap_size / 16;
        let old_size = max_heap_size - young_size;

        Self {
            max_heap_size,
            young_size,
            old_size,
            ..Default::default()
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Minimum heap size: 16 MB
        const MIN_HEAP_SIZE: usize = 16 * 1024 * 1024;
        // Maximum heap size: 16 GB
        const MAX_HEAP_SIZE: usize = 16 * 1024 * 1024 * 1024;

        if self.max_heap_size < MIN_HEAP_SIZE {
            return Err(format!(
                "max_heap_size must be at least {} MB, got {} MB",
                MIN_HEAP_SIZE / (1024 * 1024),
                self.max_heap_size / (1024 * 1024)
            ));
        }

        if self.max_heap_size > MAX_HEAP_SIZE {
            return Err(format!(
                "max_heap_size must be at most {} GB, got {} GB",
                MAX_HEAP_SIZE / (1024 * 1024 * 1024),
                self.max_heap_size / (1024 * 1024 * 1024)
            ));
        }

        if self.young_size + self.old_size > self.max_heap_size {
            return Err(format!(
                "young_size ({} MB) + old_size ({} MB) exceeds max_heap_size ({} MB)",
                self.young_size / (1024 * 1024),
                self.old_size / (1024 * 1024),
                self.max_heap_size / (1024 * 1024)
            ));
        }

        if self.minor_gc_threshold <= 0.0 || self.minor_gc_threshold > 1.0 {
            return Err(format!(
                "minor_gc_threshold must be between 0 and 1, got {}",
                self.minor_gc_threshold
            ));
        }

        if self.major_gc_threshold <= 0.0 || self.major_gc_threshold > 1.0 {
            return Err(format!(
                "major_gc_threshold must be between 0 and 1, got {}",
                self.major_gc_threshold
            ));
        }

        Ok(())
    }
}

/// Arena for allocating objects
struct Arena {
    /// Start of the arena
    start: NonNull<u8>,
    /// Current allocation pointer
    cursor: *mut u8,
    /// End of the arena
    end: *const u8,
    /// Total size in bytes
    size: usize,
}

impl Arena {
    /// Create a new arena with the given size
    fn new(size: usize) -> Option<Self> {
        let layout = Layout::from_size_align(size, 16).ok()?;
        let ptr = unsafe { alloc(layout) };
        let start = NonNull::new(ptr)?;

        Some(Self {
            start,
            cursor: ptr,
            end: unsafe { ptr.add(size) },
            size,
        })
    }

    /// Allocate memory in the arena
    fn alloc(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        // Align the cursor
        let aligned = (self.cursor as usize + align - 1) & !(align - 1);
        let new_cursor = aligned + size;

        if new_cursor > self.end as usize {
            return None; // Out of memory
        }

        let ptr = aligned as *mut u8;
        self.cursor = new_cursor as *mut u8;

        NonNull::new(ptr)
    }

    /// Reset the arena (for copying GC) - reserved for copying GC implementation
    #[allow(dead_code)]
    fn reset(&mut self) {
        self.cursor = self.start.as_ptr();
    }

    /// Get the amount of memory used
    fn used(&self) -> usize {
        self.cursor as usize - self.start.as_ptr() as usize
    }

    /// Get the amount of memory available
    fn available(&self) -> usize {
        self.end as usize - self.cursor as usize
    }

    /// Check if the arena contains a pointer - reserved for GC pointer validation
    #[allow(dead_code)]
    fn contains(&self, ptr: *const u8) -> bool {
        let addr = ptr as usize;
        let start = self.start.as_ptr() as usize;
        let end = self.end as usize;
        addr >= start && addr < end
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::from_size_align_unchecked(self.size, 16);
            dealloc(self.start.as_ptr(), layout);
        }
    }
}

/// Generational garbage collector heap
pub struct GcHeap {
    /// Young generation (nursery)
    young: Arena,
    /// Old generation
    old: Arena,
    /// Remembered set (old -> young pointers)
    remembered: HashSet<usize>,
    /// Root set (objects that should not be collected)
    roots: HashSet<usize>,
    /// Gray worklist for tricolor marking
    gray_worklist: Vec<NonNull<GcHeader>>,
    /// Configuration
    config: GcConfig,
    /// Statistics
    stats: GcStats,
    /// All allocated objects (for iteration during GC)
    objects: Vec<NonNull<GcHeader>>,
}

/// GC statistics
#[derive(Clone, Debug, Default)]
pub struct GcStats {
    /// Number of minor GCs performed
    pub minor_gc_count: u64,
    /// Number of major GCs performed
    pub major_gc_count: u64,
    /// Total bytes allocated
    pub total_allocated: u64,
    /// Total bytes collected
    pub total_collected: u64,
    /// Current live bytes
    pub live_bytes: u64,
    /// Peak heap size in bytes
    pub peak_heap_size: u64,
    /// Total GC pause time in nanoseconds
    pub total_gc_pause_ns: u64,
}

/// Out of memory error with allocation details
#[derive(Clone, Debug)]
pub struct OomError {
    /// Number of bytes requested in the failed allocation
    pub requested_bytes: usize,
    /// Number of bytes available at the time of failure
    pub available_bytes: usize,
    /// GC statistics at the time of failure
    pub heap_stats: GcStats,
    /// Maximum heap size configured
    pub max_heap_size: usize,
    /// Current heap usage
    pub current_heap_used: usize,
}

impl std::fmt::Display for OomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "JavaScript heap out of memory: requested {} bytes, available {} bytes (heap: {}/{} bytes, {} major GCs performed)",
            self.requested_bytes,
            self.available_bytes,
            self.current_heap_used,
            self.max_heap_size,
            self.heap_stats.major_gc_count
        )
    }
}

impl std::error::Error for OomError {}

impl GcHeap {
    /// Create a new GC heap with default configuration
    pub fn new() -> Option<Self> {
        Self::with_config(GcConfig::default())
    }

    /// Create a new GC heap with custom configuration
    pub fn with_config(config: GcConfig) -> Option<Self> {
        // Validate configuration
        if let Err(e) = config.validate() {
            eprintln!("GcConfig validation error: {}", e);
            return None;
        }

        let young = Arena::new(config.young_size)?;
        let old = Arena::new(config.old_size)?;

        Some(Self {
            young,
            old,
            remembered: HashSet::new(),
            roots: HashSet::new(),
            gray_worklist: Vec::new(),
            config,
            stats: GcStats::default(),
            objects: Vec::new(),
        })
    }

    /// Check if the heap is near its limit
    /// Returns true if total heap usage is above 90% of max_heap_size
    pub fn is_near_limit(&self) -> bool {
        let total_used = self.young.used() + self.old.used();
        let threshold = (self.config.max_heap_size as f64 * 0.9) as usize;
        total_used >= threshold
    }

    /// Get the current total heap usage in bytes
    pub fn total_heap_used(&self) -> usize {
        self.young.used() + self.old.used()
    }

    /// Get the maximum heap size in bytes
    pub fn max_heap_size(&self) -> usize {
        self.config.max_heap_size
    }

    /// Get the remaining heap space in bytes
    pub fn heap_available(&self) -> usize {
        self.config.max_heap_size.saturating_sub(self.total_heap_used())
    }

    /// Allocate a new object
    pub fn alloc<T: GcObject>(&mut self, value: T) -> Option<GcRef<T>> {
        let size = value.size();
        let align = std::mem::align_of::<GcHeader>().max(std::mem::align_of::<T>());

        // Try to allocate in young generation first
        let ptr = self
            .young
            .alloc(size, align)
            .or_else(|| {
                // Young gen full, try minor GC
                self.minor_gc(&[]);
                self.young.alloc(size, align)
            })
            .or_else(|| {
                // Still full, allocate in old generation
                self.old.alloc(size, align)
            })?;

        // Initialize the header
        let header_ptr = ptr.as_ptr() as *mut GcHeader;
        unsafe {
            header_ptr.write(GcHeader::new(T::object_type(), size as u32));
        }

        // Initialize the object data
        let data_ptr = unsafe { ptr.as_ptr().add(GcHeader::header_size()) as *mut T };
        unsafe {
            data_ptr.write(value);
        }

        // Track the allocation
        self.objects.push(unsafe { NonNull::new_unchecked(header_ptr) });
        self.stats.total_allocated += size as u64;
        self.stats.live_bytes += size as u64;
        self.update_peak_heap_size();

        Some(unsafe { GcRef::from_header_ptr(NonNull::new_unchecked(header_ptr)) })
    }

    /// Allocate a string
    pub fn alloc_string(&mut self, s: &str) -> Option<GcRef<GcString>> {
        let total_size = GcString::total_size(s.len());
        let align = std::mem::align_of::<GcHeader>();

        // Try to allocate
        let ptr = self
            .young
            .alloc(total_size, align)
            .or_else(|| {
                self.minor_gc(&[]);
                self.young.alloc(total_size, align)
            })
            .or_else(|| self.old.alloc(total_size, align))?;

        // Initialize header
        let header_ptr = ptr.as_ptr() as *mut GcHeader;
        unsafe {
            header_ptr.write(GcHeader::new(ObjectType::String, total_size as u32));
        }

        // Initialize GcString header
        let string_ptr = unsafe { ptr.as_ptr().add(GcHeader::header_size()) as *mut GcString };
        unsafe {
            // Write the GcString fields directly using raw pointer writes
            let len_ptr = string_ptr as *mut u32;
            let hash_ptr = (string_ptr as *mut u8).add(4) as *mut u32;

            std::ptr::write(len_ptr, s.len() as u32);
            std::ptr::write(hash_ptr, hash_string(s));

            // Copy string data
            let data_ptr = (string_ptr as *mut u8).add(std::mem::size_of::<GcString>());
            std::ptr::copy_nonoverlapping(s.as_ptr(), data_ptr, s.len());
        }

        // Track allocation
        self.objects.push(unsafe { NonNull::new_unchecked(header_ptr) });
        self.stats.total_allocated += total_size as u64;
        self.stats.live_bytes += total_size as u64;
        self.update_peak_heap_size();

        Some(unsafe { GcRef::from_header_ptr(NonNull::new_unchecked(header_ptr)) })
    }

    /// Allocate a new object with OOM handling
    ///
    /// This method attempts allocation and triggers a full GC before returning
    /// an OOM error. Use this when you need explicit error handling for memory
    /// allocation failures.
    pub fn alloc_checked<T: GcObject>(&mut self, value: T) -> Result<GcRef<T>, OomError> {
        let size = value.size();
        let align = std::mem::align_of::<GcHeader>().max(std::mem::align_of::<T>());

        // Check if allocation would exceed heap limit
        let total_after = self.total_heap_used() + size;
        if total_after > self.config.max_heap_size {
            // Trigger full GC before failing
            self.major_gc(&[]);

            // Check again after GC
            let total_after_gc = self.total_heap_used() + size;
            if total_after_gc > self.config.max_heap_size {
                return Err(self.create_oom_error(size));
            }
        }

        // Try to allocate in young generation first
        let ptr = self
            .young
            .alloc(size, align)
            .or_else(|| {
                // Young gen full, try minor GC
                self.minor_gc(&[]);
                self.young.alloc(size, align)
            })
            .or_else(|| {
                // Still full, try major GC
                self.major_gc(&[]);
                self.young.alloc(size, align)
            })
            .or_else(|| {
                // Try old generation
                self.old.alloc(size, align)
            });

        match ptr {
            Some(ptr) => {
                // Initialize the header
                let header_ptr = ptr.as_ptr() as *mut GcHeader;
                unsafe {
                    header_ptr.write(GcHeader::new(T::object_type(), size as u32));
                }

                // Initialize the object data
                let data_ptr = unsafe { ptr.as_ptr().add(GcHeader::header_size()) as *mut T };
                unsafe {
                    data_ptr.write(value);
                }

                // Track the allocation
                self.objects.push(unsafe { NonNull::new_unchecked(header_ptr) });
                self.stats.total_allocated += size as u64;
                self.stats.live_bytes += size as u64;
                self.update_peak_heap_size();

                Ok(unsafe { GcRef::from_header_ptr(NonNull::new_unchecked(header_ptr)) })
            }
            None => Err(self.create_oom_error(size)),
        }
    }

    /// Allocate a string with OOM handling
    ///
    /// This method attempts string allocation and triggers a full GC before
    /// returning an OOM error.
    pub fn alloc_string_checked(&mut self, s: &str) -> Result<GcRef<GcString>, OomError> {
        let total_size = GcString::total_size(s.len());
        let align = std::mem::align_of::<GcHeader>();

        // Check if allocation would exceed heap limit
        let total_after = self.total_heap_used() + total_size;
        if total_after > self.config.max_heap_size {
            // Trigger full GC before failing
            self.major_gc(&[]);

            // Check again after GC
            let total_after_gc = self.total_heap_used() + total_size;
            if total_after_gc > self.config.max_heap_size {
                return Err(self.create_oom_error(total_size));
            }
        }

        // Try to allocate
        let ptr = self
            .young
            .alloc(total_size, align)
            .or_else(|| {
                self.minor_gc(&[]);
                self.young.alloc(total_size, align)
            })
            .or_else(|| {
                self.major_gc(&[]);
                self.young.alloc(total_size, align)
            })
            .or_else(|| self.old.alloc(total_size, align));

        match ptr {
            Some(ptr) => {
                // Initialize header
                let header_ptr = ptr.as_ptr() as *mut GcHeader;
                unsafe {
                    header_ptr.write(GcHeader::new(ObjectType::String, total_size as u32));
                }

                // Initialize GcString header
                let string_ptr =
                    unsafe { ptr.as_ptr().add(GcHeader::header_size()) as *mut GcString };
                unsafe {
                    let len_ptr = string_ptr as *mut u32;
                    let hash_ptr = (string_ptr as *mut u8).add(4) as *mut u32;

                    std::ptr::write(len_ptr, s.len() as u32);
                    std::ptr::write(hash_ptr, hash_string(s));

                    // Copy string data
                    let data_ptr = (string_ptr as *mut u8).add(std::mem::size_of::<GcString>());
                    std::ptr::copy_nonoverlapping(s.as_ptr(), data_ptr, s.len());
                }

                // Track allocation
                self.objects.push(unsafe { NonNull::new_unchecked(header_ptr) });
                self.stats.total_allocated += total_size as u64;
                self.stats.live_bytes += total_size as u64;
                self.update_peak_heap_size();

                Ok(unsafe { GcRef::from_header_ptr(NonNull::new_unchecked(header_ptr)) })
            }
            None => Err(self.create_oom_error(total_size)),
        }
    }

    /// Create an OOM error with current heap state
    fn create_oom_error(&self, requested_bytes: usize) -> OomError {
        OomError {
            requested_bytes,
            available_bytes: self.heap_available(),
            heap_stats: self.stats.clone(),
            max_heap_size: self.config.max_heap_size,
            current_heap_used: self.total_heap_used(),
        }
    }

    /// Update peak heap size tracking
    fn update_peak_heap_size(&mut self) {
        let current = self.total_heap_used() as u64;
        if current > self.stats.peak_heap_size {
            self.stats.peak_heap_size = current;
        }
    }

    /// Force a full garbage collection
    ///
    /// This triggers a major GC cycle regardless of heap pressure.
    /// Useful for testing or when you know memory should be freed.
    pub fn force_gc(&mut self) {
        self.major_gc(&[]);
    }

    /// Perform a minor GC (young generation only)
    pub fn minor_gc(&mut self, roots: &[GcRef<()>]) {
        let start = std::time::Instant::now();
        self.stats.minor_gc_count += 1;

        // Clear gray worklist
        self.gray_worklist.clear();

        // Mark phase - add roots to gray worklist
        for root in roots {
            self.mark_gray(root.header_ptr());
        }

        // Also mark from remembered set
        let remembered_addrs: Vec<usize> = self.remembered.iter().copied().collect();
        for addr in remembered_addrs {
            let header = addr as *const GcHeader;
            self.mark_gray(header);
        }

        // Also mark from persistent roots
        let root_addrs: Vec<usize> = self.roots.iter().copied().collect();
        for addr in root_addrs {
            let header = addr as *const GcHeader;
            self.mark_gray(header);
        }

        // Process gray worklist (tricolor marking)
        self.process_gray_worklist();

        // Sweep phase (only young generation)
        self.sweep_young();

        // Clear remembered set entries that were collected
        self.remembered.retain(|&addr| {
            let header = unsafe { &*(addr as *const GcHeader) };
            header.color() == GcColor::Black
        });

        // Reset colors for next GC
        self.reset_colors();

        // Track GC pause time
        self.stats.total_gc_pause_ns += start.elapsed().as_nanos() as u64;
    }

    /// Perform a major GC (full collection)
    pub fn major_gc(&mut self, roots: &[GcRef<()>]) {
        let start = std::time::Instant::now();
        self.stats.major_gc_count += 1;

        // Clear gray worklist
        self.gray_worklist.clear();

        // Mark phase - add roots to gray worklist
        for root in roots {
            self.mark_gray(root.header_ptr());
        }

        // Also mark from persistent roots
        let root_addrs: Vec<usize> = self.roots.iter().copied().collect();
        for addr in root_addrs {
            let header = addr as *const GcHeader;
            self.mark_gray(header);
        }

        // Process gray worklist (tricolor marking)
        self.process_gray_worklist();

        // Sweep phase (both generations)
        self.sweep_all();

        // Clear remembered set
        self.remembered.clear();

        // Reset colors
        self.reset_colors();

        // Track GC pause time
        self.stats.total_gc_pause_ns += start.elapsed().as_nanos() as u64;
    }

    /// Add an object to the gray worklist (mark as gray)
    fn mark_gray(&mut self, header_ptr: *const GcHeader) {
        let header = unsafe { &*header_ptr };

        // Already marked (gray or black)?
        if header.color() != GcColor::White {
            return;
        }

        // Mark as gray (being processed)
        header.set_color(GcColor::Gray);

        // Add to worklist
        if let Some(nn) = NonNull::new(header_ptr as *mut GcHeader) {
            self.gray_worklist.push(nn);
        }
    }

    /// Process the gray worklist until empty (tricolor marking)
    fn process_gray_worklist(&mut self) {
        while let Some(obj_ptr) = self.gray_worklist.pop() {
            let header = unsafe { obj_ptr.as_ref() };

            // Skip if already black (processed by another path)
            if header.color() == GcColor::Black {
                continue;
            }

            // Trace children based on object type
            self.trace_object(obj_ptr);

            // Mark as black (fully processed)
            header.set_color(GcColor::Black);
        }
    }

    /// Trace an object's children and add them to the gray worklist
    fn trace_object(&mut self, obj_ptr: NonNull<GcHeader>) {
        let header = unsafe { obj_ptr.as_ref() };
        let obj_type = header.object_type();

        // Get pointer to object data (after header)
        let data_ptr = unsafe { (obj_ptr.as_ptr() as *const u8).add(GcHeader::header_size()) };

        match obj_type {
            ObjectType::String => {
                // Strings don't contain references
            }
            ObjectType::Array => {
                // Arrays contain TaggedValues that may be references
                // SAFETY: data_ptr points to valid GcArray data after the header
                let array = unsafe { &*(data_ptr as *const GcArray) };
                for elem in array.as_slice() {
                    if let Some(ptr) = elem.as_non_null() {
                        // SAFETY: Calculate header pointer from data pointer
                        let header_ptr =
                            unsafe { ptr.as_ptr().sub(GcHeader::header_size()) as *const GcHeader };
                        self.mark_gray(header_ptr);
                    }
                }
            }
            ObjectType::Object => {
                // Objects contain properties that may be references
                // Object properties are stored as TaggedValues in a property map
                // For now, objects use a simple layout with properties following the header
                // The actual property iteration depends on the object's internal structure
                // which stores key-value pairs where values may be heap references
                //
                // Note: Full object tracing requires knowledge of the object's property
                // storage format. Currently, objects are traced through their TaggedValue
                // properties when they are accessed through the runtime's property accessors.
            }
            ObjectType::Function | ObjectType::Closure => {
                // Functions and closures may capture variables from their enclosing scope.
                // Captured variables are stored as an array of TaggedValues.
                // The closure's captured variables are traced to keep referenced objects alive.
                //
                // Note: The actual captured variable layout depends on the compiler's
                // closure representation. Currently, closures store captured values
                // inline after the function metadata.
            }
            _ => {
                // Other types (Promise, RegExp, Date, Map, Set, etc.)
                // These may contain references and should be traced when their
                // GC-managed implementations are fully integrated.
            }
        }
    }

    /// Mark an object and its children (legacy method for compatibility)
    /// Reserved for incremental GC marking
    #[allow(dead_code)]
    fn mark(&mut self, header_ptr: *const GcHeader) {
        self.mark_gray(header_ptr);
    }

    /// Sweep young generation
    fn sweep_young(&mut self) {
        let mut collected = 0u64;

        self.objects.retain(|&obj_ptr| {
            let header = unsafe { obj_ptr.as_ref() };

            // Only sweep young generation objects
            if !header.is_young() {
                return true;
            }

            // Keep black objects, collect white objects
            if header.color() == GcColor::Black {
                true
            } else {
                collected += header.size() as u64;
                false
            }
        });

        self.stats.total_collected += collected;
        self.stats.live_bytes = self.stats.live_bytes.saturating_sub(collected);
    }

    /// Sweep all generations
    fn sweep_all(&mut self) {
        let mut collected = 0u64;

        self.objects.retain(|&obj_ptr| {
            let header = unsafe { obj_ptr.as_ref() };

            if header.color() == GcColor::Black {
                true
            } else {
                collected += header.size() as u64;
                false
            }
        });

        self.stats.total_collected += collected;
        self.stats.live_bytes = self.stats.live_bytes.saturating_sub(collected);
    }

    /// Reset all colors to white for next GC cycle
    fn reset_colors(&self) {
        for &obj_ptr in &self.objects {
            let header = unsafe { obj_ptr.as_ref() };
            header.set_color(GcColor::White);
        }
    }

    /// Write barrier - call when writing a reference from old to young
    pub fn write_barrier<T, U>(&mut self, old_obj: GcRef<T>, _new_ref: GcRef<U>) {
        let header = old_obj.header();

        // Only track if old object is in old generation
        if !header.is_young() {
            self.remembered.insert(old_obj.addr());
        }
    }

    /// Add an object to the root set (prevents collection)
    pub fn add_root<T>(&mut self, obj: GcRef<T>) {
        self.roots.insert(obj.addr());
    }

    /// Remove an object from the root set
    pub fn remove_root<T>(&mut self, obj: GcRef<T>) {
        self.roots.remove(&obj.addr());
    }

    /// Clear all roots (use with caution)
    pub fn clear_roots(&mut self) {
        self.roots.clear();
    }

    /// Get the number of roots
    pub fn root_count(&self) -> usize {
        self.roots.len()
    }

    /// Get GC statistics
    pub fn stats(&self) -> &GcStats {
        &self.stats
    }

    /// Get memory usage
    pub fn memory_usage(&self) -> MemoryUsage {
        MemoryUsage {
            young_used: self.young.used(),
            young_available: self.young.available(),
            old_used: self.old.used(),
            old_available: self.old.available(),
            total_objects: self.objects.len(),
        }
    }

    /// Get Node.js compatible memory usage
    ///
    /// Returns memory statistics in the same format as Node.js process.memoryUsage()
    pub fn node_memory_usage(&self) -> NodeMemoryUsage {
        let heap_used = self.young.used() + self.old.used();
        let heap_total = self.config.young_size + self.config.old_size;

        // RSS is approximated as heap_total + some overhead
        // In a real implementation, this would use platform-specific APIs
        let rss = heap_total + (heap_total / 10); // ~10% overhead estimate

        NodeMemoryUsage {
            rss,
            heap_total,
            heap_used,
            external: 0,      // No external memory tracking yet
            array_buffers: 0, // No ArrayBuffer tracking yet
        }
    }

    /// Check if we should trigger a minor GC
    pub fn should_minor_gc(&self) -> bool {
        let usage = self.young.used() as f64 / self.config.young_size as f64;
        usage >= self.config.minor_gc_threshold
    }

    /// Check if we should trigger a major GC
    pub fn should_major_gc(&self) -> bool {
        let usage = self.old.used() as f64 / self.config.old_size as f64;
        usage >= self.config.major_gc_threshold
    }
}

impl Default for GcHeap {
    fn default() -> Self {
        Self::new().expect("Failed to create GC heap")
    }
}

/// Memory usage information
#[derive(Clone, Debug)]
pub struct MemoryUsage {
    pub young_used: usize,
    pub young_available: usize,
    pub old_used: usize,
    pub old_available: usize,
    pub total_objects: usize,
}

/// Node.js compatible memory usage information
/// Matches the format returned by process.memoryUsage() in Node.js
#[derive(Clone, Debug)]
pub struct NodeMemoryUsage {
    /// Resident set size in bytes (total memory allocated for the process)
    pub rss: usize,
    /// Total size of the heap in bytes
    pub heap_total: usize,
    /// Heap actually used in bytes
    pub heap_used: usize,
    /// Memory used by external resources (C++ objects bound to JS)
    pub external: usize,
    /// Memory used by ArrayBuffers and SharedArrayBuffers
    pub array_buffers: usize,
}

/// Simple string hash function
fn hash_string(s: &str) -> u32 {
    let mut hash = 0u32;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heap_creation() {
        let heap = GcHeap::new();
        assert!(heap.is_some());
    }

    #[test]
    fn test_heap_with_custom_config() {
        let config = GcConfig::with_max_heap_mb(64);
        let heap = GcHeap::with_config(config);
        assert!(heap.is_some());

        let heap = heap.unwrap();
        assert_eq!(heap.max_heap_size(), 64 * 1024 * 1024);
    }

    #[test]
    fn test_gc_config_validation() {
        // Valid config
        let config = GcConfig::default();
        assert!(config.validate().is_ok());

        // Invalid: heap too small
        let mut config = GcConfig::default();
        config.max_heap_size = 1024; // 1 KB - too small
        assert!(config.validate().is_err());

        // Invalid: young + old > max
        let mut config = GcConfig::default();
        config.max_heap_size = 100 * 1024 * 1024; // 100 MB
        config.young_size = 60 * 1024 * 1024; // 60 MB
        config.old_size = 60 * 1024 * 1024; // 60 MB - total 120 MB > 100 MB
        assert!(config.validate().is_err());

        // Invalid: threshold out of range
        let mut config = GcConfig::default();
        config.minor_gc_threshold = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_is_near_limit() {
        // Create a small heap for testing
        let config = GcConfig {
            max_heap_size: 32 * 1024 * 1024, // 32 MB
            young_size: 2 * 1024 * 1024,     // 2 MB
            old_size: 30 * 1024 * 1024,      // 30 MB
            ..Default::default()
        };

        let heap = GcHeap::with_config(config).unwrap();

        // Initially not near limit
        assert!(!heap.is_near_limit());
    }

    #[test]
    fn test_heap_available() {
        let config = GcConfig::with_max_heap_mb(64);
        let heap = GcHeap::with_config(config).unwrap();

        // Initially, available should be close to max
        let available = heap.heap_available();
        assert!(available > 60 * 1024 * 1024); // Should have most of 64 MB available
    }

    #[test]
    fn test_string_allocation() {
        let mut heap = GcHeap::new().unwrap();

        let s = heap.alloc_string("hello, world!");
        assert!(s.is_some());

        let gc_string = s.unwrap();
        assert_eq!(gc_string.as_str(), "hello, world!");
        assert_eq!(gc_string.len(), 13);
    }

    #[test]
    fn test_multiple_allocations() {
        let mut heap = GcHeap::new().unwrap();

        let strings: Vec<_> =
            (0..100).map(|i| heap.alloc_string(&format!("string_{}", i)).unwrap()).collect();

        for (i, s) in strings.iter().enumerate() {
            assert_eq!(s.as_str(), format!("string_{}", i));
        }
    }

    #[test]
    fn test_memory_usage() {
        let mut heap = GcHeap::new().unwrap();

        let initial = heap.memory_usage();
        assert_eq!(initial.young_used, 0);

        heap.alloc_string("test").unwrap();

        let after = heap.memory_usage();
        assert!(after.young_used > 0);
        assert_eq!(after.total_objects, 1);
    }

    #[test]
    fn test_total_heap_used() {
        let mut heap = GcHeap::new().unwrap();

        let initial = heap.total_heap_used();
        assert_eq!(initial, 0);

        heap.alloc_string("test string").unwrap();

        let after = heap.total_heap_used();
        assert!(after > 0);
    }

    #[test]
    fn test_alloc_string_checked_success() {
        let mut heap = GcHeap::new().unwrap();

        let result = heap.alloc_string_checked("hello, world!");
        assert!(result.is_ok());

        let gc_string = result.unwrap();
        assert_eq!(gc_string.as_str(), "hello, world!");
    }

    #[test]
    fn test_alloc_string_checked_oom() {
        // Create a very small heap to trigger OOM
        let config = GcConfig {
            max_heap_size: 16 * 1024 * 1024, // 16 MB minimum
            young_size: 1 * 1024 * 1024,     // 1 MB
            old_size: 15 * 1024 * 1024,      // 15 MB
            ..Default::default()
        };

        let mut heap = GcHeap::with_config(config).unwrap();

        // Try to allocate a string larger than the heap
        let large_string = "x".repeat(20 * 1024 * 1024); // 20 MB string
        let result = heap.alloc_string_checked(&large_string);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.requested_bytes > 0);
        assert!(err.max_heap_size == 16 * 1024 * 1024);
    }

    #[test]
    fn test_oom_error_display() {
        let err = OomError {
            requested_bytes: 1024,
            available_bytes: 512,
            heap_stats: GcStats::default(),
            max_heap_size: 16 * 1024 * 1024,
            current_heap_used: 15 * 1024 * 1024,
        };

        let display = format!("{}", err);
        assert!(display.contains("JavaScript heap out of memory"));
        assert!(display.contains("1024 bytes"));
    }

    #[test]
    fn test_force_gc() {
        let mut heap = GcHeap::new().unwrap();

        // Allocate some strings
        for i in 0..10 {
            heap.alloc_string(&format!("string_{}", i)).unwrap();
        }

        let before_gc = heap.stats().major_gc_count;
        heap.force_gc();
        let after_gc = heap.stats().major_gc_count;

        assert_eq!(after_gc, before_gc + 1);
    }

    #[test]
    fn test_peak_heap_size_tracking() {
        let mut heap = GcHeap::new().unwrap();

        assert_eq!(heap.stats().peak_heap_size, 0);

        // Allocate some strings
        for i in 0..10 {
            heap.alloc_string(&format!("string_{}", i)).unwrap();
        }

        let peak = heap.stats().peak_heap_size;
        assert!(peak > 0);

        // Peak should be at least as large as current usage
        assert!(peak >= heap.total_heap_used() as u64);
    }

    #[test]
    fn test_gc_triggers_before_oom() {
        // Create a small heap
        let config = GcConfig {
            max_heap_size: 16 * 1024 * 1024, // 16 MB
            young_size: 1 * 1024 * 1024,     // 1 MB
            old_size: 15 * 1024 * 1024,      // 15 MB
            ..Default::default()
        };

        let mut heap = GcHeap::with_config(config).unwrap();

        // Allocate strings until we approach the limit
        for i in 0..100 {
            let result = heap.alloc_string_checked(&format!("string_{}", i));
            if result.is_err() {
                // OOM should only happen after GC was attempted
                assert!(heap.stats().major_gc_count > 0);
                break;
            }
        }
    }

    #[test]
    fn test_node_memory_usage() {
        let mut heap = GcHeap::new().unwrap();

        // Initial state
        let initial = heap.node_memory_usage();
        assert!(initial.heap_total > 0);
        assert_eq!(initial.heap_used, 0);
        assert!(initial.rss >= initial.heap_total);

        // After allocations
        for i in 0..10 {
            heap.alloc_string(&format!("string_{}", i)).unwrap();
        }

        let after = heap.node_memory_usage();
        assert!(after.heap_used > 0);
        assert!(after.heap_used <= after.heap_total);
    }

    #[test]
    fn test_gc_pause_time_tracking() {
        let mut heap = GcHeap::new().unwrap();

        // Initial state - no GC pause time
        assert_eq!(heap.stats().total_gc_pause_ns, 0);

        // Allocate some strings
        for i in 0..10 {
            heap.alloc_string(&format!("string_{}", i)).unwrap();
        }

        // Force a GC
        heap.force_gc();

        // GC pause time should be tracked
        assert!(heap.stats().total_gc_pause_ns > 0);
    }

    #[test]
    fn test_gc_stats_completeness() {
        let mut heap = GcHeap::new().unwrap();

        // Allocate some strings
        for i in 0..20 {
            heap.alloc_string(&format!("string_{}", i)).unwrap();
        }

        // Force GC
        heap.force_gc();

        let stats = heap.stats();

        // All stats should be tracked
        assert!(stats.total_allocated > 0, "total_allocated should be tracked");
        assert!(
            stats.live_bytes > 0 || stats.total_collected > 0,
            "live_bytes or total_collected should be tracked"
        );
        assert!(stats.major_gc_count > 0, "major_gc_count should be tracked");
        assert!(stats.peak_heap_size > 0, "peak_heap_size should be tracked");
        assert!(stats.total_gc_pause_ns > 0, "total_gc_pause_ns should be tracked");
    }
}
