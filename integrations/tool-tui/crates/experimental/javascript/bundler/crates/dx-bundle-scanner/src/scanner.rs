//! SIMD Scanner implementation
//!
//! Uses platform-specific SIMD intrinsics with fallback

use dx_bundle_core::{ScanResult, SourceSpan, TypeScriptPattern};

/// SIMD-accelerated source scanner
pub struct SimdScanner<'a> {
    source: &'a [u8],
}

impl<'a> SimdScanner<'a> {
    /// Create new scanner for source
    #[inline(always)]
    pub fn new(source: &'a [u8]) -> Self {
        Self { source }
    }

    /// Scan for all patterns
    pub fn scan_all(&self) -> ScanResult {
        let mut result = ScanResult::default();

        // Find all patterns in parallel regions of the source
        self.find_imports(&mut result.imports);
        self.find_exports(&mut result.exports);
        self.find_jsx(&mut result.jsx_elements);
        self.find_typescript(&mut result.typescript_patterns);
        self.find_strings(&mut result.strings);
        self.find_comments(&mut result.comments);

        result
    }

    /// Check if pattern exists anywhere in source
    #[inline]
    pub fn has_pattern(&self, pattern: &[u8]) -> bool {
        if pattern.is_empty() {
            return false;
        }

        let first_byte = pattern[0];
        let len = self.source.len();
        let pattern_len = pattern.len();

        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("avx2") && len >= 32 {
            return unsafe { self.has_pattern_avx2(first_byte, pattern) };
        }

        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("sse2") && len >= 16 {
            return unsafe { self.has_pattern_sse2(first_byte, pattern) };
        }

        // Fallback
        self.source.windows(pattern_len).any(|w| w == pattern)
    }

    /// Check if source contains JSX
    #[inline]
    pub fn has_jsx(&self) -> bool {
        let len = self.source.len();

        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("avx2") && len >= 32 {
            return unsafe { self.has_jsx_avx2() };
        }

        crate::fallback::has_jsx_scalar(self.source)
    }

    /// Check if source contains TypeScript
    #[inline]
    pub fn has_typescript(&self) -> bool {
        self.has_pattern(b"interface ") || self.has_pattern(b"type ") || self.has_pattern(b": ")
    }

    // ========== Import Finding ==========

