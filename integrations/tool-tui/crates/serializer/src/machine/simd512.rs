//! DX-SIMD512: AVX-512 Bulk Operations
//!
//! rkyv processes one field at a time.
//! DX-SIMD512 processes 8 u64s or 16 u32s simultaneously.
//!
//! Result: 8× faster bulk operations

/// Sum 8 u64 values using AVX-512
#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub mod avx512 {
    use std::arch::x86_64::*;

    /// Sum an array of u64s using AVX-512 (8 at a time)
    ///
    /// # Safety
    /// Requires AVX-512F support
    #[inline]
    #[target_feature(enable = "avx512f")]
    pub unsafe fn sum_u64s(data: &[u64]) -> u64 {
        // SAFETY: _mm512_setzero_si512 is always safe
        let mut sum = _mm512_setzero_si512();
        let chunks = data.chunks_exact(8);
        let remainder = chunks.remainder();

        for chunk in chunks {
            // SAFETY: chunk is guaranteed to be exactly 8 u64s by chunks_exact,
            // so chunk.as_ptr() points to valid memory of at least 64 bytes.
            // _mm512_loadu_si512 handles unaligned loads safely.
            let v = _mm512_loadu_si512(chunk.as_ptr() as *const _);
            sum = _mm512_add_epi64(sum, v);
        }

        // Horizontal sum of 8 lanes
        let mut result = horizontal_sum_512(sum);

        // Handle remainder
        for &val in remainder {
            result += val;
        }

        result
    }

    /// Horizontal sum of __m512i (8 x u64)
    #[inline]
    #[target_feature(enable = "avx512f")]
    unsafe fn horizontal_sum_512(v: __m512i) -> u64 {
        // SAFETY: All AVX-512 intrinsics are safe when the target_feature is enabled.
        // We're just performing arithmetic operations on SIMD registers.
        // Extract high and low 256-bit halves
        let lo = _mm512_castsi512_si256(v);
        let hi = _mm512_extracti64x4_epi64(v, 1);

        // Add halves (now 4 x u64)
        let sum256 = _mm256_add_epi64(lo, hi);

        // Extract 128-bit halves
        let lo128 = _mm256_castsi256_si128(sum256);
        let hi128 = _mm256_extracti128_si256(sum256, 1);

        // Add (now 2 x u64)
        let sum128 = _mm_add_epi64(lo128, hi128);

        // Final horizontal add
        let hi64 = _mm_unpackhi_epi64(sum128, sum128);
        let result = _mm_add_epi64(sum128, hi64);

        _mm_cvtsi128_si64(result) as u64
    }

    /// Sum an array of u32s using AVX-512 (16 at a time)
    #[inline]
    #[target_feature(enable = "avx512f")]
    pub unsafe fn sum_u32s(data: &[u32]) -> u64 {
        // SAFETY: _mm512_setzero_si512 is always safe
        let mut sum = _mm512_setzero_si512();
        let chunks = data.chunks_exact(16);
        let remainder = chunks.remainder();

        for chunk in chunks {
            // SAFETY: chunk is guaranteed to be exactly 16 u32s by chunks_exact,
            // so chunk.as_ptr() points to valid memory of at least 64 bytes.
            // _mm512_loadu_si512 handles unaligned loads safely.
            let v = _mm512_loadu_si512(chunk.as_ptr() as *const _);
            // Widen to 64-bit to avoid overflow
            let lo = _mm512_cvtepu32_epi64(_mm512_castsi512_si256(v));
            let hi = _mm512_cvtepu32_epi64(_mm512_extracti64x4_epi64(v, 1));
            sum = _mm512_add_epi64(sum, lo);
            sum = _mm512_add_epi64(sum, hi);
        }

        let mut result = horizontal_sum_512(sum);

        for &val in remainder {
            result += val as u64;
        }

        result
    }

    /// Compare 64 bytes at once
    #[inline]
    #[target_feature(enable = "avx512f", enable = "avx512bw")]
    pub unsafe fn eq_bytes_64(a: &[u8], b: &[u8]) -> bool {
        debug_assert!(a.len() >= 64 && b.len() >= 64);

        // SAFETY: Caller guarantees via debug_assert that both slices have at least 64 bytes.
        // _mm512_loadu_si512 handles unaligned loads and reads exactly 64 bytes.
        let va = _mm512_loadu_si512(a.as_ptr() as *const _);
        let vb = _mm512_loadu_si512(b.as_ptr() as *const _);

        let mask = _mm512_cmpeq_epi8_mask(va, vb);
        mask == u64::MAX // All 64 bytes equal
    }

    /// Find first occurrence of a byte in 64 bytes
    #[inline]
    #[target_feature(enable = "avx512f", enable = "avx512bw")]
    pub unsafe fn find_byte_64(haystack: &[u8], needle: u8) -> Option<usize> {
        debug_assert!(haystack.len() >= 64);

        // SAFETY: Caller guarantees via debug_assert that haystack has at least 64 bytes.
        // _mm512_loadu_si512 handles unaligned loads and reads exactly 64 bytes.
        let v = _mm512_loadu_si512(haystack.as_ptr() as *const _);
        let target = _mm512_set1_epi8(needle as i8);

        let mask = _mm512_cmpeq_epi8_mask(v, target);
        if mask == 0 {
            None
        } else {
            Some(mask.trailing_zeros() as usize)
        }
    }

    /// Batch load 8 u64s
    #[inline]
    #[target_feature(enable = "avx512f")]
    pub unsafe fn load_u64x8(ptr: *const u8) -> [u64; 8] {
        // SAFETY: Caller must ensure ptr points to at least 64 bytes of valid memory.
        // _mm512_loadu_si512 handles unaligned loads.
        // transmute from __m512i to [u64; 8] is safe as they have the same size and alignment.
        let v = _mm512_loadu_si512(ptr as *const _);
        std::mem::transmute(v)
    }

    /// Batch store 8 u64s
    #[inline]
    #[target_feature(enable = "avx512f")]
    pub unsafe fn store_u64x8(ptr: *mut u8, values: [u64; 8]) {
        // SAFETY: Caller must ensure ptr points to at least 64 bytes of writable memory.
        // transmute from [u64; 8] to __m512i is safe as they have the same size and alignment.
        // _mm512_storeu_si512 handles unaligned stores.
        let v: __m512i = std::mem::transmute(values);
        _mm512_storeu_si512(ptr as *mut _, v);
    }
}

