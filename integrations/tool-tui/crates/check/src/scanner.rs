//! SIMD Pattern Scanner
//!
//! AVX2/NEON accelerated pattern matching for quick rejection.
//! Scans 32-64 bytes simultaneously to find banned patterns.

use std::collections::HashSet;

/// Patterns to scan for (before parsing)
pub static BANNED_PATTERNS: &[&[u8]] = &[
    b"console.log",
    b"console.warn",
    b"console.error",
    b"console.debug",
    b"console.info",
    b"debugger",
    b"eval(",
    b"document.write",
    b"innerHTML",
    b"@ts-ignore",
    b"@ts-nocheck",
    b"TODO:",
    b"FIXME:",
    b"XXX:",
    b"HACK:",
];

/// A pattern match result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternMatch {
    /// Which pattern matched
    pub pattern_id: usize,
    /// Byte offset in source
    pub offset: usize,
    /// Length of match
    pub length: usize,
}

/// SIMD-accelerated pattern scanner
pub struct PatternScanner {
    /// Patterns to search for
    patterns: Vec<Pattern>,
    /// First bytes of all patterns (for SIMD comparison)
    first_bytes: Vec<u8>,
}

#[derive(Clone)]
struct Pattern {
    id: usize,
    bytes: Vec<u8>,
}

impl PatternScanner {
    /// Create a new scanner with default banned patterns
    #[must_use]
    pub fn new() -> Self {
        Self::with_patterns(BANNED_PATTERNS)
    }

    /// Create a scanner with custom patterns
    #[must_use]
    pub fn with_patterns(patterns: &[&[u8]]) -> Self {
        let patterns: Vec<Pattern> = patterns
            .iter()
            .enumerate()
            .map(|(id, &bytes)| Pattern {
                id,
                bytes: bytes.to_vec(),
            })
            .collect();

        let first_bytes: Vec<u8> = patterns.iter().map(|p| p.bytes[0]).collect();

        Self {
            patterns,
            first_bytes,
        }
    }

    /// Scan source for patterns
    /// Returns matches sorted by offset
    #[must_use]
    pub fn scan(&self, source: &[u8]) -> Vec<PatternMatch> {
        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        {
            if is_x86_feature_detected!("avx2") {
                return unsafe { self.scan_avx2(source) };
            }
        }

        #[cfg(all(target_arch = "aarch64", feature = "simd"))]
        {
            return unsafe { self.scan_neon(source) };
        }

        // Fallback to scalar implementation
        self.scan_scalar(source)
    }

    /// Scalar fallback implementation
    fn scan_scalar(&self, source: &[u8]) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        for (offset, _) in source.iter().enumerate() {
            for pattern in &self.patterns {
                if source[offset..].starts_with(&pattern.bytes) {
                    matches.push(PatternMatch {
                        pattern_id: pattern.id,
                        offset,
                        length: pattern.bytes.len(),
                    });
                }
            }
        }

