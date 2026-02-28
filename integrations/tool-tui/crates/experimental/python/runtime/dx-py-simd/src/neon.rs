//! NEON String Engine - ARM SIMD-accelerated string operations
//!
//! This module provides NEON-accelerated string operations for ARM64 platforms.
//! It processes 16 bytes per iteration using NEON vector instructions.

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

use crate::engine::SimdStringEngine;

/// NEON-accelerated string engine for ARM64 platforms
pub struct NeonStringEngine;

impl NeonStringEngine {
    /// Create a new NEON string engine
    ///
    /// # Safety
    /// This is safe to call on any ARM64 platform as NEON is always available.
    pub fn new() -> Self {
        Self
    }

    /// Find substring using NEON (16 bytes at a time)
    #[cfg(target_arch = "aarch64")]
    #[inline]
    #[target_feature(enable = "neon")]
    unsafe fn find_neon(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        }
        if needle.len() > haystack.len() {
            return None;
        }

        let first = vdupq_n_u8(needle[0]);
        let mut i = 0;

        // Process 16 bytes at a time
        while i + 16 <= haystack.len() - needle.len() + 1 {
            let chunk = vld1q_u8(haystack.as_ptr().add(i));
            let eq = vceqq_u8(chunk, first);
            let mask = Self::neon_movemask(eq);

            if mask != 0 {
                let mut bit = 0;
                while bit < 16 {
                    if (mask >> bit) & 1 != 0 {
                        let pos = i + bit;
                        if pos + needle.len() <= haystack.len()
                            && &haystack[pos..pos + needle.len()] == needle
                        {
                            return Some(pos);
                        }
                    }
                    bit += 1;
                }
            }
            i += 16;
        }

