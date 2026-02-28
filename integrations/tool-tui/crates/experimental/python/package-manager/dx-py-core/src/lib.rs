//! dx-py-core: Core types and binary format definitions for DX-Py package manager
//!
//! This crate provides the foundational types, binary format headers, and shared
//! utilities used across all DX-Py crates.

pub mod atomic;
pub mod diagnostics;
pub mod error;
pub mod headers;
pub mod pep440;
pub mod version;
pub mod wheel;

/// Magic number for DX Python Package format
pub const DPP_MAGIC: &[u8; 4] = b"DPP\x01";

/// Magic number for DX Python Lock format
pub const DPL_MAGIC: &[u8; 4] = b"DPL\x01";

/// Magic number for DX Python Index format
pub const DPI_MAGIC: &[u8; 4] = b"DPI\x01";

/// Current protocol version
pub const PROTOCOL_VERSION: u16 = 1;

/// Maximum package size (2GB - supports large packages like PyTorch)
pub const MAX_PACKAGE_SIZE: u64 = 2 * 1024 * 1024 * 1024;

/// Maximum number of files in a package
pub const MAX_FILE_COUNT: u32 = 500_000;

/// Maximum lock file size (512MB)
pub const MAX_LOCK_SIZE: u64 = 512 * 1024 * 1024;

/// Maximum package name length
pub const MAX_NAME_LENGTH: usize = 64;

/// Maximum version string length
pub const MAX_VERSION_LENGTH: usize = 24;

pub use atomic::{atomic_write, atomic_write_str, AtomicDir, AtomicFile, CleanupGuard};
pub use diagnostics::{
    colors, ConflictNode, ConflictTree, Diagnostic, DiagnosticsBuilder, Severity, SourceLocation,
    SourceSnippet, Suggestion,
};
pub use error::{Error, Result};
pub use headers::{fnv1a_hash, DplEntry, DplHeader, DppHeader, DppMetadata, SourceType};
pub use pep440::{ParseError as Pep440ParseError, Pep440Version, PreRelease};
pub use version::{compare_versions, compare_versions_scalar, PackedVersion};
pub use wheel::{Arch, ManylinuxVersion, Os, PlatformEnvironment, PythonImpl, WheelTag};