        matches
    }

    /// AVX2 implementation (`x86_64`)
    ///
    /// # Safety
    /// This function requires AVX2 support. The caller must verify
    /// `is_x86_feature_detected!("avx2")` returns true before calling.
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn scan_avx2(&self, source: &[u8]) -> Vec<PatternMatch> {
        use std::arch::x86_64::{
            __m256i, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8, _mm256_set1_epi8,
        };

        let mut matches = Vec::new();
        let mut offset = 0;

        // Create masks for first bytes of each pattern
        // We can check up to 32 positions at once with AVX2

        while offset + 32 <= source.len() {
            // SAFETY: We verified offset + 32 <= source.len() above.
            // The pointer is valid for reading 32 bytes.
            // _mm256_loadu_si256 handles unaligned loads safely.
            let chunk = unsafe { _mm256_loadu_si256(source[offset..].as_ptr().cast::<__m256i>()) };

            // For each pattern's first byte, check if it exists in the chunk
            for pattern in &self.patterns {
                if pattern.bytes.is_empty() {
                    continue;
                }

                let first_byte = pattern.bytes[0];

                // SAFETY: _mm256_set1_epi8 is safe to call with any i8 value.
                // It broadcasts the byte to all 32 positions in the vector.
                let needle = _mm256_set1_epi8(first_byte as i8);

                // SAFETY: _mm256_cmpeq_epi8 compares two valid __m256i vectors.
                // _mm256_movemask_epi8 extracts the high bit of each byte.
                // Both operations are safe with valid AVX2 vectors.
                let cmp = _mm256_cmpeq_epi8(chunk, needle);
                let mask = _mm256_movemask_epi8(cmp) as u32;

                if mask != 0 {
                    // There are potential matches - verify them
                    for bit_pos in 0..32 {
                        if mask & (1 << bit_pos) != 0 {
                            let pos = offset + bit_pos;
                            if pos + pattern.bytes.len() <= source.len()
                                && source[pos..].starts_with(&pattern.bytes)
                            {
                                matches.push(PatternMatch {
                                    pattern_id: pattern.id,
                                    offset: pos,
                                    length: pattern.bytes.len(),
                                });
                            }
                        }
                    }
                }
            }

            offset += 32;
        }

        // Handle remaining bytes with scalar
        while offset < source.len() {
            for pattern in &self.patterns {
                if source[offset..].starts_with(&pattern.bytes) {
                    matches.push(PatternMatch {
                        pattern_id: pattern.id,
                        offset,
                        length: pattern.bytes.len(),
                    });
                }
            }
            offset += 1;
        }

        // Sort by offset
        matches.sort_by_key(|m| m.offset);
        matches
    }

    /// NEON implementation (aarch64)
    #[cfg(target_arch = "aarch64")]
    unsafe fn scan_neon(&self, source: &[u8]) -> Vec<PatternMatch> {
        use std::arch::aarch64::*;

        let mut matches = Vec::new();
        let mut offset = 0;

        while offset + 16 <= source.len() {
            // Load 16 bytes from source
            let chunk = vld1q_u8(source[offset..].as_ptr());

            for pattern in &self.patterns {
                if pattern.bytes.is_empty() {
                    continue;
                }

                let first_byte = pattern.bytes[0];

                // Broadcast first byte
                let needle = vdupq_n_u8(first_byte);

                // Compare
                let cmp = vceqq_u8(chunk, needle);

                // Extract match mask
                // This is simplified - full implementation would use vmaxvq_u8
                let mut mask_bytes = [0u8; 16];
                vst1q_u8(mask_bytes.as_mut_ptr(), cmp);

                for (bit_pos, &mask_byte) in mask_bytes.iter().enumerate() {
                    if mask_byte == 0xFF {
                        let pos = offset + bit_pos;
                        if pos + pattern.bytes.len() <= source.len()
                            && source[pos..].starts_with(&pattern.bytes)
                        {
                            matches.push(PatternMatch {
                                pattern_id: pattern.id,
                                offset: pos,
                                length: pattern.bytes.len(),
                            });
                        }
                    }
                }
            }

            offset += 16;
        }

        // Handle remaining bytes with scalar
        while offset < source.len() {
            for pattern in &self.patterns {
                if source[offset..].starts_with(&pattern.bytes) {
                    matches.push(PatternMatch {
                        pattern_id: pattern.id,
                        offset,
                        length: pattern.bytes.len(),
                    });
                }
            }
            offset += 1;
        }

        matches.sort_by_key(|m| m.offset);
        matches
    }

    /// Quick check if source has any banned patterns
    /// Useful for fast rejection of clean files
    #[must_use]
    pub fn has_any_match(&self, source: &[u8]) -> bool {
        #[cfg(all(target_arch = "x86_64", feature = "simd"))]
        {
            if is_x86_feature_detected!("avx2") {
                return unsafe { self.has_any_match_avx2(source) };
            }
        }

        // Scalar fallback
        for pattern in &self.patterns {
            if source.windows(pattern.bytes.len()).any(|w| w == pattern.bytes.as_slice()) {
                return true;
            }
        }
        false
    }

    /// Quick check using AVX2 if source has any banned patterns
    ///
    /// # Safety
    /// This function requires AVX2 support. The caller must verify
    /// `is_x86_feature_detected!("avx2")` returns true before calling.
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn has_any_match_avx2(&self, source: &[u8]) -> bool {
        use std::arch::x86_64::{
            __m256i, _mm256_cmpeq_epi8, _mm256_loadu_si256, _mm256_movemask_epi8, _mm256_set1_epi8,
        };

        // Quick check using first bytes only
        let first_bytes_set: HashSet<u8> = self.first_bytes.iter().copied().collect();

        let mut offset = 0;
        while offset + 32 <= source.len() {
            // SAFETY: We verified offset + 32 <= source.len() above.
            // The pointer is valid for reading 32 bytes.
            // _mm256_loadu_si256 handles unaligned loads safely.
            let chunk = unsafe { _mm256_loadu_si256(source[offset..].as_ptr().cast::<__m256i>()) };

            for &first_byte in &first_bytes_set {
                // SAFETY: These AVX2 intrinsics are safe to call with valid vectors.
                // - _mm256_set1_epi8: broadcasts byte to all positions
                // - _mm256_cmpeq_epi8: compares two vectors element-wise
                // - _mm256_movemask_epi8: extracts high bits as a mask
                let needle = _mm256_set1_epi8(first_byte as i8);
                let cmp = _mm256_cmpeq_epi8(chunk, needle);
                let mask = _mm256_movemask_epi8(cmp) as u32;

                if mask != 0 {
                    // Verify full pattern
                    for pattern in &self.patterns {
                        if pattern.bytes.is_empty() || pattern.bytes[0] != first_byte {
                            continue;
                        }

                        for bit_pos in 0..32 {
                            if mask & (1 << bit_pos) != 0 {
                                let pos = offset + bit_pos;
                                if pos + pattern.bytes.len() <= source.len()
                                    && source[pos..].starts_with(&pattern.bytes)
                                {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }

            offset += 32;
        }

        // Check remaining bytes
        for pattern in &self.patterns {
            if source[offset..]
                .windows(pattern.bytes.len())
                .any(|w| w == pattern.bytes.as_slice())
            {
                return true;
            }
        }

        false
    }

    /// Get pattern by ID
    #[must_use]
    pub fn get_pattern(&self, id: usize) -> Option<&[u8]> {
        self.patterns.get(id).map(|p| p.bytes.as_slice())
    }
}

impl Default for PatternScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_scalar() {
        let scanner = PatternScanner::new();
        let source = b"const x = 1; console.log('hello'); debugger;";

        let matches = scanner.scan_scalar(source);

        assert!(!matches.is_empty());
        assert!(matches.iter().any(|m| m.pattern_id == 0)); // console.log
        assert!(matches.iter().any(|m| m.pattern_id == 5)); // debugger
    }

    #[test]
    fn test_has_any_match() {
        let scanner = PatternScanner::new();

        assert!(scanner.has_any_match(b"console.log('test')"));
        assert!(scanner.has_any_match(b"debugger;"));
        assert!(!scanner.has_any_match(b"const x = 1;"));
    }

    #[test]
    fn test_custom_patterns() {
        let scanner = PatternScanner::with_patterns(&[b"foo", b"bar"]);
        let source = b"foo bar baz";

        let matches = scanner.scan(source);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].offset, 0); // foo
        assert_eq!(matches[1].offset, 4); // bar
    }
}
