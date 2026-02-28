//! SIMD-accelerated parsing utilities for DXM.
//!
//! This module provides SIMD-accelerated implementations of common parsing
//! operations. On x86_64 platforms with AVX2 support, these functions can
//! achieve significant speedups (2x or more) compared to scalar implementations.
//!
//! On platforms without SIMD support, fallback scalar implementations are used.
//!
//! # Performance Targets
//!
//! - Parse throughput: 100 MB/s
//! - Compile throughput: 50 MB/s
//! - Token counting: 10 MB/s
//! - Memory usage: 2x input size

/// Find the first occurrence of a byte in a slice using SIMD when available.
///
/// This is useful for quickly scanning through large documents to find
/// delimiters like `|`, `\n`, `@`, etc.
#[inline]
pub fn find_byte(haystack: &[u8], needle: u8) -> Option<usize> {
    // Use memchr for optimized byte searching (it uses SIMD internally)
    memchr::memchr(needle, haystack)
}

/// Find the last occurrence of a byte in a slice using SIMD when available.
#[inline]
pub fn rfind_byte(haystack: &[u8], needle: u8) -> Option<usize> {
    memchr::memrchr(needle, haystack)
}

/// Find the first occurrence of any byte in a set using SIMD when available.
///
/// This is useful for finding any of several delimiters at once.
#[inline]
pub fn find_any_byte(haystack: &[u8], needles: &[u8]) -> Option<usize> {
    match needles.len() {
        0 => None,
        1 => memchr::memchr(needles[0], haystack),
        2 => memchr::memchr2(needles[0], needles[1], haystack),
        3 => memchr::memchr3(needles[0], needles[1], needles[2], haystack),
        _ => haystack.iter().position(|b| needles.contains(b)),
    }
}

/// Count occurrences of a byte in a slice using SIMD when available.
#[inline]
pub fn count_byte(haystack: &[u8], needle: u8) -> usize {
    memchr::memchr_iter(needle, haystack).count()
}

/// Find the first newline character in a slice.
#[inline]
pub fn find_newline(haystack: &[u8]) -> Option<usize> {
    find_byte(haystack, b'\n')
}

/// Find the first pipe character in a slice.
#[inline]
pub fn find_pipe(haystack: &[u8]) -> Option<usize> {
    find_byte(haystack, b'|')
}

/// Find the first occurrence of a delimiter (newline or pipe).
#[inline]
pub fn find_delimiter(haystack: &[u8]) -> Option<usize> {
    memchr::memchr2(b'\n', b'|', haystack)
}

/// Check if SIMD acceleration is available on this platform.
///
/// Returns true if the platform supports SIMD-accelerated operations.
/// The memchr crate automatically uses SIMD when available.
#[inline]
pub fn is_simd_available() -> bool {
    // memchr uses SIMD automatically on supported platforms
    cfg!(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64"))
}

/// Fast whitespace trimming using SIMD-accelerated byte searching.
///
/// Returns a slice with leading and trailing whitespace removed.
#[inline]
pub fn trim_whitespace(input: &[u8]) -> &[u8] {
    let start = input.iter().position(|&b| !is_whitespace_byte(b)).unwrap_or(input.len());
    let end = input
        .iter()
        .rposition(|&b| !is_whitespace_byte(b))
        .map(|i| i + 1)
        .unwrap_or(start);
    &input[start..end]
}

/// Check if a byte is ASCII whitespace.
#[inline]
pub const fn is_whitespace_byte(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

/// Count consecutive whitespace bytes from the start.
#[inline]
pub fn count_leading_whitespace(input: &[u8]) -> usize {
    input.iter().take_while(|&&b| is_whitespace_byte(b)).count()
}

/// Count consecutive whitespace bytes from the end.
#[inline]
pub fn count_trailing_whitespace(input: &[u8]) -> usize {
    input.iter().rev().take_while(|&&b| is_whitespace_byte(b)).count()
}

/// Fast line counting using SIMD-accelerated newline search.
#[inline]
pub fn count_lines(input: &[u8]) -> usize {
    if input.is_empty() {
        return 0;
    }
    count_byte(input, b'\n') + 1
}

/// Find all occurrences of a byte in a slice.
///
/// Returns an iterator over the positions of all occurrences.
#[inline]
pub fn find_all_bytes(haystack: &[u8], needle: u8) -> impl Iterator<Item = usize> + '_ {
    memchr::memchr_iter(needle, haystack)
}

/// Find the first occurrence of a two-byte sequence.
#[inline]
pub fn find_two_bytes(haystack: &[u8], first: u8, second: u8) -> Option<usize> {
    let mut pos = 0;
    while pos < haystack.len().saturating_sub(1) {
        if let Some(idx) = memchr::memchr(first, &haystack[pos..]) {
            let abs_idx = pos + idx;
            if abs_idx + 1 < haystack.len() && haystack[abs_idx + 1] == second {
                return Some(abs_idx);
            }
            pos = abs_idx + 1;
        } else {
            break;
        }
    }
    None
}

/// Find markdown code fence (```) using SIMD.
#[inline]
pub fn find_code_fence(haystack: &[u8]) -> Option<usize> {
    let mut pos = 0;
    while pos < haystack.len().saturating_sub(2) {
        if let Some(idx) = memchr::memchr(b'`', &haystack[pos..]) {
            let abs_idx = pos + idx;
            if abs_idx + 2 < haystack.len()
                && haystack[abs_idx + 1] == b'`'
                && haystack[abs_idx + 2] == b'`'
            {
                return Some(abs_idx);
            }
            pos = abs_idx + 1;
        } else {
            break;
        }
    }
    None
}

/// Find markdown header marker (#) at the start of a line.
#[inline]
pub fn find_header_marker(haystack: &[u8]) -> Option<usize> {
    // Check if starts with #
    if haystack.first() == Some(&b'#') {
        return Some(0);
    }
    // Find \n# pattern
    let mut pos = 0;
    while pos < haystack.len().saturating_sub(1) {
        if let Some(idx) = memchr::memchr(b'\n', &haystack[pos..]) {
            let abs_idx = pos + idx;
            if abs_idx + 1 < haystack.len() && haystack[abs_idx + 1] == b'#' {
                return Some(abs_idx + 1);
            }
            pos = abs_idx + 1;
        } else {
            break;
        }
    }
    None
}

/// Fast check if input contains any of the specified bytes.
#[inline]
pub fn contains_any_byte(haystack: &[u8], needles: &[u8]) -> bool {
    find_any_byte(haystack, needles).is_some()
}

/// Split input by newlines efficiently.
///
/// Returns an iterator over lines (without the newline characters).
pub fn split_lines(input: &[u8]) -> impl Iterator<Item = &[u8]> {
    SplitLines { remaining: input }
}

/// Iterator for splitting by newlines.
pub struct SplitLines<'a> {
    remaining: &'a [u8],
}

