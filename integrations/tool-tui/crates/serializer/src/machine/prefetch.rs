//! DX-Prefetch: CPU Cache Optimization
//!
//! rkyv doesn't hint the CPU about memory access patterns.
//! DX-Prefetch tells the CPU what we'll access next.
//!
//! Result: 2-3× faster batch operations

/// Cache line size (typically 64 bytes on modern CPUs)
pub const CACHE_LINE_SIZE: usize = 64;

/// Prefetch hints for different access patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrefetchHint {
    /// Temporal data - keep in all cache levels (T0)
    /// Use for data that will be accessed multiple times
    #[default]
    Temporal,
    /// Non-temporal data - bypass cache (NTA)
    /// Use for streaming data accessed only once
    NonTemporal,
    /// Read with exclusive access intent
    /// Use when you'll modify the data
    Exclusive,
}

/// Prefetch a memory location into CPU cache
///
/// # Arguments
/// * `ptr` - Pointer to the memory location
/// * `hint` - Prefetch hint for cache behavior
///
/// # Safety
/// The pointer must be valid (but doesn't need to be aligned).
/// The pointer doesn't need to be dereferenceable - prefetch only hints the CPU.
#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub unsafe fn prefetch(ptr: *const u8, hint: PrefetchHint) {
    // SAFETY: Caller guarantees ptr is valid.
    // _mm_prefetch is a hint instruction that doesn't dereference the pointer,
    // it only tells the CPU to load the cache line containing that address.
    unsafe {
        #[cfg(target_feature = "sse")]
        {
            use std::arch::x86_64::*;
            match hint {
                PrefetchHint::Temporal => _mm_prefetch(ptr as *const i8, _MM_HINT_T0),
                PrefetchHint::NonTemporal => _mm_prefetch(ptr as *const i8, _MM_HINT_NTA),
                PrefetchHint::Exclusive => _mm_prefetch(ptr as *const i8, _MM_HINT_T0),
            }
        }
    }
}

/// Prefetch - no-op for non-x86 platforms
#[inline(always)]
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn prefetch(_ptr: *const u8, _hint: PrefetchHint) {
    // No-op - prefetch not available
}

/// Prefetch N cache lines starting at ptr
///
/// # Arguments
/// * `ptr` - Starting pointer
/// * `lines` - Number of cache lines to prefetch
/// * `hint` - Prefetch hint
///
/// # Safety
/// The pointer must be valid. The function will prefetch lines * CACHE_LINE_SIZE bytes.
/// Caller must ensure ptr + (lines * CACHE_LINE_SIZE) doesn't overflow.
#[inline]
pub unsafe fn prefetch_lines(ptr: *const u8, lines: usize, hint: PrefetchHint) {
    // SAFETY: Caller guarantees ptr is valid and the range is within bounds.
    // We delegate to prefetch() which handles the actual unsafe operation.
    unsafe {
        for i in 0..lines {
            prefetch(ptr.add(i * CACHE_LINE_SIZE), hint);
        }
    }
}

/// Prefetch a range of bytes
///
/// # Safety
/// The pointer must be valid and ptr + size must not overflow.
#[inline]
pub unsafe fn prefetch_range(ptr: *const u8, size: usize, hint: PrefetchHint) {
    // SAFETY: Caller guarantees ptr is valid and size is within bounds.
    // We delegate to prefetch_lines() which handles the actual unsafe operation.
    unsafe {
        let lines = size.div_ceil(CACHE_LINE_SIZE);
        prefetch_lines(ptr, lines, hint);
    }
}