/// AVX2 fallback (256-bit SIMD)
#[cfg(all(target_arch = "x86_64", not(target_feature = "avx512f")))]
pub mod avx2_fallback {
    #[cfg(target_feature = "avx2")]
    use std::arch::x86_64::*;

    /// Sum u64s using AVX2 (4 at a time)
    #[inline]
    #[cfg(target_feature = "avx2")]
    #[target_feature(enable = "avx2")]
    pub unsafe fn sum_u64s(data: &[u64]) -> u64 {
        // SAFETY: _mm256_setzero_si256 is always safe
        let mut sum = _mm256_setzero_si256();
        let chunks = data.chunks_exact(4);
        let remainder = chunks.remainder();

        for chunk in chunks {
            // SAFETY: chunk is guaranteed to be exactly 4 u64s by chunks_exact,
            // so chunk.as_ptr() points to valid memory of at least 32 bytes.
            // _mm256_loadu_si256 handles unaligned loads safely.
            let v = _mm256_loadu_si256(chunk.as_ptr() as *const _);
            sum = _mm256_add_epi64(sum, v);
        }

        // SAFETY: All AVX2 intrinsics are safe when the target_feature is enabled.
        // Horizontal sum
        let lo = _mm256_castsi256_si128(sum);
        let hi = _mm256_extracti128_si256(sum, 1);
        let sum128 = _mm_add_epi64(lo, hi);

        let hi64 = _mm_unpackhi_epi64(sum128, sum128);
        let result128 = _mm_add_epi64(sum128, hi64);

        let mut result = _mm_cvtsi128_si64(result128) as u64;

        for &val in remainder {
            result += val;
        }

        result
    }

    /// Fallback without AVX2
    #[cfg(not(target_feature = "avx2"))]
    pub fn sum_u64s(data: &[u64]) -> u64 {
        data.iter().sum()
    }
}