        // Scalar fallback for remainder
        for j in i..=haystack.len().saturating_sub(needle.len()) {
            if &haystack[j..j + needle.len()] == needle {
                return Some(j);
            }
        }
        None
    }

    /// Extract a bitmask from NEON comparison result
    #[cfg(target_arch = "aarch64")]
    #[inline]
    #[target_feature(enable = "neon")]
    unsafe fn neon_movemask(v: uint8x16_t) -> u16 {
        // Create shift pattern to extract high bits
        let shift = vld1q_u8(
            [
                0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20,
                0x40, 0x80,
            ]
            .as_ptr(),
        );
        let masked = vandq_u8(v, shift);
        let low = vget_low_u8(masked);
        let high = vget_high_u8(masked);
        let low_sum = vaddv_u8(low) as u16;
        let high_sum = (vaddv_u8(high) as u16) << 8;
        low_sum | high_sum
    }

    /// String equality using NEON (16 bytes at a time)
    #[cfg(target_arch = "aarch64")]
    #[inline]
    #[target_feature(enable = "neon")]
    unsafe fn eq_neon(&self, a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut i = 0;

        // Compare 16 bytes at a time
        while i + 16 <= a.len() {
            let chunk_a = vld1q_u8(a.as_ptr().add(i));
            let chunk_b = vld1q_u8(b.as_ptr().add(i));
            let eq = vceqq_u8(chunk_a, chunk_b);

            // Check if all bytes are equal (all 0xFF)
            let min = vminvq_u8(eq);
            if min != 0xFF {
                return false;
            }
            i += 16;
        }

        // Scalar fallback for remainder
        a[i..] == b[i..]
    }

    /// Convert to lowercase using NEON
    #[cfg(target_arch = "aarch64")]
    #[inline]
    #[target_feature(enable = "neon")]
    unsafe fn to_lowercase_neon(&self, s: &str) -> String {
        let bytes = s.as_bytes();
        let mut result = Vec::with_capacity(bytes.len());
        let mut i = 0;

        let upper_a = vdupq_n_u8(b'A');
        let upper_z = vdupq_n_u8(b'Z');
        let case_bit = vdupq_n_u8(0x20);

        while i + 16 <= bytes.len() {
            let chunk = vld1q_u8(bytes.as_ptr().add(i));

            // Check if in range A-Z
            let ge_a = vcgeq_u8(chunk, upper_a);
            let le_z = vcleq_u8(chunk, upper_z);
            let is_upper = vandq_u8(ge_a, le_z);

            // Add 0x20 to uppercase letters
            let to_add = vandq_u8(is_upper, case_bit);
            let lower = vaddq_u8(chunk, to_add);

            // Store result
            let mut buf = [0u8; 16];
            vst1q_u8(buf.as_mut_ptr(), lower);
            result.extend_from_slice(&buf);
            i += 16;
        }

        // Scalar fallback for remainder
        for &byte in &bytes[i..] {
            if byte >= b'A' && byte <= b'Z' {
                result.push(byte + 0x20);
            } else {
                result.push(byte);
            }
        }

        String::from_utf8_unchecked(result)
    }

    /// Convert to uppercase using NEON
    #[cfg(target_arch = "aarch64")]
    #[inline]
    #[target_feature(enable = "neon")]
    unsafe fn to_uppercase_neon(&self, s: &str) -> String {
        let bytes = s.as_bytes();
        let mut result = Vec::with_capacity(bytes.len());
        let mut i = 0;

        let lower_a = vdupq_n_u8(b'a');
        let lower_z = vdupq_n_u8(b'z');
        let case_bit = vdupq_n_u8(0x20);

        while i + 16 <= bytes.len() {
            let chunk = vld1q_u8(bytes.as_ptr().add(i));

            // Check if in range a-z
            let ge_a = vcgeq_u8(chunk, lower_a);
            let le_z = vcleq_u8(chunk, lower_z);
            let is_lower = vandq_u8(ge_a, le_z);

            // Subtract 0x20 from lowercase letters
            let to_sub = vandq_u8(is_lower, case_bit);
            let upper = vsubq_u8(chunk, to_sub);

            // Store result
            let mut buf = [0u8; 16];
            vst1q_u8(buf.as_mut_ptr(), upper);
            result.extend_from_slice(&buf);
            i += 16;
        }

        // Scalar fallback for remainder
        for &byte in &bytes[i..] {
            if byte >= b'a' && byte <= b'z' {
                result.push(byte - 0x20);
            } else {
                result.push(byte);
            }
        }

        String::from_utf8_unchecked(result)
    }

    // Non-aarch64 fallback implementations
    #[cfg(not(target_arch = "aarch64"))]
    fn find_neon(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        }
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    #[cfg(not(target_arch = "aarch64"))]
    fn eq_neon(&self, a: &[u8], b: &[u8]) -> bool {
        a == b
    }

    #[cfg(not(target_arch = "aarch64"))]
    fn to_lowercase_neon(&self, s: &str) -> String {
        s.to_lowercase()
    }

    #[cfg(not(target_arch = "aarch64"))]
    fn to_uppercase_neon(&self, s: &str) -> String {
        s.to_uppercase()
    }
}

