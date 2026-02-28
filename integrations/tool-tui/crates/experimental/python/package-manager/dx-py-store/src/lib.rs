//! dx-py-store: Memory-mapped package store for DX-Py package manager
//!
//! This crate provides a content-addressed package store with memory-mapped access
//! for zero-copy package operations and shared storage across projects.

mod error;
mod index;
mod mapped;
mod store;

pub use error::{StoreError, StoreResult};
pub use index::{PackageFileEntry, PackageIndex, PackageIndexHeader};
pub use mapped::MappedPackage;
pub use store::{InstallResult, PackageStore};

/// Magic number for DX-Py Package Store format
pub const DXPK_MAGIC: &[u8; 4] = b"DXPK";

/// Current store format version
pub const STORE_VERSION: u16 = 1;

/// Maximum package size (2GB)
pub const MAX_PACKAGE_SIZE: u64 = 2 * 1024 * 1024 * 1024;

/// Maximum file path length in package
pub const MAX_PATH_LENGTH: usize = 256;