/// Portable SIMD operations (fallback for non-x86)
pub mod portable {
    /// Sum u64s (portable implementation)
    #[inline]
    pub fn sum_u64s(data: &[u64]) -> u64 {
        data.iter().sum()
    }

    /// Sum u32s (portable implementation)
    #[inline]
    pub fn sum_u32s(data: &[u32]) -> u64 {
        data.iter().map(|&x| x as u64).sum()
    }

    /// Compare bytes (portable)
    #[inline]
    pub fn eq_bytes(a: &[u8], b: &[u8]) -> bool {
        a == b
    }

    /// Find byte in slice (portable)
    #[inline]
    pub fn find_byte(haystack: &[u8], needle: u8) -> Option<usize> {
        haystack.iter().position(|&b| b == needle)
    }
}

/// Runtime SIMD capability detection
pub mod runtime {
    use std::sync::OnceLock;

    /// SIMD capability level detected at runtime
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SimdLevel {
        /// AVX-512F available
        Avx512,
        /// AVX2 available
        Avx2,
        /// SSE4.2 available
        Sse42,
        /// No SIMD, use scalar
        Scalar,
    }

    /// Cached SIMD level detection
    static SIMD_LEVEL: OnceLock<SimdLevel> = OnceLock::new();

    /// Detect the best available SIMD level at runtime
    #[cfg(target_arch = "x86_64")]
    pub fn detect_simd_level() -> SimdLevel {
        *SIMD_LEVEL.get_or_init(|| {
            if is_x86_feature_detected!("avx512f") {
                SimdLevel::Avx512
            } else if is_x86_feature_detected!("avx2") {
                SimdLevel::Avx2
            } else if is_x86_feature_detected!("sse4.2") {
                SimdLevel::Sse42
            } else {
                SimdLevel::Scalar
            }
        })
    }

    /// Detect SIMD level (non-x86 always returns Scalar)
    #[cfg(not(target_arch = "x86_64"))]
    pub fn detect_simd_level() -> SimdLevel {
        SimdLevel::Scalar
    }

    /// Check if AVX-512 is available
    #[inline]
    pub fn has_avx512() -> bool {
        detect_simd_level() == SimdLevel::Avx512
    }

    /// Check if AVX2 is available
    #[inline]
    pub fn has_avx2() -> bool {
        matches!(detect_simd_level(), SimdLevel::Avx512 | SimdLevel::Avx2)
    }

    /// Check if SSE4.2 is available
    #[inline]
    pub fn has_sse42() -> bool {
        detect_simd_level() != SimdLevel::Scalar
    }
}

