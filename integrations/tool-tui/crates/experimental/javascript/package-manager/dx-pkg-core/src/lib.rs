//! dx-pkg-core: Core types and utilities for the DX package manager
//!
//! This crate provides the fundamental data structures and utilities used across
//! all dx-package-manager crates. It includes:
//!
//! - **Version handling**: Semantic versioning and constraint parsing
//! - **Hash functions**: Fast content hashing with xxHash
//! - **Binary headers**: Memory-mapped binary format headers
//! - **Platform utilities**: Cross-platform path handling and symlinks
//!
//! # Binary Format
//!
//! All structures use `#[repr(C, packed)]` for binary compatibility and
//! zero-copy memory mapping. This enables extremely fast package loading
//! by memory-mapping package files directly.
//!
//! # Examples
//!
//! ```
//! use dx_pkg_core::version::{Version, VersionConstraint};
//!
//! // Parse and compare versions
//! let v1 = Version::parse("1.2.3").unwrap();
//! let v2 = Version::new(1, 5, 0);
//! assert!(v1 < v2);
//!
//! // Check version constraints
//! let constraint = VersionConstraint::parse("^1.0.0").unwrap();
//! assert!(constraint.matches(&v2));
//! ```

pub mod error;
pub mod hash;
pub mod headers;
pub mod platform;
pub mod version;

pub use error::{Error, Result};
pub use hash::{xxhash128, xxhash64, ContentHash};
pub use headers::{DxlHeader, DxpHeader, DxrpRequestHeader, DxrpResponseHeader};
pub use platform::{create_junction, create_symlink, is_safe_path, normalize_path, to_unix_path};
pub use version::{decode_version, encode_version, Version, VersionConstraint};

/// Magic numbers for binary format identification
pub const DXP_MAGIC: &[u8; 4] = b"DXP\0";
pub const DXL_MAGIC: &[u8; 4] = b"DXL\0";
pub const DXRP_REQUEST_MAGIC: &[u8; 4] = b"DXRP";
pub const DXRP_RESPONSE_MAGIC: &[u8; 4] = b"DXRR";

/// Protocol version
pub const PROTOCOL_VERSION: u16 = 1;

/// Maximum supported file sizes (security limits)
pub const MAX_PACKAGE_SIZE: u64 = 1024 * 1024 * 1024; // 1GB
pub const MAX_FILE_COUNT: u32 = 100_000;
pub const MAX_PATH_LENGTH: u16 = 1024;
pub const MAX_LOCK_SIZE: u64 = 512 * 1024 * 1024; // 512MB

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_numbers() {
        assert_eq!(DXP_MAGIC, b"DXP\0");
        assert_eq!(DXL_MAGIC, b"DXL\0");
        assert_eq!(DXRP_REQUEST_MAGIC, b"DXRP");
        assert_eq!(DXRP_RESPONSE_MAGIC, b"DXRR");
    }
}