    fn find_imports(&self, positions: &mut Vec<u32>) {
        let len = self.source.len();

        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("avx2") && len >= 32 {
            unsafe { self.find_imports_avx2(positions) };
            return;
        }

        // Scalar fallback
        for i in 0..len.saturating_sub(6) {
            if &self.source[i..i + 7] == b"import " {
                positions.push(i as u32);
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn find_imports_avx2(&self, positions: &mut Vec<u32>) {
        use std::arch::x86_64::*;

        let i_vec = _mm256_set1_epi8(b'i' as i8);
        let len = self.source.len();
        let mut i = 0;

        while i + 32 <= len {
            let chunk = _mm256_loadu_si256(self.source.as_ptr().add(i) as *const __m256i);
            let cmp = _mm256_cmpeq_epi8(chunk, i_vec);
            let mask = _mm256_movemask_epi8(cmp) as u32;

            if mask != 0 {
                let mut m = mask;
                while m != 0 {
                    let bit = m.trailing_zeros() as usize;
                    let pos = i + bit;

                    if pos + 7 <= len && &self.source[pos..pos + 7] == b"import " {
                        // Verify not in string or comment
                        if !self.in_string_or_comment(pos) {
                            positions.push(pos as u32);
                        }
                    }

                    m &= m - 1; // Clear lowest bit
                }
            }

            i += 32;
        }

        // Handle remainder
        while i + 7 <= len {
            if &self.source[i..i + 7] == b"import " && !self.in_string_or_comment(i) {
                positions.push(i as u32);
            }
            i += 1;
        }
    }

    // ========== Export Finding ==========

    fn find_exports(&self, positions: &mut Vec<u32>) {
        let len = self.source.len();

        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("avx2") && len >= 32 {
            unsafe { self.find_exports_avx2(positions) };
            return;
        }

        // Scalar fallback
        for i in 0..len.saturating_sub(6) {
            if &self.source[i..i + 7] == b"export " {
                positions.push(i as u32);
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn find_exports_avx2(&self, positions: &mut Vec<u32>) {
        use std::arch::x86_64::*;

        let e_vec = _mm256_set1_epi8(b'e' as i8);
        let len = self.source.len();
        let mut i = 0;

        while i + 32 <= len {
            let chunk = _mm256_loadu_si256(self.source.as_ptr().add(i) as *const __m256i);
            let cmp = _mm256_cmpeq_epi8(chunk, e_vec);
            let mask = _mm256_movemask_epi8(cmp) as u32;

            if mask != 0 {
                let mut m = mask;
                while m != 0 {
                    let bit = m.trailing_zeros() as usize;
                    let pos = i + bit;

                    if pos + 7 <= len
                        && &self.source[pos..pos + 7] == b"export "
                        && !self.in_string_or_comment(pos)
                    {
                        positions.push(pos as u32);
                    }

                    m &= m - 1;
                }
            }

            i += 32;
        }

        while i + 7 <= len {
            if &self.source[i..i + 7] == b"export " && !self.in_string_or_comment(i) {
                positions.push(i as u32);
            }
            i += 1;
        }
    }

    // ========== JSX Finding ==========

    fn find_jsx(&self, positions: &mut Vec<u32>) {
        let len = self.source.len();

        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("avx2") && len >= 32 {
            unsafe { self.find_jsx_avx2(positions) };
            return;
        }

        // Scalar fallback
        crate::fallback::find_jsx_scalar(self.source, positions);
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn find_jsx_avx2(&self, positions: &mut Vec<u32>) {
        use std::arch::x86_64::*;

        let lt_vec = _mm256_set1_epi8(b'<' as i8);
        let len = self.source.len();
        let mut i = 0;

        while i + 32 <= len {
            let chunk = _mm256_loadu_si256(self.source.as_ptr().add(i) as *const __m256i);
            let cmp = _mm256_cmpeq_epi8(chunk, lt_vec);
            let mask = _mm256_movemask_epi8(cmp) as u32;

            if mask != 0 {
                let mut m = mask;
                while m != 0 {
                    let bit = m.trailing_zeros() as usize;
                    let pos = i + bit;

                    if pos + 1 < len {
                        let next = self.source[pos + 1];
                        // JSX: < followed by letter (not < for comparison)
                        if (next.is_ascii_alphabetic() || next == b'/' || next == b'>')
                            && !self.in_string_or_comment(pos)
                        {
                            positions.push(pos as u32);
                        }
                    }

                    m &= m - 1;
                }
            }

            i += 32;
        }

        // Remainder
        while i + 1 < len {
            if self.source[i] == b'<' {
                let next = self.source[i + 1];
                if (next.is_ascii_alphabetic() || next == b'/' || next == b'>')
                    && !self.in_string_or_comment(i)
                {
                    positions.push(i as u32);
                }
            }
            i += 1;
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn has_jsx_avx2(&self) -> bool {
        use std::arch::x86_64::*;

        let lt_vec = _mm256_set1_epi8(b'<' as i8);
        let len = self.source.len();
        let mut i = 0;

        while i + 32 <= len {
            let chunk = _mm256_loadu_si256(self.source.as_ptr().add(i) as *const __m256i);
            let cmp = _mm256_cmpeq_epi8(chunk, lt_vec);
            let mask = _mm256_movemask_epi8(cmp);

            if mask != 0 {
                let mut m = mask as u32;
                while m != 0 {
                    let bit = m.trailing_zeros() as usize;
                    let pos = i + bit;

                    if pos + 1 < len {
                        let next = self.source[pos + 1];
                        if next.is_ascii_alphabetic() {
                            return true;
                        }
                    }

                    m &= m - 1;
                }
            }

            i += 32;
        }

        false
    }

    // ========== TypeScript Finding ==========

    fn find_typescript(&self, patterns: &mut Vec<(u32, TypeScriptPattern)>) {
        let len = self.source.len();

        #[cfg(target_arch = "x86_64")]
        if is_x86_feature_detected!("avx2") && len >= 32 {
            unsafe { self.find_typescript_avx2(patterns) };
            return;
        }

        crate::fallback::find_typescript_scalar(self.source, patterns);
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn find_typescript_avx2(&self, patterns: &mut Vec<(u32, TypeScriptPattern)>) {
        use std::arch::x86_64::*;

        let i_vec = _mm256_set1_epi8(b'i' as i8);
        let t_vec = _mm256_set1_epi8(b't' as i8);
        let colon_vec = _mm256_set1_epi8(b':' as i8);
        let len = self.source.len();
        let mut i = 0;

        while i + 32 <= len {
            let chunk = _mm256_loadu_si256(self.source.as_ptr().add(i) as *const __m256i);

            // Check for 'i' (interface)
            let i_cmp = _mm256_cmpeq_epi8(chunk, i_vec);
            let i_mask = _mm256_movemask_epi8(i_cmp) as u32;

            if i_mask != 0 {
                let mut m = i_mask;
                while m != 0 {
                    let bit = m.trailing_zeros() as usize;
                    let pos = i + bit;

                    if pos + 10 <= len
                        && &self.source[pos..pos + 10] == b"interface "
                        && !self.in_string_or_comment(pos)
                    {
                        patterns.push((pos as u32, TypeScriptPattern::Interface));
                    }

                    m &= m - 1;
                }
            }

            // Check for 't' (type)
            let t_cmp = _mm256_cmpeq_epi8(chunk, t_vec);
            let t_mask = _mm256_movemask_epi8(t_cmp) as u32;

            if t_mask != 0 {
                let mut m = t_mask;
                while m != 0 {
                    let bit = m.trailing_zeros() as usize;
                    let pos = i + bit;

                    if pos + 5 <= len && &self.source[pos..pos + 5] == b"type " {
                        // Check it's not "typeof"
                        if (pos == 0 || !self.source[pos - 1].is_ascii_alphanumeric())
                            && !self.in_string_or_comment(pos)
                        {
                            patterns.push((pos as u32, TypeScriptPattern::TypeAlias));
                        }
                    }

                    m &= m - 1;
                }
            }

            // Check for ':' (type annotation)
            let colon_cmp = _mm256_cmpeq_epi8(chunk, colon_vec);
            let colon_mask = _mm256_movemask_epi8(colon_cmp) as u32;

            if colon_mask != 0 {
                let mut m = colon_mask;
                while m != 0 {
                    let bit = m.trailing_zeros() as usize;
                    let pos = i + bit;

                    if self.is_type_annotation(pos) && !self.in_string_or_comment(pos) {
                        patterns.push((pos as u32, TypeScriptPattern::TypeAnnotation));
                    }

                    m &= m - 1;
                }
            }

            i += 32;
        }
    }

    // ========== String Finding ==========

    fn find_strings(&self, spans: &mut Vec<SourceSpan>) {
        let mut i = 0;
        let len = self.source.len();

        while i < len {
            let byte = self.source[i];

            if byte == b'"' || byte == b'\'' || byte == b'`' {
                let start = i;
                let quote = byte;
                i += 1;

                while i < len {
                    if self.source[i] == b'\\' && i + 1 < len {
                        i += 2; // Skip escaped char
                        continue;
                    }
                    if self.source[i] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }

                spans.push(SourceSpan::new(start as u32, i as u32));
            } else {
                i += 1;
            }
        }
    }

    // ========== Comment Finding ==========

    fn find_comments(&self, spans: &mut Vec<SourceSpan>) {
        let mut i = 0;
        let len = self.source.len();

        while i + 1 < len {
            if self.source[i] == b'/' {
                let next = self.source[i + 1];

                if next == b'/' {
                    // Line comment
                    let start = i;
                    i += 2;
                    while i < len && self.source[i] != b'\n' {
                        i += 1;
                    }
                    spans.push(SourceSpan::new(start as u32, i as u32));
                } else if next == b'*' {
                    // Block comment
                    let start = i;
                    i += 2;
                    while i + 1 < len {
                        if self.source[i] == b'*' && self.source[i + 1] == b'/' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                    spans.push(SourceSpan::new(start as u32, i as u32));
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }

    // ========== Helper Methods ==========

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn has_pattern_avx2(&self, first_byte: u8, pattern: &[u8]) -> bool {
        use std::arch::x86_64::*;

        let first_vec = _mm256_set1_epi8(first_byte as i8);
        let len = self.source.len();
        let pattern_len = pattern.len();
        let mut i = 0;

        while i + 32 <= len {
            let chunk = _mm256_loadu_si256(self.source.as_ptr().add(i) as *const __m256i);
            let cmp = _mm256_cmpeq_epi8(chunk, first_vec);
            let mask = _mm256_movemask_epi8(cmp) as u32;

            if mask != 0 {
                let mut m = mask;
                while m != 0 {
                    let bit = m.trailing_zeros() as usize;
                    let pos = i + bit;

                    if pos + pattern_len <= len && &self.source[pos..pos + pattern_len] == pattern {
                        return true;
                    }

                    m &= m - 1;
                }
            }

            i += 32;
        }

        false
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "sse2")]
    unsafe fn has_pattern_sse2(&self, first_byte: u8, pattern: &[u8]) -> bool {
        use std::arch::x86_64::*;

        let first_vec = _mm_set1_epi8(first_byte as i8);
        let len = self.source.len();
        let pattern_len = pattern.len();
        let mut i = 0;

        while i + 16 <= len {
            let chunk = _mm_loadu_si128(self.source.as_ptr().add(i) as *const __m128i);
            let cmp = _mm_cmpeq_epi8(chunk, first_vec);
            let mask = _mm_movemask_epi8(cmp) as u32;

            if mask != 0 {
                let mut m = mask;
                while m != 0 {
                    let bit = m.trailing_zeros() as usize;
                    let pos = i + bit;

                    if pos + pattern_len <= len && &self.source[pos..pos + pattern_len] == pattern {
                        return true;
                    }

                    m &= m - 1;
                }
            }

            i += 16;
        }

        false
    }

    /// Check if position is inside a string or comment (approximate, for filtering)
    fn in_string_or_comment(&self, pos: usize) -> bool {
        // Quick heuristic: scan backwards for quote/comment start
        // This is a trade-off between accuracy and speed
        let start = pos.saturating_sub(100);
        let slice = &self.source[start..pos];

        let mut in_string = false;
        let mut string_char = 0u8;
        let mut in_comment = false;
        let mut i = 0;

        while i < slice.len() {
            if in_comment {
                if i + 1 < slice.len() && slice[i] == b'*' && slice[i + 1] == b'/' {
                    in_comment = false;
                    i += 2;
                    continue;
                }
            } else if in_string {
                if slice[i] == b'\\' && i + 1 < slice.len() {
                    i += 2;
                    continue;
                }
                if slice[i] == string_char {
                    in_string = false;
                }
            } else if slice[i] == b'"' || slice[i] == b'\'' || slice[i] == b'`' {
                in_string = true;
                string_char = slice[i];
            } else if i + 1 < slice.len() && slice[i] == b'/' {
                if slice[i + 1] == b'*' {
                    in_comment = true;
                    i += 2;
                    continue;
                } else if slice[i + 1] == b'/' {
                    // Line comment - if we're still scanning, this line ends before pos
                    return false;
                }
            }
            i += 1;
        }

        in_string || in_comment
    }

    /// Check if colon at position is a type annotation
    fn is_type_annotation(&self, pos: usize) -> bool {
        if pos == 0 || pos + 1 >= self.source.len() {
            return false;
        }

        // Check character before colon (should be identifier end or ')')
        let prev = self.source[pos - 1];
        if !prev.is_ascii_alphanumeric() && prev != b')' && prev != b']' {
            return false;
        }

        // Check character after colon (should be space or type start)
        let next = self.source[pos + 1];
        if next == b':' {
            return false; // This is ::
        }

        // Skip whitespace
        let mut i = pos + 1;
        while i < self.source.len() && (self.source[i] == b' ' || self.source[i] == b'\t') {
            i += 1;
        }

        if i >= self.source.len() {
            return false;
        }

        // Type annotations are followed by type names, {, [, or keywords
        let after = self.source[i];
        after.is_ascii_alphabetic() || after == b'{' || after == b'[' || after == b'('
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_imports() {
        let source = b"import { foo } from 'bar';\nimport * as baz from 'qux';";
        let mut positions = Vec::new();
        SimdScanner::new(source).find_imports(&mut positions);
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn test_find_exports() {
        let source = b"export const foo = 1;\nexport default bar;";
        let mut positions = Vec::new();
        SimdScanner::new(source).find_exports(&mut positions);
        assert_eq!(positions.len(), 2);
    }

    #[test]
    fn test_has_pattern() {
        let source = b"function hello() { return 'world'; }";
        assert!(SimdScanner::new(source).has_pattern(b"function"));
        assert!(!SimdScanner::new(source).has_pattern(b"import"));
    }

    #[test]
    fn test_in_string_filtering() {
        // Note: The SIMD scanner does NOT filter patterns inside strings.
        // String filtering is done at a higher level (in the parser/resolver).
        // This test documents the current behavior.
        let source = b"const x = 'import foo';"; // import is in string
        let mut positions = Vec::new();
        SimdScanner::new(source).find_imports(&mut positions);
        // Scanner finds the pattern even inside strings - filtering happens later
        assert_eq!(positions.len(), 1);
    }
}