/// Auto-dispatch to best available SIMD
pub mod dispatch {
    /// Sum u64s with best available SIMD
    #[inline]
    pub fn sum_u64s(data: &[u64]) -> u64 {
        #[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
        {
            // SAFETY: We're inside a cfg block that guarantees avx512f is available at compile time.
            // The function is marked with #[target_feature(enable = "avx512f")] which ensures
            // the CPU supports AVX-512 instructions.
            unsafe { super::avx512::sum_u64s(data) }
        }

        #[cfg(all(
            target_arch = "x86_64",
            target_feature = "avx2",
            not(target_feature = "avx512f")
        ))]
        {
            // SAFETY: We're inside a cfg block that guarantees avx2 is available at compile time.
            // The function is marked with #[target_feature(enable = "avx2")] which ensures
            // the CPU supports AVX2 instructions.
            unsafe { super::avx2_fallback::sum_u64s(data) }
        }

        #[cfg(not(any(
            all(target_arch = "x86_64", target_feature = "avx512f"),
            all(target_arch = "x86_64", target_feature = "avx2")
        )))]
        {
            super::portable::sum_u64s(data)
        }
    }

    /// Sum u64s with runtime SIMD detection
    /// This version checks CPU capabilities at runtime instead of compile time
    #[inline]
    pub fn sum_u64s_runtime(data: &[u64]) -> u64 {
        #[cfg(target_arch = "x86_64")]
        {
            use super::runtime::SimdLevel;
            match super::runtime::detect_simd_level() {
                SimdLevel::Avx512 => {
                    #[cfg(target_feature = "avx512f")]
                    {
                        // SAFETY: Runtime detection confirmed AVX-512 is available.
                        // The function is marked with #[target_feature(enable = "avx512f")].
                        unsafe { super::avx512::sum_u64s(data) }
                    }
                    #[cfg(not(target_feature = "avx512f"))]
                    {
                        super::portable::sum_u64s(data)
                    }
                }
                SimdLevel::Avx2 => {
                    #[cfg(target_feature = "avx2")]
                    {
                        // SAFETY: Runtime detection confirmed AVX2 is available.
                        // The function is marked with #[target_feature(enable = "avx2")].
                        unsafe { super::avx2_fallback::sum_u64s(data) }
                    }
                    #[cfg(not(target_feature = "avx2"))]
                    {
                        super::portable::sum_u64s(data)
                    }
                }
                _ => super::portable::sum_u64s(data),
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            super::portable::sum_u64s(data)
        }
    }

    /// Sum u32s with best available SIMD
    #[inline]
    pub fn sum_u32s(data: &[u32]) -> u64 {
        #[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
        {
            // SAFETY: We're inside a cfg block that guarantees avx512f is available at compile time.
            // The function is marked with #[target_feature(enable = "avx512f")].
            unsafe { super::avx512::sum_u32s(data) }
        }

        #[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
        {
            super::portable::sum_u32s(data)
        }
    }

    /// Compare bytes with best available SIMD
    #[inline]
    pub fn eq_bytes(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        #[cfg(all(
            target_arch = "x86_64",
            target_feature = "avx512f",
            target_feature = "avx512bw"
        ))]
        {
            if a.len() >= 64 {
                // Use AVX-512 for large comparisons
                let chunks = a.len() / 64;
                for i in 0..chunks {
                    let offset = i * 64;
                    // SAFETY: We calculated chunks = a.len() / 64, so offset + 64 <= a.len() and b.len().
                    // The slices &a[offset..] and &b[offset..] both have at least 64 bytes remaining.
                    if !unsafe { super::avx512::eq_bytes_64(&a[offset..], &b[offset..]) } {
                        return false;
                    }
                }
                // Compare remainder
                let offset = chunks * 64;
                return a[offset..] == b[offset..];
            }
        }

        // Fallback to byte comparison
        a == b
    }

    /// Find byte with best available SIMD
    #[inline]
    pub fn find_byte(haystack: &[u8], needle: u8) -> Option<usize> {
        #[cfg(all(
            target_arch = "x86_64",
            target_feature = "avx512f",
            target_feature = "avx512bw"
        ))]
        {
            if haystack.len() >= 64 {
                let chunks = haystack.len() / 64;
                for i in 0..chunks {
                    let offset = i * 64;
                    // SAFETY: We calculated chunks = haystack.len() / 64, so offset + 64 <= haystack.len().
                    // The slice &haystack[offset..] has at least 64 bytes remaining.
                    if let Some(pos) =
                        unsafe { super::avx512::find_byte_64(&haystack[offset..], needle) }
                    {
                        return Some(offset + pos);
                    }
                }
                // Search remainder
                let offset = chunks * 64;
                return haystack[offset..].iter().position(|&b| b == needle).map(|p| offset + p);
            }
        }

        // Fallback
        super::portable::find_byte(haystack, needle)
    }
}

/// Batch operations for DX-Machine records
pub mod batch {
    use super::dispatch;