impl<'a> Iterator for SplitLines<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }

        match find_newline(self.remaining) {
            Some(idx) => {
                let line = &self.remaining[..idx];
                self.remaining = &self.remaining[idx + 1..];
                Some(line)
            }
            None => {
                let line = self.remaining;
                self.remaining = &[];
                Some(line)
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_byte_empty() {
        assert_eq!(find_byte(b"", b'x'), None);
    }

    #[test]
    fn test_find_byte_not_found() {
        assert_eq!(find_byte(b"hello world", b'x'), None);
    }

    #[test]
    fn test_find_byte_found_start() {
        assert_eq!(find_byte(b"hello", b'h'), Some(0));
    }

    #[test]
    fn test_find_byte_found_middle() {
        assert_eq!(find_byte(b"hello", b'l'), Some(2));
    }

    #[test]
    fn test_find_byte_found_end() {
        assert_eq!(find_byte(b"hello", b'o'), Some(4));
    }

    #[test]
    fn test_find_byte_large_input() {
        let input = vec![b'a'; 1000];
        assert_eq!(find_byte(&input, b'a'), Some(0));
        assert_eq!(find_byte(&input, b'b'), None);

        let mut input_with_needle = vec![b'a'; 500];
        input_with_needle.push(b'x');
        input_with_needle.extend(vec![b'a'; 499]);
        assert_eq!(find_byte(&input_with_needle, b'x'), Some(500));
    }

    #[test]
    fn test_find_any_byte_empty() {
        assert_eq!(find_any_byte(b"", b"xy"), None);
        assert_eq!(find_any_byte(b"hello", b""), None);
    }

    #[test]
    fn test_find_any_byte_found() {
        assert_eq!(find_any_byte(b"hello|world", b"|\n"), Some(5));
        assert_eq!(find_any_byte(b"hello\nworld", b"|\n"), Some(5));
    }

    #[test]
    fn test_count_byte_empty() {
        assert_eq!(count_byte(b"", b'x'), 0);
    }

    #[test]
    fn test_count_byte_none() {
        assert_eq!(count_byte(b"hello", b'x'), 0);
    }

    #[test]
    fn test_count_byte_some() {
        assert_eq!(count_byte(b"hello", b'l'), 2);
        assert_eq!(count_byte(b"hello", b'h'), 1);
    }

    #[test]
    fn test_count_byte_large_input() {
        let input = vec![b'a'; 1000];
        assert_eq!(count_byte(&input, b'a'), 1000);
        assert_eq!(count_byte(&input, b'b'), 0);
    }

    #[test]
    fn test_find_newline() {
        assert_eq!(find_newline(b"hello\nworld"), Some(5));
        assert_eq!(find_newline(b"hello"), None);
    }

    #[test]
    fn test_find_pipe() {
        assert_eq!(find_pipe(b"col1|col2"), Some(4));
        assert_eq!(find_pipe(b"no pipe here"), None);
    }

    #[test]
    fn test_find_delimiter() {
        assert_eq!(find_delimiter(b"hello|world"), Some(5));
        assert_eq!(find_delimiter(b"hello\nworld"), Some(5));
        assert_eq!(find_delimiter(b"hello"), None);
    }

    #[test]
    fn test_is_simd_available() {
        // This should return true on x86_64, x86, or aarch64
        let available = is_simd_available();
        // Just verify it doesn't panic
        println!("SIMD available: {}", available);
    }

    #[test]
    fn test_rfind_byte() {
        assert_eq!(rfind_byte(b"hello", b'l'), Some(3));
        assert_eq!(rfind_byte(b"hello", b'h'), Some(0));
        assert_eq!(rfind_byte(b"hello", b'x'), None);
        assert_eq!(rfind_byte(b"", b'x'), None);
    }

    #[test]
    fn test_trim_whitespace() {
        assert_eq!(trim_whitespace(b"  hello  "), b"hello");
        assert_eq!(trim_whitespace(b"\t\nhello\n\t"), b"hello");
        assert_eq!(trim_whitespace(b"hello"), b"hello");
        assert_eq!(trim_whitespace(b"   "), b"");
        assert_eq!(trim_whitespace(b""), b"");
    }

    #[test]
    fn test_is_whitespace_byte() {
        assert!(is_whitespace_byte(b' '));
        assert!(is_whitespace_byte(b'\t'));
        assert!(is_whitespace_byte(b'\n'));
        assert!(is_whitespace_byte(b'\r'));
        assert!(!is_whitespace_byte(b'a'));
        assert!(!is_whitespace_byte(b'0'));
    }

    #[test]
    fn test_count_leading_whitespace() {
        assert_eq!(count_leading_whitespace(b"  hello"), 2);
        assert_eq!(count_leading_whitespace(b"\t\nhello"), 2);
        assert_eq!(count_leading_whitespace(b"hello"), 0);
        assert_eq!(count_leading_whitespace(b"   "), 3);
        assert_eq!(count_leading_whitespace(b""), 0);
    }

    #[test]
    fn test_count_trailing_whitespace() {
        assert_eq!(count_trailing_whitespace(b"hello  "), 2);
        assert_eq!(count_trailing_whitespace(b"hello\t\n"), 2);
        assert_eq!(count_trailing_whitespace(b"hello"), 0);
        assert_eq!(count_trailing_whitespace(b"   "), 3);
        assert_eq!(count_trailing_whitespace(b""), 0);
    }

    #[test]
    fn test_count_lines() {
        assert_eq!(count_lines(b""), 0);
        assert_eq!(count_lines(b"hello"), 1);
        assert_eq!(count_lines(b"hello\nworld"), 2);
        assert_eq!(count_lines(b"a\nb\nc"), 3);
        assert_eq!(count_lines(b"a\nb\nc\n"), 4);
    }

    #[test]
    fn test_find_two_bytes() {
        assert_eq!(find_two_bytes(b"hello->world", b'-', b'>'), Some(5));
        assert_eq!(find_two_bytes(b"hello-world", b'-', b'>'), None);
        assert_eq!(find_two_bytes(b"->start", b'-', b'>'), Some(0));
        assert_eq!(find_two_bytes(b"", b'-', b'>'), None);
    }

    #[test]
    fn test_find_code_fence() {
        assert_eq!(find_code_fence(b"```rust\ncode\n```"), Some(0));
        assert_eq!(find_code_fence(b"text\n```rust"), Some(5));
        assert_eq!(find_code_fence(b"no fence here"), None);
        assert_eq!(find_code_fence(b"``not enough"), None);
    }

    #[test]
    fn test_find_header_marker() {
        assert_eq!(find_header_marker(b"# Hello"), Some(0));
        assert_eq!(find_header_marker(b"text\n# Hello"), Some(5));
        assert_eq!(find_header_marker(b"no header"), None);
    }

    #[test]
    fn test_contains_any_byte() {
        assert!(contains_any_byte(b"hello|world", b"|\n"));
        assert!(contains_any_byte(b"hello\nworld", b"|\n"));
        assert!(!contains_any_byte(b"hello world", b"|\n"));
    }

    #[test]
    fn test_split_lines() {
        let lines: Vec<_> = split_lines(b"a\nb\nc").collect();
        assert_eq!(lines, vec![b"a".as_slice(), b"b".as_slice(), b"c".as_slice()]);

        let lines: Vec<_> = split_lines(b"single").collect();
        assert_eq!(lines, vec![b"single".as_slice()]);

        let lines: Vec<_> = split_lines(b"").collect();
        assert!(lines.is_empty());

        // When input ends with newline, we get "a" then empty remaining
        let lines: Vec<_> = split_lines(b"a\n").collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], b"a".as_slice());

        // Multiple lines without trailing newline
        let lines: Vec<_> = split_lines(b"a\nb\nc").collect();
        assert_eq!(lines.len(), 3);
    }
}