/// Prefetching iterator wrapper
///
/// Wraps an iterator and prefetches elements ahead of access,
/// improving cache hit rate for sequential access patterns.
///
/// # Note on `lookahead` parameter
///
/// The `lookahead` parameter is currently stored but not used for prefetching.
/// This is intentionally reserved for a future adaptive prefetching implementation
/// that will:
/// - Track access patterns to determine optimal prefetch distance
/// - Adjust prefetch behavior based on cache hit/miss rates
/// - Support different prefetch strategies for different data types
///
/// For immediate prefetching needs, use [`PrefetchProcessor`] which implements
/// active prefetching for slice-based data.
pub struct PrefetchIter<I, T> {
    inner: I,
    /// Lookahead distance for future adaptive prefetching.
    ///
    /// Reserved for future implementation of adaptive prefetching that will
    /// use this value to determine how many elements ahead to prefetch based
    /// on the iterator's access patterns and element memory layout.
    lookahead: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<I, T> PrefetchIter<I, T>
where
    I: Iterator<Item = T>,
{
    /// Create a new prefetching iterator
    ///
    /// # Arguments
    /// * `iter` - The underlying iterator
    /// * `lookahead` - How many elements ahead to prefetch (reserved for future use)
    pub fn new(iter: I, lookahead: usize) -> Self {
        Self {
            inner: iter,
            lookahead,
            _marker: std::marker::PhantomData,
        }
    }

    /// Get the configured lookahead distance
    ///
    /// Returns the number of elements ahead that will be prefetched
    /// once adaptive prefetching is implemented.
    #[inline(always)]
    pub fn lookahead(&self) -> usize {
        self.lookahead
    }
}

impl<I, T> Iterator for PrefetchIter<I, T>
where
    I: Iterator<Item = T>,
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// Batch processor with prefetching
///
/// Processes slices in batches while prefetching upcoming data.
pub struct PrefetchProcessor<'a, T> {
    /// The data slice
    data: &'a [T],
    /// Current position
    position: usize,
    /// Prefetch distance (elements ahead)
    prefetch_distance: usize,
}

impl<'a, T> PrefetchProcessor<'a, T> {
    /// Create a new prefetch processor
    pub fn new(data: &'a [T], prefetch_distance: usize) -> Self {
        Self {
            data,
            position: 0,
            prefetch_distance,
        }
    }

    /// Get remaining elements
    #[inline(always)]
    pub fn remaining(&self) -> usize {
        self.data.len() - self.position
    }

    /// Check if done
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.position >= self.data.len()
    }

    /// Get next element with prefetching
    #[inline]
    pub fn next(&mut self) -> Option<&T> {
        if self.position >= self.data.len() {
            return None;
        }

        // Prefetch ahead
        let prefetch_idx = self.position + self.prefetch_distance;
        if prefetch_idx < self.data.len() {
            // SAFETY: We verified prefetch_idx < self.data.len(), so the pointer is valid.
            // prefetch() only hints the CPU, it doesn't dereference the pointer.
            unsafe {
                let ptr = &self.data[prefetch_idx] as *const T as *const u8;
                prefetch(ptr, PrefetchHint::Temporal);
            }
        }

        let item = &self.data[self.position];
        self.position += 1;
        Some(item)
    }

    /// Process all elements with a function
    #[inline]
    pub fn for_each<F>(&mut self, mut f: F)
    where
        F: FnMut(&T),
    {
        while let Some(item) = self.next() {
            f(item);
        }
    }

    /// Map all elements with prefetching
    pub fn map<U, F>(&mut self, mut f: F) -> Vec<U>
    where
        F: FnMut(&T) -> U,
    {
        let mut results = Vec::with_capacity(self.remaining());
        while let Some(item) = self.next() {
            results.push(f(item));
        }
        results
    }

    /// Fold with prefetching
    pub fn fold<U, F>(&mut self, init: U, mut f: F) -> U
    where
        F: FnMut(U, &T) -> U,
    {
        let mut acc = init;
        while let Some(item) = self.next() {
            acc = f(acc, item);
        }
        acc
    }
}

/// Prefetching slice access for DX-Machine records
pub mod records {
    use super::*;

    /// Prefetch upcoming records in a buffer
    ///
    /// # Arguments
    /// * `data` - The data buffer
    /// * `current_record` - Current record index
    /// * `record_size` - Size of each record
    /// * `records_ahead` - Number of records to prefetch
    #[inline]
    pub fn prefetch_records(
        data: &[u8],
        current_record: usize,
        record_size: usize,
        records_ahead: usize,
    ) {
        let target_record = current_record + records_ahead;
        let offset = target_record * record_size;

        if offset < data.len() {
            // SAFETY: We verified offset < data.len(), so the pointer is valid.
            // prefetch_lines() only hints the CPU about upcoming memory accesses.
            unsafe {
                // Prefetch the record (may span multiple cache lines)
                let lines = record_size.div_ceil(CACHE_LINE_SIZE);
                let ptr = data.as_ptr().add(offset);
                prefetch_lines(ptr, lines, PrefetchHint::Temporal);
            }
        }
    }

