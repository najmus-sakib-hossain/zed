//! Named Constants for DX JavaScript Tooling
//!
//! This module provides named constants to replace magic numbers throughout the codebase.
//! All constants are documented with their purpose and usage.

// ============================================================================
// Hash-related constants
// ============================================================================

/// Length of hash prefix used for quick comparisons (8 bytes = 64 bits)
pub const HASH_PREFIX_LEN: usize = 8;

/// Full hash length for BLAKE3 (32 bytes = 256 bits)
pub const HASH_FULL_LEN: usize = 32;

/// Hex-encoded hash length (64 characters for 32 bytes)
pub const HASH_HEX_LEN: usize = 64;

// ============================================================================
// Buffer sizes
// ============================================================================

/// Default buffer size for file reading operations (8 KB)
pub const DEFAULT_READ_BUFFER_SIZE: usize = 8192;

/// Threshold for using memory-mapped I/O instead of regular read (4 KB)
/// Files smaller than this are read normally as the mmap overhead isn't worth it
pub const MMAP_THRESHOLD_BYTES: usize = 4096;

/// Maximum JSON nesting depth to prevent stack overflow
pub const MAX_JSON_DEPTH: usize = 512;

/// Maximum size for inline string storage (small string optimization)
pub const INLINE_STRING_MAX_LEN: usize = 23;

// ============================================================================
// Protocol magic numbers
// ============================================================================

/// Magic bytes for compiled code cache files
pub const CACHE_MAGIC: &[u8] = b"DXCACHE\x00";

/// Magic bytes for package format files
pub const PACKAGE_MAGIC: &[u8] = b"DXPKG\x00\x00\x00";

/// Magic bytes for lock file format
pub const LOCK_MAGIC: &[u8] = b"DXLOCK\x00\x00";

/// Cache file extension
pub const CACHE_FILE_EXTENSION: &str = "dxc";

// ============================================================================
// Timeout values (milliseconds)
// ============================================================================

/// Default timeout for network operations (30 seconds)
pub const DEFAULT_NETWORK_TIMEOUT_MS: u64 = 30_000;

/// Default timeout for compilation operations (60 seconds)
pub const DEFAULT_COMPILE_TIMEOUT_MS: u64 = 60_000;

/// Default timeout for file operations (10 seconds)
pub const DEFAULT_FILE_TIMEOUT_MS: u64 = 10_000;

// ============================================================================
// Property test iterations
// ============================================================================

/// Minimum iterations for standard property tests
pub const PROPTEST_MIN_ITERATIONS: u32 = 100;

/// Iterations for critical property tests (round-trip, serialization)
pub const PROPTEST_CRITICAL_ITERATIONS: u32 = 500;

/// Iterations for CI environment (more thorough testing)
pub const PROPTEST_CI_ITERATIONS: u32 = 1000;

// ============================================================================
// Runtime limits
// ============================================================================

/// Maximum call stack depth to prevent stack overflow
pub const MAX_CALL_STACK_DEPTH: usize = 1000;

/// Maximum number of local variables per function
pub const MAX_LOCALS_PER_FUNCTION: usize = 65536;

/// Maximum string length (1 GB)
pub const MAX_STRING_LENGTH: usize = 1024 * 1024 * 1024;

/// Maximum array length (2^32 - 1, per ECMAScript spec)
pub const MAX_ARRAY_LENGTH: u32 = 0xFFFF_FFFF;

// ============================================================================
// Cache configuration
// ============================================================================

/// Maximum number of modules to keep in memory cache
pub const MAX_CACHED_MODULES: usize = 1000;

/// Maximum total size of memory cache (256 MB)
pub const MAX_CACHE_SIZE_BYTES: usize = 256 * 1024 * 1024;

/// Cache entry TTL in seconds (1 hour)
pub const CACHE_TTL_SECONDS: u64 = 3600;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_constants_consistency() {
        // HASH_HEX_LEN should be 2x HASH_FULL_LEN (hex encoding)
        assert_eq!(HASH_HEX_LEN, HASH_FULL_LEN * 2);
        // HASH_PREFIX_LEN should be less than HASH_FULL_LEN - verified at compile time
        const _: () = assert!(HASH_PREFIX_LEN < HASH_FULL_LEN);
    }

    #[test]
    fn test_magic_bytes_length() {
        // All magic bytes should be 8 bytes for alignment
        assert_eq!(CACHE_MAGIC.len(), 8);
        assert_eq!(PACKAGE_MAGIC.len(), 8);
        assert_eq!(LOCK_MAGIC.len(), 8);
    }
}