impl Default for NeonStringEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SimdStringEngine for NeonStringEngine {
    fn find(&self, haystack: &str, needle: &str) -> Option<usize> {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            self.find_neon(haystack.as_bytes(), needle.as_bytes())
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.find_neon(haystack.as_bytes(), needle.as_bytes())
        }
    }

    fn count(&self, haystack: &str, needle: &str) -> usize {
        if needle.is_empty() {
            return haystack.len() + 1;
        }
        let mut count = 0;
        let mut start = 0;
        while let Some(pos) = self.find(&haystack[start..], needle) {
            count += 1;
            start += pos + needle.len().max(1);
            if start >= haystack.len() {
                break;
            }
        }
        count
    }

    fn eq(&self, a: &str, b: &str) -> bool {
        if a.len() != b.len() {
            return false;
        }
        #[cfg(target_arch = "aarch64")]
        unsafe {
            self.eq_neon(a.as_bytes(), b.as_bytes())
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.eq_neon(a.as_bytes(), b.as_bytes())
        }
    }

    fn to_lowercase(&self, s: &str) -> String {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            self.to_lowercase_neon(s)
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.to_lowercase_neon(s)
        }
    }

    fn to_uppercase(&self, s: &str) -> String {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            self.to_uppercase_neon(s)
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            self.to_uppercase_neon(s)
        }
    }

    fn split<'a>(&self, s: &'a str, delimiter: &str) -> Vec<&'a str> {
        if delimiter.is_empty() {
            return vec![s];
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
        parts.join(separator)
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
        "NEON"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find() {
        let engine = NeonStringEngine::new();
        assert_eq!(engine.find("hello world", "world"), Some(6));
        assert_eq!(engine.find("hello world", "hello"), Some(0));
        assert_eq!(engine.find("hello world", "xyz"), None);
        assert_eq!(engine.find("hello world", ""), Some(0));
        assert_eq!(engine.find("", "hello"), None);
    }

    #[test]
    fn test_count() {
        let engine = NeonStringEngine::new();
        assert_eq!(engine.count("aaa", "a"), 3);
        assert_eq!(engine.count("ababa", "aba"), 1); // Non-overlapping
        assert_eq!(engine.count("hello", "x"), 0);
    }

    #[test]
    fn test_eq() {
        let engine = NeonStringEngine::new();
        assert!(engine.eq("hello", "hello"));
        assert!(!engine.eq("hello", "world"));
        assert!(!engine.eq("hello", "hell"));
        assert!(engine.eq("", ""));
    }

    #[test]
    fn test_to_lowercase() {
        let engine = NeonStringEngine::new();
        assert_eq!(engine.to_lowercase("HELLO"), "hello");
        assert_eq!(engine.to_lowercase("Hello World"), "hello world");
        assert_eq!(engine.to_lowercase("hello"), "hello");
        assert_eq!(engine.to_lowercase("123ABC"), "123abc");
    }

    #[test]
    fn test_to_uppercase() {
        let engine = NeonStringEngine::new();
        assert_eq!(engine.to_uppercase("hello"), "HELLO");
        assert_eq!(engine.to_uppercase("Hello World"), "HELLO WORLD");
        assert_eq!(engine.to_uppercase("HELLO"), "HELLO");
        assert_eq!(engine.to_uppercase("123abc"), "123ABC");
    }

    #[test]
    fn test_split() {
        let engine = NeonStringEngine::new();
        assert_eq!(engine.split("a,b,c", ","), vec!["a", "b", "c"]);
        assert_eq!(engine.split("hello", ","), vec!["hello"]);
        assert_eq!(engine.split("a::b::c", "::"), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_join() {
        let engine = NeonStringEngine::new();
        assert_eq!(engine.join(&["a", "b", "c"], ","), "a,b,c");
        assert_eq!(engine.join(&["hello"], ","), "hello");
        assert_eq!(engine.join(&[], ","), "");
    }

    #[test]
    fn test_replace() {
        let engine = NeonStringEngine::new();
        assert_eq!(engine.replace("hello world", "world", "rust"), "hello rust");
        assert_eq!(engine.replace("aaa", "a", "b"), "bbb");
        assert_eq!(engine.replace("hello", "x", "y"), "hello");
    }

    #[test]
    fn test_long_string() {
        let engine = NeonStringEngine::new();
        let long_str = "a".repeat(1000);
        let needle = "aaa";

        // Should find at position 0
        assert_eq!(engine.find(&long_str, needle), Some(0));

        // Test case conversion on long strings
        let upper = "A".repeat(1000);
        assert_eq!(engine.to_lowercase(&upper), long_str);
        assert_eq!(engine.to_uppercase(&long_str), upper);
    }
}