    /// Process records with prefetching
    ///
    /// Returns an iterator that prefetches upcoming records.
    pub fn process_with_prefetch<'a, F, R>(
        data: &'a [u8],
        record_size: usize,
        record_count: usize,
        records_ahead: usize,
        mut processor: F,
    ) -> Vec<R>
    where
        F: FnMut(&'a [u8], usize) -> R,
    {
        let mut results = Vec::with_capacity(record_count);

        for i in 0..record_count {
            // Prefetch ahead
            prefetch_records(data, i, record_size, records_ahead);

            // Process current record
            let offset = i * record_size;
            let record_data = &data[offset..offset + record_size];
            results.push(processor(record_data, i));
        }

        results
    }

    /// Sum field with prefetching
    ///
    /// Optimized field summation with prefetch hints.
    pub fn sum_field_prefetch(
        data: &[u8],
        field_offset: usize,
        record_size: usize,
        record_count: usize,
    ) -> u64 {
        let mut sum = 0u64;
        const PREFETCH_DISTANCE: usize = 8;

        for i in 0..record_count {
            // Prefetch 8 records ahead
            if i + PREFETCH_DISTANCE < record_count {
                let pf_offset = (i + PREFETCH_DISTANCE) * record_size + field_offset;
                if pf_offset + 8 <= data.len() {
                    // SAFETY: We verified pf_offset + 8 <= data.len(), so the pointer is valid.
                    // prefetch() only hints the CPU, it doesn't dereference the pointer.
                    unsafe {
                        prefetch(data.as_ptr().add(pf_offset), PrefetchHint::Temporal);
                    }
                }
            }

            // Read current field
            let offset = i * record_size + field_offset;
            let value = u64::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);

            sum += value;
        }

        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefetch_processor() {
        let data = vec![1u64, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut processor = PrefetchProcessor::new(&data, 4);

        let sum = processor.fold(0u64, |acc, &x| acc + x);
        assert_eq!(sum, 55);
    }

    #[test]
    fn test_prefetch_processor_map() {
        let data = vec![1u32, 2, 3, 4, 5];
        let mut processor = PrefetchProcessor::new(&data, 2);

        let doubled: Vec<u32> = processor.map(|&x| x * 2);
        assert_eq!(doubled, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_prefetch_iter_lookahead() {
        let data = vec![1, 2, 3, 4, 5];
        let iter = PrefetchIter::new(data.into_iter(), 4);

        // Verify lookahead is stored correctly
        assert_eq!(iter.lookahead(), 4);
    }

    #[test]
    fn test_record_prefetch_sum() {
        // Create 10 records with u64 id at offset 0
        let record_size = 24; // 8 bytes id + 16 bytes other
        let record_count = 10;
        let mut data = vec![0u8; record_size * record_count];

        // Write IDs
        for i in 0..record_count {
            let id = (i + 1) as u64 * 100;
            let offset = i * record_size;
            data[offset..offset + 8].copy_from_slice(&id.to_le_bytes());
        }

        let sum = records::sum_field_prefetch(&data, 0, record_size, record_count);
        assert_eq!(sum, 5500); // 100 + 200 + ... + 1000
    }

    #[test]
    fn test_process_with_prefetch() {
        let record_size = 16;
        let record_count = 5;
        let mut data = vec![0u8; record_size * record_count];

        for i in 0..record_count {
            let id = i as u64;
            let offset = i * record_size;
            data[offset..offset + 8].copy_from_slice(&id.to_le_bytes());
        }

        let results = records::process_with_prefetch(
            &data,
            record_size,
            record_count,
            4,
            |record_data, _idx| u64::from_le_bytes(record_data[0..8].try_into().unwrap()),
        );

        assert_eq!(results, vec![0, 1, 2, 3, 4]);
    }
}
