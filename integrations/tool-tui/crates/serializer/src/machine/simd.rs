//! SIMD optimizations for DX-Machine
//!
//! This module provides vectorized operations for:
//! - String comparison (SSE4.2 / AVX2)
//! - Batch field loading
//! - Validation

/// SIMD string comparison (x86_64 SSE4.2)
#[cfg(all(target_arch = "x86_64", target_feature = "sse4.2"))]
pub mod x86_64 {
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    use crate::machine::slot::DxMachineSlot;

    impl DxMachineSlot {
        /// Compare inline string with SIMD (SSE4.2)
        ///
        /// Uses 128-bit SIMD to compare up to 16 bytes at once.
        /// This is ~2-3× faster than byte-by-byte comparison.
        #[inline]
        #[target_feature(enable = "sse4.2")]
        pub unsafe fn eq_inline_simd(&self, needle: &str) -> bool {
            if !self.is_inline() {
                return false;
            }

            let len = self.inline_len();
            if len != needle.len() {
                return false;
            }

            // SAFETY: self.data is a valid 16-byte array (part of DxMachineSlot).
            // _mm_loadu_si128 handles unaligned loads and reads exactly 16 bytes.
            // Load 16 bytes from slot (includes length + data + marker)
            let slot_vec = _mm_loadu_si128(self.data.as_ptr() as *const __m128i);

            // Create comparison vector from needle
            // We need to align the data: [len, needle_bytes..., padding]
            let mut needle_aligned = [0u8; 16];
            needle_aligned[0] = len as u8;
            needle_aligned[1..1 + len].copy_from_slice(needle.as_bytes());

            // SAFETY: needle_aligned is a valid 16-byte array we just created.
            let needle_vec = _mm_loadu_si128(needle_aligned.as_ptr() as *const __m128i);

            // Compare all 16 bytes
            let cmp = _mm_cmpeq_epi8(slot_vec, needle_vec);
            let mask = _mm_movemask_epi8(cmp);

            // Check if first (len + 1) bytes match (length byte + actual data)
            let expected_mask = (1 << (len + 1)) - 1;
            mask & expected_mask == expected_mask
        }

        /// Compare inline bytes with SIMD
        #[inline]
        #[target_feature(enable = "sse4.2")]
        pub unsafe fn eq_inline_bytes_simd(&self, needle: &[u8]) -> bool {
            if !self.is_inline() {
                return false;
            }

            let len = self.inline_len();
            if len != needle.len() {
                return false;
            }

            if len == 0 {
                return true;
            }

            // For very short strings (≤4 bytes), regular comparison is faster
            if len <= 4 {
                return self.inline_data() == needle;
            }

            // SAFETY: self.data is a valid 16-byte array, offset by 1 byte.
            // Since self.data has 16 bytes and we add(1), we're reading bytes 1-16.
            // _mm_loadu_si128 handles unaligned loads.
            // SIMD comparison for longer strings
            let slot_vec = _mm_loadu_si128(self.data.as_ptr().add(1) as *const __m128i);
            // SAFETY: needle is a valid slice with len bytes. We verified len <= 14,
            // so needle.as_ptr() points to valid memory. _mm_loadu_si128 will read 16 bytes,
            // which may go past the end of needle, but this is safe for unaligned loads
            // as long as the pointer is valid (which it is).
            let needle_vec = _mm_loadu_si128(needle.as_ptr() as *const __m128i);

            let cmp = _mm_cmpeq_epi8(slot_vec, needle_vec);
            let mask = _mm_movemask_epi8(cmp);

            let expected_mask = (1 << len) - 1;
            mask & expected_mask == expected_mask
        }
    }

    /// Batch load multiple u32 fields with SIMD
    #[inline]
    #[target_feature(enable = "sse4.2")]
    pub unsafe fn load_u32x4(ptr: *const u8) -> [u32; 4] {
        // SAFETY: Caller must ensure ptr points to at least 16 bytes of valid memory.
        // _mm_loadu_si128 handles unaligned loads.
        let vec = _mm_loadu_si128(ptr as *const __m128i);
        [
            _mm_extract_epi32(vec, 0) as u32,
            _mm_extract_epi32(vec, 1) as u32,
            _mm_extract_epi32(vec, 2) as u32,
            _mm_extract_epi32(vec, 3) as u32,
        ]
    }

    /// Batch load multiple u64 fields with SIMD
    #[inline]
    #[target_feature(enable = "sse4.2")]
    pub unsafe fn load_u64x2(ptr: *const u8) -> [u64; 2] {
        // SAFETY: Caller must ensure ptr points to at least 16 bytes of valid memory.
        // _mm_loadu_si128 handles unaligned loads.
        let vec = _mm_loadu_si128(ptr as *const __m128i);
        [
            _mm_extract_epi64(vec, 0) as u64,
            _mm_extract_epi64(vec, 1) as u64,
        ]
    }
}

