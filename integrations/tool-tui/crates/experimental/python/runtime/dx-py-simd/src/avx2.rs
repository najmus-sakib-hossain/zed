//! AVX2 SIMD string engine implementation
//!
//! Processes 32 bytes per iteration using AVX2 intrinsics.

use crate::engine::SimdStringEngine;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(target_arch = "x86")]
use std::arch::x86::*;

/// AVX2 string engine - processes 32 bytes per iteration
pub struct Avx2StringEngine;

impl Avx2StringEngine {
    /// Create a new AVX2 engine
    ///
    /// # Safety
    /// Caller must ensure AVX2 is available on the current CPU.
    pub unsafe fn new() -> Self {
        Self
    }

    /// Check if AVX2 is available
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    pub fn is_available() -> bool {
        is_x86_feature_detected!("avx2")
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    pub fn is_available() -> bool {
        false
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
impl Avx2StringEngine {
    /// SIMD substring search - processes 32 bytes per iteration
    #[target_feature(enable = "avx2")]
    unsafe fn find_avx2(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        }
        if needle.len() > haystack.len() {
            return None;
        }

        let first = _mm256_set1_epi8(needle[0] as i8);
        let mut i = 0;

        // Process 32 bytes at a time
        while i + 32 <= haystack.len() - needle.len() + 1 {
            let chunk = _mm256_loadu_si256(haystack.as_ptr().add(i) as *const __m256i);
            let matches = _mm256_cmpeq_epi8(chunk, first);
            let mask = _mm256_movemask_epi8(matches) as u32;

            if mask != 0 {
                let mut bit = mask;
                while bit != 0 {
                    let pos = bit.trailing_zeros() as usize;
                    let candidate_pos = i + pos;

                    if candidate_pos + needle.len() <= haystack.len()
                        && &haystack[candidate_pos..candidate_pos + needle.len()] == needle
                    {
                        return Some(candidate_pos);
                    }
                    bit &= bit - 1;
                }
            }
            i += 32;
        }

        // Scalar fallback for remainder
        (i..haystack.len() - needle.len() + 1).find(|&j| &haystack[j..j + needle.len()] == needle)
    }

    /// SIMD string equality - compares 32 bytes at a time
    #[target_feature(enable = "avx2")]
    unsafe fn eq_avx2(&self, a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut i = 0;

        // Compare 32 bytes at a time
        while i + 32 <= a.len() {
            let va = _mm256_loadu_si256(a.as_ptr().add(i) as *const __m256i);
            let vb = _mm256_loadu_si256(b.as_ptr().add(i) as *const __m256i);
            let cmp = _mm256_cmpeq_epi8(va, vb);

            if _mm256_movemask_epi8(cmp) != -1i32 {
                return false;
            }
            i += 32;
        }

        // Scalar comparison for remainder
        a[i..] == b[i..]
    }

    /// SIMD case conversion to lowercase - 32 chars at a time
    #[target_feature(enable = "avx2")]
    unsafe fn to_lowercase_avx2(&self, s: &mut [u8]) {
        let a_minus_1 = _mm256_set1_epi8(b'A' as i8 - 1);
        let z_plus_1 = _mm256_set1_epi8(b'Z' as i8 + 1);
        let diff = _mm256_set1_epi8(32);

        let mut i = 0;

        while i + 32 <= s.len() {
            let chunk = _mm256_loadu_si256(s.as_ptr().add(i) as *const __m256i);

            // Check if in range A-Z
            let gt_a = _mm256_cmpgt_epi8(chunk, a_minus_1);
            let lt_z = _mm256_cmpgt_epi8(z_plus_1, chunk);
            let is_upper = _mm256_and_si256(gt_a, lt_z);

            // Add 32 to uppercase letters
            let add_mask = _mm256_and_si256(is_upper, diff);
            let result = _mm256_add_epi8(chunk, add_mask);

            _mm256_storeu_si256(s.as_mut_ptr().add(i) as *mut __m256i, result);
            i += 32;
        }

        // Scalar remainder
        for byte in &mut s[i..] {
            if *byte >= b'A' && *byte <= b'Z' {
                *byte += 32;
            }
        }
    }

    /// SIMD case conversion to uppercase - 32 chars at a time
    #[target_feature(enable = "avx2")]
    unsafe fn to_uppercase_avx2(&self, s: &mut [u8]) {
        let a_minus_1 = _mm256_set1_epi8(b'a' as i8 - 1);
        let z_plus_1 = _mm256_set1_epi8(b'z' as i8 + 1);
        let diff = _mm256_set1_epi8(32);

        let mut i = 0;

        while i + 32 <= s.len() {
            let chunk = _mm256_loadu_si256(s.as_ptr().add(i) as *const __m256i);

            // Check if in range a-z
            let gt_a = _mm256_cmpgt_epi8(chunk, a_minus_1);
            let lt_z = _mm256_cmpgt_epi8(z_plus_1, chunk);
            let is_lower = _mm256_and_si256(gt_a, lt_z);

            // Subtract 32 from lowercase letters
            let sub_mask = _mm256_and_si256(is_lower, diff);
            let result = _mm256_sub_epi8(chunk, sub_mask);

            _mm256_storeu_si256(s.as_mut_ptr().add(i) as *mut __m256i, result);
            i += 32;
        }

        // Scalar remainder
        for byte in &mut s[i..] {
            if *byte >= b'a' && *byte <= b'z' {
                *byte -= 32;
            }
        }
    }

    /// SIMD count occurrences
    #[target_feature(enable = "avx2")]
    unsafe fn count_avx2(&self, haystack: &[u8], needle: &[u8]) -> usize {
        if needle.is_empty() {
            return haystack.len() + 1;
        }

        let mut count = 0;
        let mut pos = 0;

        while let Some(found) = self.find_avx2(&haystack[pos..], needle) {
            count += 1;
            pos += found + needle.len(); // Non-overlapping
        }

        count
    }
}

impl SimdStringEngine for Avx2StringEngine {
    fn find(&self, haystack: &str, needle: &str) -> Option<usize> {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            if Self::is_available() {
                unsafe { self.find_avx2(haystack.as_bytes(), needle.as_bytes()) }
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
                unsafe { self.count_avx2(haystack.as_bytes(), needle.as_bytes()) }
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
                unsafe { self.eq_avx2(a.as_bytes(), b.as_bytes()) }
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
                unsafe { self.to_lowercase_avx2(&mut bytes) };
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
                unsafe { self.to_uppercase_avx2(&mut bytes) };
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
        "AVX2"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_engine() -> Avx2StringEngine {
        // Safety: Tests will fall back to scalar if AVX2 not available
        unsafe { Avx2StringEngine::new() }
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
}