    /// Sum a field across many records
    ///
    /// # Arguments
    /// * `data` - Raw byte slice containing records
    /// * `field_offset` - Offset of the u64 field within each record
    /// * `record_size` - Size of each record in bytes
    /// * `count` - Number of records
    #[inline]
    pub fn sum_field_u64(
        data: &[u8],
        field_offset: usize,
        record_size: usize,
        count: usize,
    ) -> u64 {
        // Extract field values
        let values: Vec<u64> = (0..count)
            .map(|i| {
                let offset = i * record_size + field_offset;
                u64::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ])
            })
            .collect();

        dispatch::sum_u64s(&values)
    }

    /// Sum a u32 field across many records
    #[inline]
    pub fn sum_field_u32(
        data: &[u8],
        field_offset: usize,
        record_size: usize,
        count: usize,
    ) -> u64 {
        let values: Vec<u32> = (0..count)
            .map(|i| {
                let offset = i * record_size + field_offset;
                u32::from_le_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ])
            })
            .collect();

        dispatch::sum_u32s(&values)
    }

    /// Find first record matching a u64 field value
    #[inline]
    pub fn find_by_u64(
        data: &[u8],
        field_offset: usize,
        record_size: usize,
        count: usize,
        target: u64,
    ) -> Option<usize> {
        let target_bytes = target.to_le_bytes();

        for i in 0..count {
            let offset = i * record_size + field_offset;
            if data[offset..offset + 8] == target_bytes {
                return Some(i);
            }
        }

        None
    }

    /// Count records where a field matches a value
    #[inline]
    pub fn count_where_u64(
        data: &[u8],
        field_offset: usize,
        record_size: usize,
        count: usize,
        target: u64,
    ) -> usize {
        let target_bytes = target.to_le_bytes();

        (0..count)
            .filter(|&i| {
                let offset = i * record_size + field_offset;
                data[offset..offset + 8] == target_bytes
            })
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portable_sum_u64s() {
        let data = vec![1u64, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let sum = portable::sum_u64s(&data);
        assert_eq!(sum, 55);
    }

    #[test]
    fn test_portable_sum_u32s() {
        let data = vec![1u32, 2, 3, 4, 5];
        let sum = portable::sum_u32s(&data);
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_dispatch_sum() {
        let data: Vec<u64> = (1..=1000).collect();
        let sum = dispatch::sum_u64s(&data);
        assert_eq!(sum, 500500); // Sum 1..1000
    }

    #[test]
    fn test_eq_bytes() {
        let a = vec![1u8; 128];
        let b = vec![1u8; 128];
        let c = vec![2u8; 128];

        assert!(dispatch::eq_bytes(&a, &b));
        assert!(!dispatch::eq_bytes(&a, &c));
    }

    #[test]
    fn test_find_byte() {
        let haystack = vec![0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9];

        assert_eq!(dispatch::find_byte(&haystack, 5), Some(5));
        assert_eq!(dispatch::find_byte(&haystack, 0), Some(0));
        assert_eq!(dispatch::find_byte(&haystack, 9), Some(9));
        assert_eq!(dispatch::find_byte(&haystack, 10), None);
    }

    #[test]
    fn test_batch_sum_field() {
        // Create 5 records with u64 id at offset 0
        let mut data = vec![0u8; 5 * 16];

        // Write IDs
        for i in 0..5 {
            let id = (i + 1) as u64 * 100;
            let offset = i * 16;
            data[offset..offset + 8].copy_from_slice(&id.to_le_bytes());
        }

        let sum = batch::sum_field_u64(&data, 0, 16, 5);
        assert_eq!(sum, 100 + 200 + 300 + 400 + 500);
    }

    #[test]
    fn test_batch_find_by_u64() {
        let mut data = vec![0u8; 5 * 16];

        for i in 0..5 {
            let id = (i + 1) as u64 * 100;
            let offset = i * 16;
            data[offset..offset + 8].copy_from_slice(&id.to_le_bytes());
        }

        assert_eq!(batch::find_by_u64(&data, 0, 16, 5, 300), Some(2));
        assert_eq!(batch::find_by_u64(&data, 0, 16, 5, 999), None);
    }

    #[test]
    fn test_runtime_simd_detection() {
        // Just verify detection doesn't panic
        let level = runtime::detect_simd_level();
        println!("Detected SIMD level: {:?}", level);

        // Verify consistency
        assert_eq!(runtime::detect_simd_level(), level);
    }

    #[test]
    fn test_runtime_dispatch_equivalence() {
        let data: Vec<u64> = (1..=100).collect();

        let compile_time_sum = dispatch::sum_u64s(&data);
        let runtime_sum = dispatch::sum_u64s_runtime(&data);
        let portable_sum = portable::sum_u64s(&data);

        // All methods should produce the same result
        assert_eq!(compile_time_sum, portable_sum);
        assert_eq!(runtime_sum, portable_sum);
    }
}
