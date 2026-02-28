//! AVX-512 SIMD string engine implementation
//!
//! Processes 64 bytes per iteration using AVX-512 intrinsics.
//! Provides 2x throughput improvement over AVX2 on supported CPUs.
//!
//! ## Requirements
//!
//! This module requires CPUs with AVX-512F and AVX-512BW support:
//! - Intel Skylake-X and later server CPUs
//! - Intel Ice Lake and later client CPUs
//! - AMD Zen 4 and later CPUs
//!
//! ## Usage
//!
//! ```rust,ignore
//! use dx_py_simd::avx512::Avx512StringEngine;
//!
//! if Avx512StringEngine::is_available() {
//!     let engine = unsafe { Avx512StringEngine::new() };
//!     let pos = engine.find("hello world", "world");
//! }
//! ```

use crate::engine::SimdStringEngine;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(target_arch = "x86")]
use std::arch::x86::*;

/// AVX-512 string engine - processes 64 bytes per iteration
///
/// This engine provides the highest throughput on supported CPUs,
/// processing 64 bytes per iteration compared to 32 bytes for AVX2.
pub struct Avx512StringEngine;

impl Avx512StringEngine {
    /// Create a new AVX-512 engine
    ///
    /// # Safety
    /// Caller must ensure AVX-512F and AVX-512BW are available on the current CPU.
    /// Use `is_available()` to check before calling this function.
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    pub unsafe fn new() -> Self {
        Self
    }

    /// Create a new AVX-512 engine (non-x86 fallback)
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    pub fn new() -> Self {
        Self
    }