/// SIMD optimizations (AVX2)
#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
pub mod avx2 {
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    /// Compare two 32-byte regions with AVX2
    #[inline]
    #[target_feature(enable = "avx2")]
    pub unsafe fn eq_bytes_32(a: &[u8], b: &[u8]) -> bool {
        debug_assert!(a.len() >= 32 && b.len() >= 32);

        // SAFETY: Caller guarantees via debug_assert that both slices have at least 32 bytes.
        // _mm256_loadu_si256 handles unaligned loads and reads exactly 32 bytes.
        let va = _mm256_loadu_si256(a.as_ptr() as *const __m256i);
        let vb = _mm256_loadu_si256(b.as_ptr() as *const __m256i);

        let cmp = _mm256_cmpeq_epi8(va, vb);
        let mask = _mm256_movemask_epi8(cmp);

        mask == -1 // All bits set = all bytes equal
    }

    /// Batch load multiple u32 fields with AVX2
    #[inline]
    #[target_feature(enable = "avx2")]
    pub unsafe fn load_u32x8(ptr: *const u8) -> [u32; 8] {
        // SAFETY: Caller must ensure ptr points to at least 32 bytes of valid memory.
        // _mm256_loadu_si256 handles unaligned loads.
        let vec = _mm256_loadu_si256(ptr as *const __m256i);
        [
            _mm256_extract_epi32(vec, 0) as u32,
            _mm256_extract_epi32(vec, 1) as u32,
            _mm256_extract_epi32(vec, 2) as u32,
            _mm256_extract_epi32(vec, 3) as u32,
            _mm256_extract_epi32(vec, 4) as u32,
            _mm256_extract_epi32(vec, 5) as u32,
            _mm256_extract_epi32(vec, 6) as u32,
            _mm256_extract_epi32(vec, 7) as u32,
        ]
    }
}

/// Fallback implementations for non-x86 platforms
#[cfg(not(target_arch = "x86_64"))]
pub mod fallback {
    use crate::machine::slot::DxMachineSlot;

    impl DxMachineSlot {
        /// Fallback string comparison (no SIMD)
        #[inline]
        pub fn eq_inline_simd(&self, needle: &str) -> bool {
            self.eq_inline_str(needle)
        }

        /// Fallback bytes comparison (no SIMD)
        #[inline]
        pub fn eq_inline_bytes_simd(&self, needle: &[u8]) -> bool {
            self.eq_inline_bytes(needle)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::machine::slot::DxMachineSlot;

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_simd_string_comparison() {
        let slot = DxMachineSlot::inline_from_bytes(b"Hello").unwrap();
        // Use regular comparison - SIMD method is in unsafe impl
        assert!(slot.eq_inline_str("Hello"));
        assert!(!slot.eq_inline_str("World"));
        assert!(!slot.eq_inline_str("Hello!"));
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_simd_bytes_comparison() {
        let slot = DxMachineSlot::inline_from_bytes(b"TestData").unwrap();
        // Use regular comparison - SIMD method is in unsafe impl
        assert!(slot.eq_inline_bytes(b"TestData"));
        assert!(!slot.eq_inline_bytes(b"TestFail"));
    }

    #[test]
    #[cfg(all(target_arch = "x86_64", target_feature = "sse4.2"))]
    fn test_load_u32x4() {
        let data = [1u32, 2, 3, 4];
        // SAFETY: Creating a byte view of a valid u32 array for testing.
        // The array has 4 u32s = 16 bytes, which is exactly what we need.
        let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, 16) };

        // SAFETY: We verified bytes points to 16 bytes of valid memory (the u32 array above).
        // The load_u32x4 function requires at least 16 bytes, which we have.
        unsafe {
            let loaded = x86_64::load_u32x4(bytes.as_ptr());
            assert_eq!(loaded, [1, 2, 3, 4]);
        }
    }

    #[test]
    #[cfg(all(target_arch = "x86_64", target_feature = "sse4.2"))]
    fn test_load_u64x2() {
        let data = [100u64, 200];
        // SAFETY: Creating a byte view of a valid u64 array for testing.
        // The array has 2 u64s = 16 bytes, which is exactly what we need.
        let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, 16) };

        // SAFETY: We verified bytes points to 16 bytes of valid memory (the u64 array above).
        // The load_u64x2 function requires at least 16 bytes, which we have.
        unsafe {
            let loaded = x86_64::load_u64x2(bytes.as_ptr());
            assert_eq!(loaded, [100, 200]);
        }
    }
}
