//! SIMD String Engine trait definition

/// SIMD String Engine interface
///
/// All implementations must produce identical results regardless of
/// SIMD availability (correctness property).
pub trait SimdStringEngine: Send + Sync {
    /// Find substring using SIMD (32 bytes at a time for AVX2)
    ///
    /// Returns the byte offset of the first occurrence of `needle` in `haystack`,
    /// or `None` if not found.
    fn find(&self, haystack: &str, needle: &str) -> Option<usize>;

    /// Count occurrences of substring using SIMD
    fn count(&self, haystack: &str, needle: &str) -> usize;

    /// String equality check using SIMD
    ///
    /// Compares 32 bytes at a time with early exit on mismatch.
    fn eq(&self, a: &str, b: &str) -> bool;

    /// Convert to lowercase using SIMD
    ///
    /// Handles ASCII range (A-Z) with SIMD, preserves other characters.
    fn to_lowercase(&self, s: &str) -> String;

    /// Convert to uppercase using SIMD
    ///
    /// Handles ASCII range (a-z) with SIMD, preserves other characters.
    fn to_uppercase(&self, s: &str) -> String;

    /// Split string using SIMD delimiter search
    fn split<'a>(&self, s: &'a str, delimiter: &str) -> Vec<&'a str>;

    /// Join strings with SIMD memory copy
    fn join(&self, parts: &[&str], separator: &str) -> String;

    /// Replace all occurrences using SIMD search
    fn replace(&self, s: &str, from: &str, to: &str) -> String;

    /// Get the name of this engine (for debugging)
    fn name(&self) -> &'static str;
}

/// Result type for SIMD operations
pub type SimdResult<T> = Result<T, SimdError>;

/// Errors that can occur during SIMD operations
#[derive(Debug, thiserror::Error)]
pub enum SimdError {
    #[error("Invalid UTF-8 sequence")]
    InvalidUtf8,

    #[error("SIMD operation not supported on this CPU")]
    NotSupported,

    #[error("Buffer too small")]
    BufferTooSmall,
}