    /// Check if AVX-512F and AVX-512BW are available
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    pub fn is_available() -> bool {
        is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512bw")
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    pub fn is_available() -> bool {
        false
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
impl Avx512StringEngine {
    /// SIMD substring search - processes 64 bytes per iteration
    ///
    /// Uses AVX-512 to search for the first byte of the needle in 64-byte chunks,
    /// then verifies full needle matches at candidate positions.
    #[target_feature(enable = "avx512f", enable = "avx512bw")]
    unsafe fn find_avx512(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        }
        if needle.len() > haystack.len() {
            return None;
        }

        let first = _mm512_set1_epi8(needle[0] as i8);
        let mut i = 0;

        // Process 64 bytes at a time
        while i + 64 <= haystack.len() - needle.len() + 1 {
            let chunk = _mm512_loadu_si512(haystack.as_ptr().add(i) as *const __m512i);
            let eq_mask = _mm512_cmpeq_epi8_mask(chunk, first);

            if eq_mask != 0 {
                let mut mask = eq_mask;
                while mask != 0 {
                    let bit = mask.trailing_zeros() as usize;
                    let candidate_pos = i + bit;

                    if candidate_pos + needle.len() <= haystack.len()
                        && &haystack[candidate_pos..candidate_pos + needle.len()] == needle
                    {
                        return Some(candidate_pos);
                    }
                    mask &= mask - 1;
                }
            }
            i += 64;
        }

        // Scalar fallback for remainder
        (i..=haystack.len().saturating_sub(needle.len()))
            .find(|&j| &haystack[j..j + needle.len()] == needle)
    }

    /// SIMD string equality - compares 64 bytes at a time
    ///
    /// Uses AVX-512 to compare strings in 64-byte chunks with early exit on mismatch.
    #[target_feature(enable = "avx512f", enable = "avx512bw")]
    unsafe fn eq_avx512(&self, a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut i = 0;

        // Compare 64 bytes at a time
        while i + 64 <= a.len() {
            let va = _mm512_loadu_si512(a.as_ptr().add(i) as *const __m512i);
            let vb = _mm512_loadu_si512(b.as_ptr().add(i) as *const __m512i);
            let eq_mask = _mm512_cmpeq_epi8_mask(va, vb);

            // All 64 bytes must match (mask should be all 1s = u64::MAX)
            if eq_mask != u64::MAX {
                return false;
            }
            i += 64;
        }

        // Scalar comparison for remainder
        a[i..] == b[i..]
    }

    /// SIMD case conversion to lowercase - 64 chars at a time
    ///
    /// Uses AVX-512 to convert ASCII uppercase letters (A-Z) to lowercase (a-z).
    #[target_feature(enable = "avx512f", enable = "avx512bw")]
    unsafe fn to_lowercase_avx512(&self, s: &mut [u8]) {
        let a_minus_1 = _mm512_set1_epi8(b'A' as i8 - 1);
        let z_plus_1 = _mm512_set1_epi8(b'Z' as i8 + 1);
        let diff = _mm512_set1_epi8(32);

        let mut i = 0;

        while i + 64 <= s.len() {
            let chunk = _mm512_loadu_si512(s.as_ptr().add(i) as *const __m512i);

            // Check if in range A-Z using mask comparisons
            let gt_a_mask = _mm512_cmpgt_epi8_mask(chunk, a_minus_1);
            let lt_z_mask = _mm512_cmplt_epi8_mask(chunk, z_plus_1);
            let is_upper_mask = gt_a_mask & lt_z_mask;

            // Add 32 to uppercase letters using mask_add
            let result = _mm512_mask_add_epi8(chunk, is_upper_mask, chunk, diff);

            _mm512_storeu_si512(s.as_mut_ptr().add(i) as *mut __m512i, result);
            i += 64;
        }

        // Scalar remainder
        for byte in &mut s[i..] {
            if *byte >= b'A' && *byte <= b'Z' {
                *byte += 32;
            }
        }
    }

    /// SIMD case conversion to uppercase - 64 chars at a time
    ///
    /// Uses AVX-512 to convert ASCII lowercase letters (a-z) to uppercase (A-Z).
    #[target_feature(enable = "avx512f", enable = "avx512bw")]
    unsafe fn to_uppercase_avx512(&self, s: &mut [u8]) {
        let a_minus_1 = _mm512_set1_epi8(b'a' as i8 - 1);
        let z_plus_1 = _mm512_set1_epi8(b'z' as i8 + 1);
        let diff = _mm512_set1_epi8(32);

        let mut i = 0;

        while i + 64 <= s.len() {
            let chunk = _mm512_loadu_si512(s.as_ptr().add(i) as *const __m512i);

            // Check if in range a-z using mask comparisons
            let gt_a_mask = _mm512_cmpgt_epi8_mask(chunk, a_minus_1);
            let lt_z_mask = _mm512_cmplt_epi8_mask(chunk, z_plus_1);
            let is_lower_mask = gt_a_mask & lt_z_mask;

            // Subtract 32 from lowercase letters using mask_sub
            let result = _mm512_mask_sub_epi8(chunk, is_lower_mask, chunk, diff);

            _mm512_storeu_si512(s.as_mut_ptr().add(i) as *mut __m512i, result);
            i += 64;
        }

        // Scalar remainder
        for byte in &mut s[i..] {
            if *byte >= b'a' && *byte <= b'z' {
                *byte -= 32;
            }
        }
    }

    /// SIMD count occurrences - uses find in a loop
    #[target_feature(enable = "avx512f", enable = "avx512bw")]
    unsafe fn count_avx512(&self, haystack: &[u8], needle: &[u8]) -> usize {
        if needle.is_empty() {
            return haystack.len() + 1;
        }

        let mut count = 0;
        let mut pos = 0;

        while let Some(found) = self.find_avx512(&haystack[pos..], needle) {
            count += 1;
            pos += found + needle.len(); // Non-overlapping
        }

        count
    }
}

impl SimdStringEngine for Avx512StringEngine {
    fn find(&self, haystack: &str, needle: &str) -> Option<usize> {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            if Self::is_available() {
                unsafe { self.find_avx512(haystack.as_bytes(), needle.as_bytes()) }
            } else {
                haystack.find(needle)
            }
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
        {
            haystack.find(needle)
        }
    }

    fn count(&self, haystack: &str, needle: &str) -> usize {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            if Self::is_available() {
                unsafe { self.count_avx512(haystack.as_bytes(), needle.as_bytes()) }
            } else {
                haystack.matches(needle).count()
            }
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
        {
            haystack.matches(needle).count()
        }
    }

    fn eq(&self, a: &str, b: &str) -> bool {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            if Self::is_available() {
                unsafe { self.eq_avx512(a.as_bytes(), b.as_bytes()) }
            } else {
                a == b
            }
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
        {
            a == b
        }
    }

    fn to_lowercase(&self, s: &str) -> String {
        // Only use SIMD for ASCII strings
        if !s.is_ascii() {
            return s.to_lowercase();
        }

        let mut bytes = s.as_bytes().to_vec();

        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            if Self::is_available() {
                unsafe { self.to_lowercase_avx512(&mut bytes) };
            } else {
                for byte in &mut bytes {
                    if *byte >= b'A' && *byte <= b'Z' {
                        *byte += 32;
                    }
                }
            }
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
        {
            for byte in &mut bytes {
                if *byte >= b'A' && *byte <= b'Z' {
                    *byte += 32;
                }
            }
        }

        // Safety: We only modified ASCII bytes, so UTF-8 validity is preserved
        unsafe { String::from_utf8_unchecked(bytes) }
    }

    fn to_uppercase(&self, s: &str) -> String {
        // Only use SIMD for ASCII strings
        if !s.is_ascii() {
            return s.to_uppercase();
        }

        let mut bytes = s.as_bytes().to_vec();

        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            if Self::is_available() {
                unsafe { self.to_uppercase_avx512(&mut bytes) };
            } else {
                for byte in &mut bytes {
                    if *byte >= b'a' && *byte <= b'z' {
                        *byte -= 32;
                    }
                }
            }
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
        {
            for byte in &mut bytes {
                if *byte >= b'a' && *byte <= b'z' {
                    *byte -= 32;
                }
            }
        }

        // Safety: We only modified ASCII bytes, so UTF-8 validity is preserved
        unsafe { String::from_utf8_unchecked(bytes) }
    }

    fn split<'a>(&self, s: &'a str, delimiter: &str) -> Vec<&'a str> {
        if delimiter.is_empty() {
            return s.split("").filter(|s| !s.is_empty()).collect();
        }

        let mut result = Vec::new();
        let mut start = 0;

        while let Some(pos) = self.find(&s[start..], delimiter) {
            result.push(&s[start..start + pos]);
            start += pos + delimiter.len();
        }

        result.push(&s[start..]);
        result
    }

    fn join(&self, parts: &[&str], separator: &str) -> String {
        if parts.is_empty() {
            return String::new();
        }

        // Calculate total length
        let total_len: usize =
            parts.iter().map(|s| s.len()).sum::<usize>() + separator.len() * (parts.len() - 1);

        let mut result = String::with_capacity(total_len);

        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                result.push_str(separator);
            }
            result.push_str(part);
        }

        result
    }

    fn replace(&self, s: &str, from: &str, to: &str) -> String {
        if from.is_empty() {
            return s.to_string();
        }

        let mut result = String::with_capacity(s.len());
        let mut start = 0;

        while let Some(pos) = self.find(&s[start..], from) {
            result.push_str(&s[start..start + pos]);
            result.push_str(to);
            start += pos + from.len();
        }

        result.push_str(&s[start..]);
        result
    }

    fn name(&self) -> &'static str {
        "AVX-512"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_engine() -> Avx512StringEngine {
        // Safety: Tests will fall back to scalar if AVX-512 not available
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        unsafe {
            Avx512StringEngine::new()
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
        Avx512StringEngine::new()
    }

    #[test]
    fn test_find() {
        let engine = get_engine();

        assert_eq!(engine.find("hello world", "world"), Some(6));
        assert_eq!(engine.find("hello world", "foo"), None);
        assert_eq!(engine.find("hello", ""), Some(0));

        // Test with longer strings (to exercise SIMD path)
        let long_haystack = "a".repeat(100) + "needle" + &"b".repeat(100);
        assert_eq!(engine.find(&long_haystack, "needle"), Some(100));
    }

    #[test]
    fn test_eq() {
        let engine = get_engine();

        assert!(engine.eq("hello", "hello"));
        assert!(!engine.eq("hello", "world"));

        // Test with longer strings
        let long_a = "a".repeat(100);
        let long_b = "a".repeat(100);
        let long_c = "a".repeat(99) + "b";

        assert!(engine.eq(&long_a, &long_b));
        assert!(!engine.eq(&long_a, &long_c));
    }

    #[test]
    fn test_to_lowercase() {
        let engine = get_engine();

        assert_eq!(engine.to_lowercase("HELLO"), "hello");
        assert_eq!(engine.to_lowercase("Hello World"), "hello world");

        // Test with longer strings
        let long_upper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(10);
        let long_lower = "abcdefghijklmnopqrstuvwxyz".repeat(10);
        assert_eq!(engine.to_lowercase(&long_upper), long_lower);
    }

    #[test]
    fn test_to_uppercase() {
        let engine = get_engine();

        assert_eq!(engine.to_uppercase("hello"), "HELLO");
        assert_eq!(engine.to_uppercase("Hello World"), "HELLO WORLD");

        // Test with longer strings
        let long_lower = "abcdefghijklmnopqrstuvwxyz".repeat(10);
        let long_upper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(10);
        assert_eq!(engine.to_uppercase(&long_lower), long_upper);
    }

    #[test]
    fn test_count() {
        let engine = get_engine();

        assert_eq!(engine.count("hello hello hello", "hello"), 3);
        assert_eq!(engine.count("aaa", "a"), 3);
    }

    #[test]
    fn test_split() {
        let engine = get_engine();

        assert_eq!(engine.split("a,b,c", ","), vec!["a", "b", "c"]);
        assert_eq!(engine.split("hello", ","), vec!["hello"]);
    }

    #[test]
    fn test_join() {
        let engine = get_engine();

        assert_eq!(engine.join(&["a", "b", "c"], ","), "a,b,c");
        assert_eq!(engine.join(&["hello"], ","), "hello");
        assert_eq!(engine.join(&[], ","), "");
    }

    #[test]
    fn test_replace() {
        let engine = get_engine();

        assert_eq!(engine.replace("hello world", "world", "rust"), "hello rust");
        assert_eq!(engine.replace("aaa", "a", "b"), "bbb");
    }

    #[test]
    fn test_availability() {
        // This test just verifies the availability check doesn't panic
        let available = Avx512StringEngine::is_available();
        println!("AVX-512 available: {}", available);
    }
}
